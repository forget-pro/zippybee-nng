# NNG Node.js 使用示例

这个项目提供了使用 NNG (nanomsg-next-generation) 进行消息传输的 Node.js 绑定，包含了智能重连、错误处理和中文提示功能。

## 功能特性

✅ **智能重连机制** - 连接断开时自动重试，最多 3 次  
✅ **中文错误提示** - 所有错误都有对应的中文描述  
✅ **主动关闭检测** - 正确区分主动关闭和意外断开  
✅ **callback 持续有效** - 重连后 Node.js callback 继续工作  
✅ **线程安全** - 使用 Arc 和 Mutex 保证多线程安全

## 快速开始

### 1. 简单示例

```bash
node simple-example.js
```

这个示例展示了最基本的用法：

```javascript
const { Socket } = require("./index.js");

const disposable = Socket.recv_message(
  "tcp://localhost:8888",
  { recv_timeout: 5000, send_timeout: 3000 },
  (error, buffer) => {
    if (error) {
      console.error("错误:", error.message);
      return;
    }

    if (buffer) {
      const message = buffer.toString();
      console.log("收到消息:", message);
    }
  }
);

// 关闭连接
await disposable.dispose();
```

### 2. 完整示例

```bash
node example.js
```

这个示例包含了：

- 完整的错误处理
- 消息分类处理
- 连接状态监控
- 优雅关闭
- 重连统计

### 3. 测试重连功能

```bash
# 1. 生成测试服务器
node test-server.js

# 2. 启动测试服务器
python mock_server.py

# 3. 在另一个终端运行客户端
node example.js
```

## 错误处理

### 支持的错误类型

| 错误类型     | 中文描述         | 处理建议               |
| ------------ | ---------------- | ---------------------- |
| 初始连接失败 | 连接失败: [原因] | 检查服务器地址和端口   |
| 连接已关闭   | 连接已关闭       | 自动重连               |
| 操作超时     | 操作超时         | 检查网络延迟           |
| 连接被拒绝   | 连接被拒绝       | 检查服务器状态         |
| 地址已被使用 | 地址已被使用     | 更换端口或关闭占用进程 |
| 目标不可达   | 目标不可达       | 检查网络连接           |
| 连接被重置   | 连接被重置       | 网络问题，会自动重连   |

### 错误处理示例

```javascript
function handleError(error) {
  console.error("发生错误:", error.message);

  if (error.message.includes("初始连接失败")) {
    console.log("建议: 检查服务器是否启动");
  } else if (error.message.includes("连接断开，重试失败")) {
    console.log("建议: 服务器可能已关闭，请检查服务器状态");
  }
}
```

## 重连机制

### 自动重连流程

1. **检测连接断开** - 监控 `nng::Error::Closed` 和其他网络错误
2. **显示中文错误信息** - 打印详细的错误原因
3. **延迟重连** - 等待递增的时间间隔（1 秒、2 秒、3 秒）
4. **创建新连接** - 重新创建 socket 并连接
5. **恢复消息接收** - callback 继续工作，无需重新绑定

### 重连配置

```javascript
const MAX_RETRIES = 3; // 最大重试次数
const RETRY_DELAY = 1000 * retry_count; // 重连延迟（递增）
```

## 最佳实践

### 1. 错误处理

```javascript
class NngClient {
  handleMessage(error, buffer) {
    if (error) {
      // 根据错误类型进行不同处理
      this.handleError(error);
      return;
    }

    // 处理正常消息
    this.processMessage(buffer);
  }

  handleError(error) {
    if (error.message.includes("重试失败")) {
      // 启动应用层重连
      this.scheduleReconnect();
    }
  }
}
```

### 2. 优雅关闭

```javascript
process.on("SIGINT", async () => {
  console.log("正在关闭连接...");
  if (disposable) {
    await disposable.dispose();
  }
  process.exit(0);
});
```

### 3. 消息处理

```javascript
function processMessage(buffer) {
  const message = buffer.toString();

  try {
    // 尝试解析 JSON
    const data = JSON.parse(message);
    handleJsonMessage(data);
  } catch (e) {
    // 处理普通文本消息
    handleTextMessage(message);
  }
}
```

## 配置选项

```javascript
const options = {
  recv_timeout: 5000, // 接收超时时间（毫秒）
  send_timeout: 3000, // 发送超时时间（毫秒）
};
```

## 故障排除

### 常见问题

1. **连接失败**

   - 检查服务器是否运行
   - 验证地址和端口是否正确
   - 检查防火墙设置

2. **频繁重连**

   - 检查网络稳定性
   - 增加超时时间
   - 检查服务器负载

3. **消息丢失**
   - 检查缓冲区大小
   - 确认消息处理速度
   - 监控内存使用

### 调试信息

启用详细日志：

```javascript
// 所有重连过程都会打印中文日志
// 例如：
// "连接意外关闭，原因: 远程服务器主动关闭连接"
// "尝试重新连接 (1/3)"
// "重连成功"
```

## 许可证

请参考项目根目录的 LICENSE 文件。
