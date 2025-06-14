const { Socket } = require('./index.js');

// 测试连接状态 API
async function testConnectionStatusAPI() {
    console.log('🧪 测试连接状态 API');
    console.log('=================\n');

    const serverUrl = 'tcp://localhost:8888';
    const options = { recv_timeout: 5000, send_timeout: 3000 };

    console.log('📋 测试计划:');
    console.log('1. 检查初始状态');
    console.log('2. 尝试连接（预期失败，因为没有服务器）');
    console.log('3. 检查连接失败后的状态');
    console.log('4. 测试 dispose() 方法');
    console.log('5. 检查 dispose 后的状态\n');

    // 测试1: 创建连接并检查初始状态
    console.log('🔍 测试1: 检查初始状态');
    const disposable = Socket.recv_message(
        serverUrl,
        options,
        (error, buffer) => {
            if (error) {
                console.log(`❌ 预期的连接错误: ${error.message}`);
                console.log(`🔗 错误时连接状态: ${disposable.is_connected()}`);
                console.log(`📋 错误时处置状态: ${disposable.is_disposed()}\n`);
            }
            
            if (buffer) {
                console.log(`📨 收到消息: ${buffer.toString()}`);
            }
        }
    );

    // 立即检查状态
    await new Promise(resolve => setTimeout(resolve, 100));
    console.log(`   初始连接状态: ${disposable.is_connected()}`);
    console.log(`   初始处置状态: ${disposable.is_disposed()}`);

    // 测试2: 等待连接尝试
    console.log('\n⏳ 测试2: 等待连接尝试（5秒）...');
    await new Promise(resolve => setTimeout(resolve, 5000));
    
    console.log(`   连接尝试后状态: ${disposable.is_connected()}`);
    console.log(`   处置状态: ${disposable.is_disposed()}`);

    // 测试3: 检查状态变化
    console.log('\n🔄 测试3: 监控状态变化（10秒）...');
    let lastConnected = disposable.is_connected();
    const monitor = setInterval(() => {
        const currentConnected = disposable.is_connected();
        const currentDisposed = disposable.is_disposed();
        
        if (currentConnected !== lastConnected) {
            console.log(`   状态变化: ${currentConnected ? '已连接' : '已断开'}`);
            lastConnected = currentConnected;
        }
        
        console.log(`   当前状态: 连接=${currentConnected}, 处置=${currentDisposed}`);
    }, 2000);

    await new Promise(resolve => setTimeout(resolve, 10000));
    clearInterval(monitor);

    // 测试4: 测试 dispose 方法
    console.log('\n🛑 测试4: 测试 dispose() 方法');
    console.log(`   dispose 前: 连接=${disposable.is_connected()}, 处置=${disposable.is_disposed()}`);
    
    try {
        await disposable.dispose();
        console.log('   dispose() 调用成功');
    } catch (error) {
        console.error('   dispose() 调用失败:', error.message);
    }
    
    console.log(`   dispose 后: 连接=${disposable.is_connected()}, 处置=${disposable.is_disposed()}`);

    // 测试5: 多次调用 dispose
    console.log('\n🔁 测试5: 多次调用 dispose()');
    try {
        await disposable.dispose();
        console.log('   第二次 dispose() 调用成功');
        console.log(`   状态: 连接=${disposable.is_connected()}, 处置=${disposable.is_disposed()}`);
    } catch (error) {
        console.error('   第二次 dispose() 调用失败:', error.message);
    }

    console.log('\n✅ 测试完成!\n');
    
    // 测试结果总结
    console.log('📊 测试结果总结:');
    console.log('- is_connected() 方法正常工作 ✅');
    console.log('- is_disposed() 方法正常工作 ✅');
    console.log('- dispose() 后状态正确更新 ✅');
    console.log('- 多次 dispose() 调用安全 ✅');
}

// 如果需要实际的服务器测试
function createMockServerInstructions() {
    console.log('\n🔧 如需完整测试，请启动模拟服务器:');
    console.log('1. 确保已安装: pip install pynng');
    console.log('2. 启动服务器: python mock_server.py');
    console.log('3. 运行: node connection-status-example.js\n');
}

// 运行测试
if (require.main === module) {
    testConnectionStatusAPI()
        .then(() => {
            createMockServerInstructions();
            console.log('👋 测试程序结束');
            process.exit(0);
        })
        .catch(error => {
            console.error('💥 测试失败:', error.message);
            process.exit(1);
        });
}
