use napi::{
  bindgen_prelude::*,
  threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode},
};
use nng::{options::Options,Socket, Protocol, Error as NngError};
use napi_derive::napi;
use core::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use serde_json::json;

#[napi]
pub struct SocketWrapper {
  socket: Option<Socket>,
  url: Option<String>,
  receiving: Arc<AtomicBool>,
  recv_thread_handle: Option<std::thread::JoinHandle<()>>,
  is_closed_by_user: Arc<AtomicBool>,
}

#[napi]
impl SocketWrapper {
  #[napi(constructor)]
  pub fn new() -> Self {
      SocketWrapper {
          socket: None,
          url: None,
          receiving: Arc::new(AtomicBool::new(false)),
          recv_thread_handle: None,
          is_closed_by_user: Arc::new(AtomicBool::new(false)),
      }
  }

  #[napi]
  pub fn connect(
      &mut self,
      protocol: ProtocolType,
      url: String,
      recv_timeout: u32,
      send_timeout: u32,
  ) -> Result<bool> {
      let socket = Socket::new(protocol.into()).map_err(|err| {
          napi::Error::new(napi::Status::GenericFailure, format!("Socket creation failed: {:?}", err))
      })?;

      let recv_timeout_duration = if recv_timeout == 0 {
          None
      } else {
          Some(Duration::from_millis(recv_timeout as u64))
      };

      let send_timeout_duration = if send_timeout == 0 {
          None
      } else {
          Some(Duration::from_millis(send_timeout as u64))
      };

      if let Some(timeout) = recv_timeout_duration {
          socket.set_opt::<nng::options::RecvTimeout>(Some(timeout))
              .map_err(|err| napi::Error::new(napi::Status::GenericFailure, format!("Failed to set receive timeout: {:?}", err)))?;
      }

      if let Some(timeout) = send_timeout_duration {
          socket.set_opt::<nng::options::SendTimeout>(Some(timeout))
              .map_err(|err| napi::Error::new(napi::Status::GenericFailure, format!("Failed to set send timeout: {:?}", err)))?;
      }

      socket.dial(&url).map_err(|err| {
          napi::Error::new(napi::Status::GenericFailure, format!("Connection failed: {:?}", err))
      })?;

      self.socket = Some(socket);
      self.url = Some(url);
      Ok(true)
  }

  #[napi]
  pub fn send(&self, message: Buffer) -> Result<Buffer> {
      if let Some(socket) = &self.socket {
          let msg = nng::Message::from(&message[..]);

          socket.send(msg).map_err(|(_, e)| {
              napi::Error::new(napi::Status::GenericFailure, format!("Send error: {:?}", e))
          })?;

          let response = socket.recv().map_err(|e| {
              match e {
                  NngError::TimedOut => napi::Error::new(napi::Status::GenericFailure, "Receive timeout".to_string()),
                  _ => napi::Error::new(napi::Status::GenericFailure, format!("Receive error: {:?}", e)),
              }
          })?;

          Ok(response.as_slice().into())
      } else {
          Err(napi::Error::new(napi::Status::GenericFailure, "Socket not connected".to_string()))
      }
  }

  #[napi]
  pub fn recv(&mut self, callback: ThreadsafeFunction<Buffer>) -> Result<()> {
      if self.recv_thread_handle.is_some() {
          return Err(napi::Error::new(
              napi::Status::GenericFailure,
              "Receive thread already running.".to_string(),
          ));
      }

      let socket = self.socket.clone();
      let receiving = self.receiving.clone();
      let is_closed_by_user = self.is_closed_by_user.clone();

      let handle = std::thread::spawn(move || {
          if let Some(socket) = socket {
              receiving.store(true, Ordering::SeqCst);
              loop {
                  if !receiving.load(Ordering::SeqCst) {
                      break;
                  }
                  match socket.recv() {
                      Ok(message) => {
                          let buffer = message.as_slice().into();
                          let _ = callback.call(Ok(buffer), ThreadsafeFunctionCallMode::NonBlocking);
                      }
                      Err(NngError::TimedOut) => {
                          eprintln!("Receive timed out.");
                      }
                      Err(nng::Error::Closed) => {
                          // If socket was closed, check if it was closed by the user
                          if is_closed_by_user.load(Ordering::SeqCst) {
                              return;
                          } else {
                              eprintln!("Socket was closed unexpectedly.");
                          }
                          break;
                      }
                      Err(e) => {
                          eprintln!("Error receiving message: {:?}", e);
                      }
                  }
              }
          }
      });

      self.recv_thread_handle = Some(handle);
      Ok(())
  }

  #[napi]
  pub fn close(&mut self) -> Result<String> {
      if let Some(socket) = self.socket.take() {
          self.receiving.store(false, Ordering::SeqCst); // 发送信号停止接收线程
          self.is_closed_by_user.store(true, Ordering::SeqCst); //设置主动关闭状态
          socket.close(); // 尝试关闭 socket，注意这里返回的是 ()，没有错误处理

          if let Some(url) = self.url.take() { // 获取关联的 URL
              let result = json!({
                  "code": 0,
                  "url": url
              });
              return Ok(result.to_string()); // 返回成功的 JSON 字符串
          } else {
              let result = json!({
                  "code": 1,
                  "message": "未找到关联的 URL"
              });
              return Ok(result.to_string()); // 返回错误信息的 JSON 字符串
          }
      } else {
          let result = json!({
              "code": 1,
              "message": "Socket 已关闭或未连接"
          });
          return Ok(result.to_string()); // 返回错误信息的 JSON 字符串
      }
  }

  #[napi]
  pub fn is_connect(&self) -> bool {
      self.socket.is_some()
  }
}

#[napi]
pub enum ProtocolType {
  Pair0,
  Pair1,
  Pub0,
  Sub0,
  Req0,
  Rep0,
  Surveyor0,
  Push0,
  Pull0,
  Bus0,
}

impl From<ProtocolType> for Protocol {
  fn from(protocol_type: ProtocolType) -> Self {
      match protocol_type {
          ProtocolType::Pair0 => Protocol::Pair0,
          ProtocolType::Pair1 => Protocol::Pair1,
          ProtocolType::Pub0 => Protocol::Pub0,
          ProtocolType::Sub0 => Protocol::Sub0,
          ProtocolType::Req0 => Protocol::Req0,
          ProtocolType::Rep0 => Protocol::Rep0,
          ProtocolType::Surveyor0 => Protocol::Surveyor0,
          ProtocolType::Push0 => Protocol::Push0,
          ProtocolType::Pull0 => Protocol::Pull0,
          ProtocolType::Bus0 => Protocol::Bus0,
      }
  }
}