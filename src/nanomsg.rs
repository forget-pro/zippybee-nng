use napi::{
  bindgen_prelude::*,
  threadsafe_function::{ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode},
};
use napi_derive::napi;
use lz4::block::{compress};
use lz4::block::CompressionMode;
use std::{
  sync::{Arc, Mutex, mpsc::{self, Sender}},
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
      let client = Arc::new(Mutex::new(Self::create_client(&options.unwrap_or_default())?));
      let (tx, rx) = mpsc::channel::<()>();
      let callback = Arc::new(callback); // 只克隆一次，减少多余的 `clone()`

      let client_clone = Arc::clone(&client);
      let callback_clone = Arc::clone(&callback);

      thread::spawn(move || {
          let client = client_clone.lock().unwrap();
          if let Err(e) = client.dial(&url) {
              callback_clone.call(
                  Err(Error::new(Status::GenericFailure, format!("Failed to connect: {}", e))),
                  ThreadsafeFunctionCallMode::NonBlocking,
              );
              return;
          }

          loop {
              if rx.try_recv().is_ok() {
                  client.close();
                  break;
              }

              match client.recv() {
                  Ok(msg) => {
                      callback.call(
                          Ok(msg.as_slice().into()),
                          ThreadsafeFunctionCallMode::NonBlocking,
                      );
                  }
                  Err(nng::Error::Closed) => {
                      let disconnect_msg = b"DISCONNECTED".to_vec();
                      callback.call(
                          Ok(disconnect_msg.into()),
                          ThreadsafeFunctionCallMode::NonBlocking,
                      );
                      break;
                  }
                  _ => {}
              }
          }
      });

      Ok(MessageRecvDisposable { closed: false, tx })
  }
}

#[napi]
pub struct MessageRecvDisposable {
  closed: bool,
  tx: Sender<()>,
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
      }
      Ok(())
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
