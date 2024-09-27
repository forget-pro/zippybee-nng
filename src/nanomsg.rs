use napi::{
    bindgen_prelude::*,
    threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode},
};
use nng::{options::Options,Socket, Protocol, Error as NngError};
use napi_derive::napi;
use std::{
    sync::{
        mpsc::{self, Sender},
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

/// Socket 配置选项
#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct SocketOptions {
    pub recv_timeout: Option<u32>, // 接收超时（毫秒），0 表示永不超时
    pub send_timeout: Option<u32>, // 发送超时（毫秒），0 表示永不超时
}

/// SocketWrapper 结构体，封装了 nng::Socket
#[napi]
pub struct SocketWrapper {
    socket: Option<Socket>,
    recv_thread_handle: Option<thread::JoinHandle<()>>,
    tx: Option<Sender<()>>,
    is_closed_by_user: Arc<AtomicBool>, // 标志是否是用户主动关闭的
}

#[napi]
impl SocketWrapper {
    /// 构造函数
    #[napi(constructor)]
    pub fn new() -> Self {
        SocketWrapper {
            socket: None,
            recv_thread_handle: None,
            tx: None,
            is_closed_by_user: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 连接到指定的 URL，并设置接收和发送超时
    #[napi]
    pub fn connect(
        &mut self,
        protocol: ProtocolType,
        url: String,
        recv_timeout: u32,
        send_timeout: u32,
    ) -> Result<()> {
        // 检查是否已经连接
        if self.socket.is_some() {
            return Err(napi::Error::new(
                napi::Status::GenericFailure,
                "Socket is already connected.".to_string(),
            ));
        }

        // 创建 Socket
        let socket = Socket::new(protocol.into())
            .map_err(|err| napi::Error::from(NngErrorWrapper(err)))?;

        // 设置接收超时
        if let Some(timeout) = if recv_timeout == 0 {
            None
        } else {
            Some(Duration::from_millis(recv_timeout as u64))
        } {
            socket
                .set_opt::<nng::options::RecvTimeout>(Some(timeout))
                .map_err(|err| {
                    napi::Error::from(NngErrorWrapper(err))
                })?;
        }

        // 设置发送超时
        if let Some(timeout) = if send_timeout == 0 {
            None
        } else {
            Some(Duration::from_millis(send_timeout as u64))
        } {
            socket
                .set_opt::<nng::options::SendTimeout>(Some(timeout))
                .map_err(|err| {
                    napi::Error::from(NngErrorWrapper(err))
                })?;
        }

        // 连接到指定的 URL
        socket
            .dial(&url)
            .map_err(|err| napi::Error::from(NngErrorWrapper(err)))?;

        self.socket = Some(socket);
        Ok(())
    }

    /// 发送消息并接收响应
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

    /// 开始接收消息
    #[napi]
    pub fn recv(&mut self, callback: ThreadsafeFunction<Buffer>) -> Result<MessageRecvDisposable> {
        if self.recv_thread_handle.is_some() {
            return Err(napi::Error::new(
                napi::Status::GenericFailure,
                "Receive thread already running.".to_string(),
            ));
        }

        if self.socket.is_none() {
            return Err(napi::Error::new(
                napi::Status::GenericFailure,
                "Socket not connected.".to_string(),
            ));
        }

        let (tx, rx) = mpsc::channel::<()>();
        self.tx = Some(tx.clone());
        let socket = self.socket.clone();
        let is_closed_by_user = Arc::clone(&self.is_closed_by_user);

        let handle = thread::spawn(move || {
            if let Some(socket) = socket {
                loop {
                    // 检查是否有关闭信号
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
                                return; // 安静退出
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
        Ok(MessageRecvDisposable{closed: false, tx})
    }

    /// 关闭 Socket 和接收线程
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
            socket.close(); // 关闭 socket
        }

        Ok(())
    }

    /// 检查 Socket 是否已连接
    #[napi]
    pub fn is_connected(&self) -> bool {
        self.socket.is_some()
    }
}

/// 错误封装
pub struct NngErrorWrapper(NngError);

impl From<NngErrorWrapper> for napi::Error {
    fn from(err: NngErrorWrapper) -> Self {
        napi::Error::new(napi::Status::GenericFailure, err.0.to_string())
    }
}

/// 协议类型枚举
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

/// 可用于关闭接收线程的结构体
#[napi]
pub struct MessageRecvDisposable {
    closed: bool,
    tx: Sender<()>,
}

#[napi]
impl MessageRecvDisposable {
    /// 关闭接收线程
    #[napi]
    pub fn close(&mut self) -> Result<()> {
        if !self.closed {
            self.tx.send(())
                .map_err(|e| napi::Error::from_reason(format!("Failed to stop msg channel: {}", e)))?;
            self.closed = true;
        }
        Ok(())
    }
}