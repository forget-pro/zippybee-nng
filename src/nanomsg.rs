use napi::{
  bindgen_prelude::*,
  threadsafe_function::{ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode},
};
use napi_derive::napi;
use lz4::block::{compress};
use lz4::block::CompressionMode;
use std::{
  sync::{Arc, Mutex, mpsc::{self, Sender}, atomic::{AtomicBool, Ordering}},
  thread,
  time::Duration,
};

use nng::{
  options::{Options, RecvTimeout, SendTimeout},
  Protocol,
};

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct SocketOptions {
  pub recv_timeout: Option<i32>,
  pub send_timeout: Option<i32>,
}

#[napi]
#[derive(Clone, Debug)]
pub struct Socket {
  client: Arc<Mutex<nng::Socket>>, // 线程安全管理 socket
  connected: bool,
  pub options: SocketOptions,
}

#[napi]
impl Socket {
  #[napi(constructor)]
  pub fn new(options: Option<SocketOptions>) -> Result<Self> {
      let opt = options.unwrap_or_default();
      Ok(Socket {
          client: Arc::new(Mutex::new(Self::create_client(&opt)?)),
          connected: false,
          options: opt,
      })
  }

  fn create_client(opt: &SocketOptions) -> Result<nng::Socket> {
      nng::Socket::new(Protocol::Pair1)
          .map(|client| {
              let _ = client.set_opt::<RecvTimeout>(Some(Duration::from_millis(
                  opt.recv_timeout.unwrap_or(5000) as u64,
              )));
              let _ = client.set_opt::<SendTimeout>(Some(Duration::from_millis(
                  opt.send_timeout.unwrap_or(5000) as u64,
              )));
              client
          })
          .map_err(|e| Error::from_reason(format!("Initiate socket failed: {}", e)))
  }

  #[napi]
  pub fn connect(&mut self, url: String) -> Result<()> {
      let client = self.client.lock().unwrap();
      let ret = client
          .dial(&url)
          .map_err(|e| Error::from_reason(format!("Connect {} failed: {}", url, e)));
      self.connected = ret.is_ok();
      ret
  }

  #[napi]
  pub fn send(&self, req: Buffer) -> Result<Buffer> {
      let client = self.client.lock().unwrap();
      let msg = nng::Message::from(&req[..]);
      client
          .send(msg)
          .map_err(|(_, e)| Error::from_reason(format!("Send rpc failed: {}", e)))?;
      client
          .recv()
          .map(|msg| msg.as_slice().into())
          .map_err(|e| Error::from_reason(format!("Recv rpc failed: {}", e)))
  }

  #[napi]
  pub fn close(&mut self) {
      let client = self.client.lock().unwrap();
      client.close();
      self.connected = false;
  }

  #[napi]
  pub fn connected(&self) -> bool {
      self.connected
  }

  #[napi(ts_args_type = "callback: (err: null | Error, bytes: Buffer) => void")]
  pub fn recv_message(
      url: String,
      options: Option<SocketOptions>,
      callback: ThreadsafeFunction<Buffer, ErrorStrategy::CalleeHandled>,
  ) -> Result<MessageRecvDisposable> {
      let options = options.unwrap_or_default();
      let client = Arc::new(Mutex::new(Self::create_client(&options)?));
      let (tx, rx) = mpsc::channel::<()>();
      let callback = Arc::new(callback);
      let url = Arc::new(url);
      let options = Arc::new(options);
      let is_connected = Arc::new(AtomicBool::new(false));

      let client_clone = Arc::clone(&client);
      let callback_clone = Arc::clone(&callback);
      let url_clone = Arc::clone(&url);
      let options_clone = Arc::clone(&options);
      let is_connected_clone = Arc::clone(&is_connected);

      thread::spawn(move || {
          let mut retry_count = 0;
          const MAX_RETRIES: u32 = 3;

          // 初始连接
          if let Err(e) = Self::establish_connection(&client_clone, &url_clone) {
              callback_clone.call(
                  Err(Error::new(Status::GenericFailure, format!("初始连接失败: {}", e))),
                  ThreadsafeFunctionCallMode::NonBlocking,
              );
              return;
          }

          // 标记为已连接
          is_connected_clone.store(true, Ordering::Relaxed);

          loop {
              // 检查是否主动关闭
              if rx.try_recv().is_ok() {
                  let client = client_clone.lock().unwrap();
                  client.close();
                  is_connected_clone.store(false, Ordering::Relaxed);
                  println!("连接已主动关闭");
                  break;
              }

              let client = client_clone.lock().unwrap();
              match client.recv() {
                  Ok(msg) => {
                      retry_count = 0; // 重置重试计数
                      // 确保连接状态为真（可能刚重连成功）
                      is_connected_clone.store(true, Ordering::Relaxed);
                      callback.call(
                          Ok(msg.as_slice().into()),
                          ThreadsafeFunctionCallMode::NonBlocking,
                      );
                  }
                  Err(nng::Error::Closed) => {
                      drop(client); // 释放锁
                      is_connected_clone.store(false, Ordering::Relaxed);
                      
                      println!("连接意外关闭，原因: 远程服务器主动关闭连接");
                      
                      if retry_count < MAX_RETRIES {
                          retry_count += 1;
                          println!("尝试重新连接 ({}/{})", retry_count, MAX_RETRIES);
                          
                          // 等待一段时间再重连
                          thread::sleep(Duration::from_millis(1000 * retry_count as u64));
                          
                          // 创建新的客户端并重新连接
                          match Self::create_client(&options_clone) {
                              Ok(new_client) => {
                                  *client_clone.lock().unwrap() = new_client;
                                  if let Err(e) = Self::establish_connection(&client_clone, &url_clone) {
                                      println!("重连失败: {}", e);
                                      continue;
                                  }
                                  println!("重连成功");
                                  is_connected_clone.store(true, Ordering::Relaxed);
                              }
                              Err(e) => {
                                  println!("创建新客户端失败: {}", e);
                                  continue;
                              }
                          }
                      } else {
                          println!("达到最大重试次数，放弃重连");
                          is_connected_clone.store(false, Ordering::Relaxed);
                          callback.call(
                              Err(Error::new(Status::GenericFailure, "连接断开，重试失败".to_string())),
                              ThreadsafeFunctionCallMode::NonBlocking,
                          );
                          break;
                      }
                  }
                  Err(nng::Error::TimedOut) => {
                      // 超时不算错误，继续循环
                      continue;
                  }
                  Err(e) => {
                      drop(client); // 释放锁
                      is_connected_clone.store(false, Ordering::Relaxed);
                      
                      let error_msg = Self::translate_error(&e);
                      println!("接收消息时发生错误: {}", error_msg);
                      
                      if retry_count < MAX_RETRIES {
                          retry_count += 1;
                          println!("尝试重新连接 ({}/{})", retry_count, MAX_RETRIES);
                          
                          // 等待一段时间再重连
                          thread::sleep(Duration::from_millis(1000 * retry_count as u64));
                          
                          // 创建新的客户端并重新连接
                          match Self::create_client(&options_clone) {
                              Ok(new_client) => {
                                  *client_clone.lock().unwrap() = new_client;
                                  if let Err(e) = Self::establish_connection(&client_clone, &url_clone) {
                                      println!("重连失败: {}", e);
                                      continue;
                                  }
                                  println!("重连成功");
                                  is_connected_clone.store(true, Ordering::Relaxed);
                              }
                              Err(e) => {
                                  println!("创建新客户端失败: {}", e);
                                  continue;
                              }
                          }
                      } else {
                          println!("达到最大重试次数，放弃重连");
                          is_connected_clone.store(false, Ordering::Relaxed);
                          callback.call(
                              Err(Error::new(Status::GenericFailure, format!("连接错误: {}", error_msg))),
                              ThreadsafeFunctionCallMode::NonBlocking,
                          );
                          break;
                      }
                  }
              }
          }
      });

      Ok(MessageRecvDisposable { 
          closed: false, 
          tx, 
          is_connected 
      })
  }

  // 建立连接的辅助方法
  fn establish_connection(client: &Arc<Mutex<nng::Socket>>, url: &str) -> Result<()> {
      let client = client.lock().unwrap();
      client.dial(url)
          .map_err(|e| Error::from_reason(format!("连接失败: {}", e)))
  }

  // 将nng错误翻译为中文
  fn translate_error(error: &nng::Error) -> String {
      match error {
          nng::Error::Closed => "连接已关闭".to_string(),
          nng::Error::TimedOut => "操作超时".to_string(),
          nng::Error::ConnectionRefused => "连接被拒绝".to_string(),
          nng::Error::InvalidInput => "无效输入".to_string(),
          nng::Error::AddressInUse => "地址已被使用".to_string(),
          nng::Error::NotSupported => "操作不支持".to_string(),
          nng::Error::AddressInvalid => "地址无效".to_string(),
          nng::Error::MessageTooLarge => "消息过大".to_string(),
          nng::Error::DestUnreachable => "目标不可达".to_string(),
          nng::Error::ConnectionAborted => "连接被中止".to_string(),
          nng::Error::ConnectionReset => "连接被重置".to_string(),
          _ => format!("未知错误: {:?}", error),
      }
  }
}

#[napi]
pub struct MessageRecvDisposable {
  closed: bool,
  tx: Sender<()>,
  is_connected: Arc<AtomicBool>,
}

#[napi]
impl MessageRecvDisposable {
  #[napi]
  pub fn dispose(&mut self) -> Result<()> {
      if !self.closed {
          self.tx.send(()).map_err(|e| {
              Error::from_reason(format!("Failed to stop msg channel: {}", e))
          })?;
          self.closed = true;
          self.is_connected.store(false, Ordering::Relaxed);
      }
      Ok(())
  }

  #[napi]
  pub fn is_connected(&self) -> bool {
      self.is_connected.load(Ordering::Relaxed)
  }

  #[napi]
  pub fn is_disposed(&self) -> bool {
      self.closed
  }
}

#[napi]
pub fn lz4_compress(input: Buffer) -> Result<Buffer> {
  match compress(&input, Some(CompressionMode::DEFAULT), false) {
      Ok(compressed) => Ok(Buffer::from(compressed)),
      Err(e) => Err(Error::new(
          Status::GenericFailure,
          format!("Compression failed: {}", e),
      )),
  }
}
