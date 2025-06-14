const { Socket } = require('./index.js');

// 简单使用示例 - 包含连接状态监控
function simpleExample() {
    console.log('🚀 简单示例 - NNG 消息接收 (带连接状态)');
    console.log('=========================================\n');

    const serverUrl = 'tcp://localhost:8888';
    const options = {
        recv_timeout: 5000,
        send_timeout: 3000
    };

    // 启动消息接收
    const disposable = Socket.recv_message(
        serverUrl,
        options,
        (error, buffer) => {
            if (error) {
                // 错误处理
                console.error('❌ 错误:', error.message);
                console.log(`🔗 当前连接状态: ${disposable.is_connected() ? '已连接' : '已断开'}`);
                
                if (error.message.includes('初始连接失败')) {
                    console.log('💡 提示: 请确保服务器在 tcp://localhost:8888 上运行');
                } else if (error.message.includes('重试失败')) {
                    console.log('💡 提示: 连接已断开且重连失败，请检查服务器状态');
                }
                return;
            }

            // 成功接收消息
            if (buffer) {
                const message = buffer.toString();
                console.log(`📨 收到消息: ${message}`);
                console.log(`📏 消息长度: ${buffer.length} 字节`);
                console.log(`🔗 连接状态: ${disposable.is_connected() ? '✅ 已连接' : '❌ 已断开'}`);
                console.log(`⏰ 时间: ${new Date().toLocaleString()}\n`);
            }
        }
    );

    // 每10秒显示一次连接状态
    const statusInterval = setInterval(() => {
        console.log(`📊 连接状态更新: ${disposable.is_connected() ? '✅ 已连接' : '❌ 已断开'} | 处置状态: ${disposable.is_disposed() ? '已处置' : '活跃中'}`);
    }, 10000);

    // 5分钟后自动关闭 (用于演示)
    setTimeout(async () => {
        console.log('⏰ 5分钟演示时间到，关闭连接...');
        clearInterval(statusInterval);
        
        console.log(`🔗 关闭前状态: 连接=${disposable.is_connected()}, 处置=${disposable.is_disposed()}`);
        
        try {
            await disposable.dispose();
            console.log('✅ 连接已关闭');
            console.log(`🔗 关闭后状态: 连接=${disposable.is_connected()}, 处置=${disposable.is_disposed()}`);
        } catch (err) {
            console.error('❌ 关闭时出错:', err.message);
        }
        process.exit(0);
    }, 5 * 60 * 1000);

    // 处理 Ctrl+C
    process.on('SIGINT', async () => {
        console.log('\n🛑 收到退出信号，关闭连接...');
        clearInterval(statusInterval);
        
        console.log(`🔗 当前状态: 连接=${disposable.is_connected()}, 处置=${disposable.is_disposed()}`);
        
        try {
            await disposable.dispose();
            console.log('✅ 连接已关闭');
            console.log(`🔗 最终状态: 连接=${disposable.is_connected()}, 处置=${disposable.is_disposed()}`);
        } catch (err) {
            console.error('❌ 关闭时出错:', err.message);
        }
        process.exit(0);
    });

    console.log('🎯 开始监听消息...');
    console.log('💡 按 Ctrl+C 退出');
    console.log('📊 每10秒会显示连接状态更新\n');

    // 显示初始状态
    setTimeout(() => {
        console.log(`🔍 初始连接状态: ${disposable.is_connected() ? '✅ 已连接' : '❌ 已断开'}\n`);
    }, 1000);
}

// 运行示例
simpleExample();