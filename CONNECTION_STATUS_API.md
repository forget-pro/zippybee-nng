# 连接状态 API 说明

## 新增功能

为 `MessageRecvDisposable` 对象添加了两个新方法来监控连接状态：

### 1. `is_connected()` - 检查连接状态

```javascript
const connected = disposable.is_connected();
console.log(`连接状态: ${connected ? '已连接' : '已断开'}`);
```

**返回值：**
- `true` - 连接正常，可以接收消息
- `false` - 连接断开（可能正在重连或已彻底断开）

**状态说明：**
- ✅ 初始连接成功后设置为 `true`
- ❌ 连接断开时设置为 `false`  
- ✅ 重连成功后重新设置为 `true`
- ❌ 调用 `dispose()` 后设置为 `false`

### 2. `is_disposed()` - 检查是否已处置

```javascript
const disposed = disposable.is_disposed();
console.log(`处置状态: ${disposed ? '已处置' : '活跃中'}`);
```

**返回值：**
- `true` - 已调用 `dispose()`，连接已关闭
- `false` - 连接仍然活跃（可能连接或断开，但未被手动关闭）

## 使用场景

### 1. 连接状态监控

```javascript
// 定期检查连接状态
setInterval(() => {
    const connected = disposable.is_connected();
    const disposed = disposable.is_disposed();
    
    if (!connected && !disposed) {
        console.log('🔄 连接断开，可能正在重连...');
    } else if (connected && !disposed) {
        console.log('✅ 连接正常');
    } else if (disposed) {
        console.log('🛑 连接已关闭');
    }
}, 5000);
```

### 2. 重连状态判断

```javascript
function checkReconnectionStatus() {
    const connected = disposable.is_connected();
    const disposed = disposable.is_disposed();
    
    if (!disposed) {
        if (connected) {
            return '✅ 已连接';
        } else {
            return '🔄 重连中';
        }
    } else {
        return '🛑 已关闭';
    }
}
```

### 3. 消息处理中的状态检查

```javascript
const disposable = Socket.recv_message(url, options, (error, buffer) => {
    if (error) {
        console.error('错误:', error.message);
        console.log(`当前状态: ${disposable.is_connected() ? '已连接' : '已断开'}`);
        return;
    }
    
    if (buffer) {
        console.log('收到消息:', buffer.toString());
        console.log(`连接状态: ${disposable.is_connected() ? '✅' : '❌'}`);
    }
});
```

### 4. 优雅关闭前的状态检查

```javascript
process.on('SIGINT', async () => {
    console.log('关闭前状态检查:');
    console.log(`  连接状态: ${disposable.is_connected()}`);
    console.log(`  处置状态: ${disposable.is_disposed()}`);
    
    if (!disposable.is_disposed()) {
        await disposable.dispose();
        console.log('连接已关闭');
    }
});
```

## 状态变化时序

```
启动 → 连接中 → 已连接 → 断开 → 重连中 → 已连接 → 主动关闭
 ↓        ↓        ↓       ↓        ↓         ↓         ↓
false   false    true   false    false     true     false
(未处置) (未处置)  (未处置) (未处置)  (未处置)   (未处置)   (已处置)
```

## 注意事项

1. **线程安全**：这些方法是线程安全的，可以在任何时候调用
2. **实时性**：状态更新是实时的，反映当前的真实连接状态
3. **重连期间**：重连期间 `is_connected()` 返回 `false`，重连成功后返回 `true`
4. **处置后**：调用 `dispose()` 后，`is_connected()` 永远返回 `false`

## 完整示例

请查看以下文件：
- `connection-status-example.js` - 完整的连接状态监控示例
- `simple-example.js` - 简单的使用优化示例

## TypeScript 定义

```typescript
interface MessageRecvDisposable {
    dispose(): Promise<void>;
    is_connected(): boolean;
    is_disposed(): boolean;
}
```
