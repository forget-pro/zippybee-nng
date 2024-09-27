use napi::{
  bindgen_prelude::*,
  threadsafe_function::{ ThreadsafeFunction, ThreadsafeFunctionCallMode},
};
use nng::{Socket, Protocol, Error as NngError};
use napi_derive::napi;
use std::{
  sync::{mpsc::{self, Sender}, Arc,  atomic::{AtomicBool, Ordering}},
  thread,
};

#[napi]
pub struct SocketWrapper {
  socket: Option<Socket>,
  recv_thread_handle: Option<thread::JoinHandle<()>>,
  tx: Option<Sender<()>>,
  is_closed_by_user: Arc<AtomicBool>,  // 标志是否是用户主动关闭的
}

#[napi]
impl SocketWrapper {
  #[napi(constructor)]
  pub fn new() -> Self {
      SocketWrapper { 
          socket: None,
          recv_thread_handle: None,
          tx: None,
          is_closed_by_user: Arc::new(AtomicBool::new(false)),
      }
  }

  #[napi]
  pub fn connect(&mut self, protocol: ProtocolType, url: String) -> Result<()> {
      let socket = Socket::new(protocol.into())
          .map_err(|err| napi::Error::from(NngErrorWrapper(err)))?;
      socket.dial(&url)
          .map_err(|err| napi::Error::from(NngErrorWrapper(err)))?;
      self.socket = Some(socket);
      Ok(())
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

      let (tx, rx) = mpsc::channel::<()>();
      self.tx = Some(tx);
      let socket = self.socket.clone();
      let is_closed_by_user = Arc::clone(&self.is_closed_by_user);

      let handle = thread::spawn(move || {
          if let Some(socket) = socket {
              loop {
                  if rx.try_recv().is_ok() {
                      // 如果接收到关闭信号，退出接收循环
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
                      Err(NngError::Closed) => {
                          // 如果是用户主动关闭的连接，不报告错误
                          if is_closed_by_user.load(Ordering::SeqCst) {
                              return;  // 安静退出
                          } else {
                              eprintln!("Socket was closed unexpectedly.");
                              return;
                          }
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
 pub fn close(&mut self) -> Result<()> {
    // 标记为用户主动关闭
    self.is_closed_by_user.store(true, Ordering::SeqCst);

    // 通知接收线程关闭
    if let Some(tx) = &self.tx {
        let _ = tx.send(());
    }

    // 等待接收线程结束
    if let Some(handle) = self.recv_thread_handle.take() {
        let _ = handle.join();
    }

    // 关闭 socket 连接
    if let Some(socket) = self.socket.take() {
        socket.close();  // 这里不再需要 map_err 处理
    }

    Ok(())
}

#[napi]
pub fn is_connect(&self) -> bool {
    self.socket.is_some()
}

}


pub struct NngErrorWrapper(NngError);

impl From<NngErrorWrapper> for napi::Error {
  fn from(err: NngErrorWrapper) -> Self {
      napi::Error::new(napi::Status::GenericFailure, err.0.to_string())
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