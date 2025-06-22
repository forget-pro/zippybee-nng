# @zippybee/nng

一个基于 Rust 的 nanomsg 协议绑定库，使用 NNG (Nanomsg Next Generation) 提供高性能的消息传递功能。

## 特性

- 支持 Pair1 协议的双向通信
- 异步消息接收，支持回调函数
- 连接状态监控和检测
- 可配置的超时设置
- LZ4 压缩支持
- TypeScript 类型定义

## 安装

```bash
npm install @zippybee/nng
```

## API 文档

### 类型定义

#### SocketOptions

Socket 配置选项对象。

```typescript
interface SocketOptions {
  recv_timeout?: number; // 接收超时时间（毫秒），默认 5000ms
  send_timeout?: number; // 发送超时时间（毫秒），默认 5000ms
  enable_heartbeat?: boolean; // 是否启用心跳（暂未实现）
  heartbeat_interval?: number; // 心跳间隔（毫秒，暂未实现）
}
```

### Socket 类

#### 构造函数

```typescript
new Socket(options?: SocketOptions)
```

创建一个新的 Socket 实例。

**参数：**

- `options` (可选): Socket 配置选项

**示例：**

```javascript
const { Socket } = require("@zippybee/nng");

// 使用默认配置
const socket = new Socket();

// 自定义配置
const socket = new Socket({
  recv_timeout: 10000, // 10秒接收超时
  send_timeout: 5000, // 5秒发送超时
});
```

#### connect(url)

连接到指定的地址。

```typescript
connect(url: string): void
```

**参数：**

- `url`: 要连接的地址，如 `"tcp://127.0.0.1:8888"`

**示例：**

```javascript
const socket = new Socket();
try {
  socket.connect("tcp://127.0.0.1:8888");
  console.log("连接成功");
} catch (error) {
  console.error("连接失败:", error.message);
}
```

#### send(data)

发送数据并等待响应（同步 RPC 模式）。

```typescript
send(data: Buffer): Buffer
```

**参数：**

- `data`: 要发送的数据（Buffer 类型）

**返回值：**

- 返回服务端的响应数据（Buffer 类型）

**示例：**

```javascript
const socket = new Socket();
socket.connect("tcp://127.0.0.1:8888");

const request = Buffer.from("Hello Server");
try {
  const response = socket.send(request);
  console.log("服务端响应:", response.toString());
} catch (error) {
  console.error("发送失败:", error.message);
}
```

#### close()

关闭连接。

```typescript
close(): void
```

**示例：**

```javascript
socket.close();
console.log("连接已关闭");
```

#### connected()

检查连接状态。

```typescript
connected(): boolean
```

**返回值：**

- 如果已连接返回 `true`，否则返回 `false`

**示例：**

```javascript
if (socket.connected()) {
  console.log("Socket 已连接");
} else {
  console.log("Socket 未连接");
}
```

#### Socket.testConnection(url, options?)

静态方法：测试指定地址的连接是否可用。

```typescript
static testConnection(url: string, options?: SocketOptions): boolean
```

**参数：**

- `url`: 要测试的地址
- `options` (可选): Socket 配置选项

**返回值：**

- 如果连接成功返回 `true`，否则返回 `false`

**示例：**

```javascript
// 测试连接
const canConnect = Socket.testConnection("tcp://127.0.0.1:8888");
if (canConnect) {
  console.log("服务器可连接");
} else {
  console.log("服务器不可用");
}

// 带配置的测试
const canConnect2 = Socket.testConnection("tcp://127.0.0.1:8888", {
  recv_timeout: 3000,
  send_timeout: 3000,
});
```

#### Socket.recvMessage(url, options?, callback)

静态方法：异步接收消息（推送模式）。

```typescript
static recvMessage(
  url: string,
  options: SocketOptions | null,
  callback: (err: null | Error, bytes: Buffer) => void
): MessageRecvDisposable
```

**参数：**

- `url`: 要连接的地址
- `options`: Socket 配置选项，传 `null` 使用默认配置
- `callback`: 回调函数
  - `err`: 错误对象，无错误时为 `null`
  - `bytes`: 接收到的数据（Buffer 类型）

**返回值：**

- 返回 `MessageRecvDisposable` 对象，用于管理连接

**示例：**

```javascript
// 基本用法
const disposable = Socket.recvMessage(
  "tcp://127.0.0.1:8888",
  null,
  (err, data) => {
    if (err) {
      console.error("接收错误:", err.message);
      return;
    }
    console.log("收到消息:", data.toString());
  }
);

// 带配置的用法
const disposable2 = Socket.recvMessage(
  "tcp://127.0.0.1:8888",
  {
    recv_timeout: 30000, // 30秒超时
    send_timeout: 10000, // 10秒超时
  },
  (err, data) => {
    if (err) {
      if (err.message.includes("Connection lost")) {
        console.log("连接丢失，需要重连");
        // 触发重连逻辑
      } else {
        console.error("其他错误:", err.message);
      }
      return;
    }
    console.log("收到消息:", data.toString());
  }
);

// 停止接收
setTimeout(() => {
  disposable.dispose();
}, 60000); // 1分钟后停止
```

### MessageRecvDisposable 类

用于管理异步消息接收的资源。

#### dispose()

停止接收消息并释放资源。

```typescript
dispose(): void
```

**示例：**

```javascript
const disposable = Socket.recvMessage(url, options, callback);

// 需要停止时调用
disposable.dispose();
```

#### isClosed()

检查是否已关闭。

```typescript
isClosed(): boolean
```

**返回值：**

- 如果已关闭返回 `true`，否则返回 `false`

**示例：**

```javascript
if (!disposable.isClosed()) {
  console.log("连接仍然活跃");
}
```

#### isConnectionAlive()

检查底层连接是否存活。

```typescript
isConnectionAlive(): boolean
```

**返回值：**

- 如果连接存活返回 `true`，否则返回 `false`

**示例：**

```javascript
if (!disposable.isConnectionAlive()) {
  console.log("连接已断开，需要重连");
  disposable.dispose();
  // 执行重连逻辑
}
```

### 工具函数

#### lz4Compress(data)

使用 LZ4 算法压缩数据。

```typescript
lz4Compress(data: Buffer): Buffer
```

**参数：**

- `data`: 要压缩的数据

**返回值：**

- 压缩后的数据

**示例：**

```javascript
const { lz4Compress } = require("@zippybee/nng");

const data = Buffer.from("这是要压缩的数据");
const compressed = lz4Compress(data);
console.log("压缩前大小:", data.length);
console.log("压缩后大小:", compressed.length);
```

## 完整示例

### 1. 简单的请求-响应模式

```javascript
const { Socket } = require("@zippybee/nng");

async function requestResponseExample() {
  const socket = new Socket({
    recv_timeout: 5000,
    send_timeout: 5000,
  });

  try {
    // 连接到服务器
    socket.connect("tcp://127.0.0.1:8888");

    // 发送请求
    const request = Buffer.from(
      JSON.stringify({
        action: "getData",
        params: { id: 123 },
      })
    );

    const response = socket.send(request);
    const result = JSON.parse(response.toString());

    console.log("服务器响应:", result);
  } catch (error) {
    console.error("操作失败:", error.message);
  } finally {
    socket.close();
  }
}

requestResponseExample();
```

### 2. 异步消息接收模式

```javascript
const { Socket } = require("@zippybee/nng");

class MessageReceiver {
  constructor(url, options = {}) {
    this.url = url;
    this.options = {
      recv_timeout: 30000, // 30秒接收超时
      send_timeout: 10000, // 10秒发送超时
      ...options,
    };
    this.disposable = null;
    this.isActive = false;
  }

  // 开始接收消息
  start(messageHandler, errorHandler) {
    if (this.isActive) {
      console.warn("已经在接收消息中");
      return;
    }

    try {
      console.log(`开始从 ${this.url} 接收消息...`);

      this.disposable = Socket.recvMessage(
        this.url,
        this.options,
        (err, data) => {
          if (err) {
            console.error("接收错误:", err.message);
            if (errorHandler) {
              errorHandler(err);
            }
            return;
          }

          if (messageHandler) {
            messageHandler(data);
          }
        }
      );

      this.isActive = true;
      console.log("开始接收消息成功");
    } catch (error) {
      console.error("启动接收失败:", error);
      if (errorHandler) {
        errorHandler(error);
      }
    }
  }

  // 停止接收消息
  stop() {
    if (!this.isActive) {
      return;
    }

    console.log("停止接收消息...");
    this.isActive = false;

    if (this.disposable) {
      try {
        this.disposable.dispose();
      } catch (error) {
        console.error("停止接收时出错:", error);
      }
      this.disposable = null;
    }
  }

  // 检查连接状态
  isConnectionAlive() {
    return this.disposable && this.disposable.isConnectionAlive();
  }

  // 测试连接
  async testConnection() {
    try {
      return Socket.testConnection(this.url, this.options);
    } catch (error) {
      console.error("测试连接失败:", error);
      return false;
    }
  }
}

// 使用示例
async function main() {
  const receiver = new MessageReceiver("tcp://127.0.0.1:8888");

  // 先测试连接
  const canConnect = await receiver.testConnection();
  if (!canConnect) {
    console.error("无法连接到服务器");
    return;
  }

  // 开始接收消息
  receiver.start(
    (data) => {
      // 处理收到的消息
      const message = data.toString();
      console.log("收到消息:", message);

      try {
        const json = JSON.parse(message);
        console.log("解析后的数据:", json);
      } catch (e) {
        console.log("非JSON格式的消息");
      }
    },
    (error) => {
      // 处理错误
      console.error("消息接收错误:", error.message);

      if (error.message.includes("Connection lost")) {
        console.log("连接丢失，尝试重连...");
        receiver.stop();

        // 3秒后重连
        setTimeout(async () => {
          const canReconnect = await receiver.testConnection();
          if (canReconnect) {
            receiver.start(messageHandler, errorHandler);
          } else {
            console.log("重连失败，服务器仍不可用");
          }
        }, 3000);
      }
    }
  );

  // 定期检查连接状态
  const healthCheck = setInterval(() => {
    if (receiver.isActive && !receiver.isConnectionAlive()) {
      console.warn("检测到连接断开");
      receiver.stop();
      // 可以在这里触发重连逻辑
    }
  }, 10000); // 每10秒检查一次

  // 优雅关闭
  process.on("SIGINT", () => {
    console.log("正在关闭...");
    clearInterval(healthCheck);
    receiver.stop();
    process.exit(0);
  });
}

main().catch(console.error);
```

### 3. 自动重连的消息接收器

```javascript
const { Socket } = require("@zippybee/nng");

class AutoReconnectReceiver {
  constructor(url, options = {}) {
    this.url = url;
    this.options = {
      recv_timeout: 30000,
      send_timeout: 10000,
      ...options,
    };
    this.disposable = null;
    this.isRunning = false;
    this.reconnectDelay = 3000; // 重连延迟
    this.maxReconnectAttempts = 5;
    this.reconnectAttempts = 0;
  }

  async start(messageHandler, errorHandler) {
    this.messageHandler = messageHandler;
    this.errorHandler = errorHandler;
    this.isRunning = true;
    this.reconnectAttempts = 0;

    await this._connect();
  }

  async _connect() {
    if (!this.isRunning) return;

    try {
      // 测试连接
      const canConnect = Socket.testConnection(this.url, this.options);
      if (!canConnect) {
        throw new Error("Cannot connect to server");
      }

      console.log("连接到服务器...");

      this.disposable = Socket.recvMessage(
        this.url,
        this.options,
        (err, data) => {
          if (err) {
            console.error("接收错误:", err.message);
            if (this.errorHandler) {
              this.errorHandler(err);
            }

            if (err.message.includes("Connection lost")) {
              this._handleConnectionLost();
            }
            return;
          }

          // 成功接收消息，重置重连计数
          this.reconnectAttempts = 0;

          if (this.messageHandler) {
            this.messageHandler(data);
          }
        }
      );

      console.log("连接建立成功");
      this.reconnectAttempts = 0;
    } catch (error) {
      console.error("连接失败:", error.message);
      this._handleConnectionLost();
    }
  }

  _handleConnectionLost() {
    if (!this.isRunning) return;

    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error("达到最大重连次数，停止重连");
      this.stop();
      if (this.errorHandler) {
        this.errorHandler(new Error("Max reconnection attempts reached"));
      }
      return;
    }

    this.reconnectAttempts++;
    console.log(
      `连接丢失，${this.reconnectDelay}ms 后进行第 ${this.reconnectAttempts} 次重连...`
    );

    if (this.disposable) {
      try {
        this.disposable.dispose();
      } catch (e) {
        console.error("关闭连接时出错:", e);
      }
      this.disposable = null;
    }

    setTimeout(() => this._connect(), this.reconnectDelay);
  }

  stop() {
    console.log("停止接收器...");
    this.isRunning = false;

    if (this.disposable) {
      try {
        this.disposable.dispose();
      } catch (error) {
        console.error("停止时出错:", error);
      }
      this.disposable = null;
    }
  }

  isActive() {
    return (
      this.isRunning &&
      this.disposable &&
      !this.disposable.isClosed() &&
      this.disposable.isConnectionAlive()
    );
  }
}

// 使用自动重连接收器
const autoReceiver = new AutoReconnectReceiver("tcp://127.0.0.1:8888");

autoReceiver.start(
  (data) => {
    console.log("收到消息:", data.toString());
  },
  (error) => {
    console.error("错误:", error.message);
  }
);

// 10分钟后停止
setTimeout(() => {
  autoReceiver.stop();
}, 10 * 60 * 1000);
```

## 错误处理

### 常见错误类型

1. **连接错误**

   - `"Failed to connect: ..."` - 连接失败
   - `"Connect xxx failed: ..."` - 特定地址连接失败

2. **发送/接收错误**

   - `"Send rpc failed: ..."` - 发送 RPC 消息失败
   - `"Recv rpc failed: ..."` - 接收 RPC 响应失败
   - `"Connection lost: ..."` - 连接丢失

3. **超时错误**
   - 当设置了超时时间且操作超时会触发相应错误

### 错误处理最佳实践

```javascript
// 1. 连接前先测试
const canConnect = Socket.testConnection(url);
if (!canConnect) {
  console.error("服务器不可用");
  return;
}

// 2. 使用 try-catch 处理同步操作
try {
  const response = socket.send(data);
  // 处理响应
} catch (error) {
  if (error.message.includes("Send rpc failed")) {
    console.log("发送失败，可能是网络问题");
  } else if (error.message.includes("Recv rpc failed")) {
    console.log("接收响应失败，可能是超时");
  }
}

// 3. 异步接收中正确处理错误
Socket.recvMessage(url, options, (err, data) => {
  if (err) {
    if (err.message.includes("Connection lost")) {
      // 连接丢失，触发重连
    } else if (err.message.includes("TimedOut")) {
      // 超时，这通常是正常的
    } else {
      // 其他错误
      console.error("未知错误:", err.message);
    }
    return;
  }

  // 处理正常数据
});
```

## 性能优化建议

1. **合理设置超时时间**

   - 接收超时建议设置为 30-60 秒，避免频繁超时
   - 发送超时建议设置为 5-10 秒

2. **连接复用**

   - 尽量复用 Socket 连接，避免频繁创建和销毁

3. **错误重试**

   - 实现指数退避的重连策略
   - 设置最大重连次数防止无限重试

4. **资源清理**
   - 及时调用 `dispose()` 方法清理资源
   - 在程序退出时确保所有连接都被正确关闭

## 许可证

MIT License
