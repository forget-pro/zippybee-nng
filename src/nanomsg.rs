use lz4::block::compress;
use lz4::block::CompressionMode;
use napi::{
  bindgen_prelude::*,
  threadsafe_function::{ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode},
};
use napi_derive::napi;
use std::{
  sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Sender},
    Arc,
  },
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
  client: nng::Socket,
  connected: bool,
  pub options: SocketOptions,
}

#[napi]
impl Socket {
  #[napi(constructor)]
  pub fn new(options: Option<SocketOptions>) -> Result<Self> {
    let opt = options.unwrap_or_default();
    Ok(Socket {
      client: Self::create_client(&opt)?,
      connected: false,
      options: opt,
    })
  }

  pub fn create_client(opt: &SocketOptions) -> Result<nng::Socket> {
    nng::Socket::new(Protocol::Pair1)
      .map(|client| {
        let _ = client.set_opt::<RecvTimeout>(Some(Duration::from_millis(
          opt
            .recv_timeout
            .and_then(|i| i.try_into().ok())
            .unwrap_or(5000), // 5秒接收超时，提高响应性
        )));
        let _ = client.set_opt::<SendTimeout>(Some(Duration::from_millis(
          opt
            .send_timeout
            .and_then(|i| i.try_into().ok())
            .unwrap_or(5000), // 5秒发送超时
        )));
        client
      })
      .map_err(|e| Error::from_reason(format!("Initiate socket failed: {}", e)))
  }

  #[napi]
  pub fn connect(&mut self, url: String) -> Result<()> {
    let ret = self
      .client
      .dial(&url)
      .map_err(|e| Error::from_reason(format!("Connect {} failed: {}", url, e)));
    self.connected = ret.is_ok();
    ret
  }

  #[napi]
  pub fn send(&self, req: Buffer) -> Result<Buffer> {
    let msg = nng::Message::from(&req[..]);
    self
      .client
      .send(msg)
      .map_err(|(_, e)| Error::from_reason(format!("Send rpc failed: {}", e)))?;
    self
      .client
      .recv()
      .map(|msg| msg.as_slice().into())
      .map_err(|e| Error::from_reason(format!("Recv rpc failed: {}", e)))
  }

  #[napi]
  pub fn close(&mut self) {
    self.client.close();
    self.connected = false;
  }

  #[napi]
  pub fn connected(&self) -> bool {
    self.connected
  }

  // 静态方法：测试连接是否可用
  #[napi]
  pub fn test_connection(url: String, options: Option<SocketOptions>) -> Result<bool> {
    let client = Self::create_client(&options.unwrap_or_default())?;
    match client.dial(&url) {
      Ok(_) => {
        client.close();
        Ok(true)
      }
      Err(_) => Ok(false),
    }
  }

  #[napi(ts_args_type = "callback: (err: null | Error, bytes: Buffer) => void")]
  pub fn recv_message(
    url: String,
    callback: ThreadsafeFunction<Buffer, ErrorStrategy::CalleeHandled>,
    options: Option<SocketOptions>,
  ) -> Result<MessageRecvDisposable> {
    let client = Self::create_client(&options.unwrap_or_default())?;
    client
      .dial(&url)
      .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to connect: {}", e)))?;
    let (tx, rx) = mpsc::channel::<()>();
    let connection_alive = Arc::new(AtomicBool::new(true));
    let connection_alive_clone = connection_alive.clone();

    thread::spawn(move || {
      loop {
        // 检查是否需要停止
        if rx.try_recv().is_ok() {
          connection_alive_clone.store(false, Ordering::Relaxed);
          client.close();
          break;
        }

        match client.recv() {
          Ok(msg) => {
            let call_result = callback.clone().call(
              Ok(msg.as_slice().into()),
              ThreadsafeFunctionCallMode::NonBlocking,
            );

            // 如果 Node.js 正在关闭，立即退出
            if matches!(call_result, napi::Status::Closing) {
              connection_alive_clone.store(false, Ordering::Relaxed);
              client.close();
              return;
            }
          }
          Err(e) => match e {
            nng::Error::Closed => {
              connection_alive_clone.store(false, Ordering::Relaxed);
              return;
            }
            nng::Error::TimedOut => continue, // 超时是正常的，继续循环
            _ => {
              // 其他错误，通知客户端并退出
              connection_alive_clone.store(false, Ordering::Relaxed);
              let _ = callback.clone().call(
                Err(Error::new(
                  Status::GenericFailure,
                  format!("Connection lost: {}", e),
                )),
                ThreadsafeFunctionCallMode::NonBlocking,
              );
              return;
            }
          },
        }
      }
    });

    Ok(MessageRecvDisposable {
      closed: false,
      tx,
      connection_alive,
    })
  }
}

#[napi]
pub struct MessageRecvDisposable {
  closed: bool,
  tx: Sender<()>,
  connection_alive: Arc<AtomicBool>,
}

#[napi]
impl MessageRecvDisposable {
  #[napi]
  pub fn dispose(&mut self) -> Result<()> {
    if !self.closed {
      self.connection_alive.store(false, Ordering::Relaxed);
      self
        .tx
        .send(())
        .map_err(|e| Error::from_reason(format!("Failed to stop msg channel: {}", e)))?;
      self.closed = true;
    }
    Ok(())
  }

  #[napi]
  pub fn is_closed(&self) -> bool {
    self.closed
  }

  // 检查连接是否还存活
  #[napi]
  pub fn is_connection_alive(&self) -> bool {
    self.connection_alive.load(Ordering::Relaxed)
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
