const { Socket } = require('./index.js');

// 连接状态监控示例
function connectionStatusExample() {
    console.log('🔗 连接状态监控示例');
    console.log('==================\n');

    const serverUrl = 'tcp://localhost:8888';
    const options = {
        recv_timeout: 5000,
        send_timeout: 3000
    };

    let messageCount = 0;

    // 启动消息接收
    const disposable = Socket.recv_message(
        serverUrl,
        options,
        (error, buffer) => {
            if (error) {
                console.error('❌ 错误:', error.message);
                console.log(`🔗 连接状态: ${disposable.is_connected() ? '已连接' : '已断开'}`);
                return;
            }

            if (buffer) {
                messageCount++;
                const message = buffer.toString();
                console.log(`📨 消息 #${messageCount}: ${message}`);
                console.log(`🔗 连接状态: ${disposable.is_connected() ? '已连接' : '已断开'}\n`);
            }
        }
    );

    // 定期检查连接状态
    const statusInterval = setInterval(() => {
        const connected = disposable.is_connected();
        const disposed = disposable.is_disposed();
        
        console.log('📊 状态检查:');
        console.log(`   - 连接状态: ${connected ? '✅ 已连接' : '❌ 已断开'}`);
        console.log(`   - 处置状态: ${disposed ? '✅ 已处置' : '⚪ 活跃中'}`);
        console.log(`   - 接收消息数: ${messageCount}`);
        console.log(`   - 检查时间: ${new Date().toLocaleTimeString()}\n`);

        // 如果连接已断开且未被处置，说明可能在重连中
        if (!connected && !disposed) {
            console.log('🔄 检测到连接断开，可能正在重连中...\n');
        }
    }, 5000); // 每5秒检查一次

    // 连接状态变化监控
    let lastConnectedState = false;
    const connectionMonitor = setInterval(() => {
        const currentState = disposable.is_connected();
        
        if (currentState !== lastConnectedState) {
            if (currentState) {
                console.log('🟢 连接状态变化: 已连接');
            } else {
                console.log('🔴 连接状态变化: 已断开');
            }
            lastConnectedState = currentState;
        }
    }, 1000); // 每秒检查状态变化

    // 处理程序退出
    process.on('SIGINT', async () => {
        console.log('\n🛑 正在关闭连接...');
        
        clearInterval(statusInterval);
        clearInterval(connectionMonitor);
        
        console.log(`🔗 关闭前连接状态: ${disposable.is_connected() ? '已连接' : '已断开'}`);
        
        try {
            await disposable.dispose();
            console.log('✅ 连接已关闭');
            console.log(`🔗 关闭后连接状态: ${disposable.is_connected() ? '已连接' : '已断开'}`);
            console.log(`📋 处置状态: ${disposable.is_disposed() ? '已处置' : '未处置'}`);
        } catch (err) {
            console.error('❌ 关闭时出错:', err.message);
        }
        
        process.exit(0);
    });

    console.log('🎯 开始监控连接状态...');
    console.log('💡 按 Ctrl+C 退出\n');

    // 启动后立即检查初始状态
    setTimeout(() => {
        console.log('🔍 初始状态检查:');
        console.log(`   - 连接状态: ${disposable.is_connected() ? '✅ 已连接' : '❌ 已断开'}`);
        console.log(`   - 处置状态: ${disposable.is_disposed() ? '✅ 已处置' : '⚪ 活跃中'}\n`);
    }, 1000);
}

// 运行示例
connectionStatusExample();
