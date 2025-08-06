/**
 * 测试重构后的架构
 * 验证完全基于上下文的新架构是否正常工作
 */

import {
  TaskContext,
  AgentContext,
  MemoryManager,
  TaskSnapshotManager,
  BaseAgent,
  AgentResult,
  AgentExecutionStatus,
} from './index'

import { WorkflowDefinition, WorkflowAgent } from './types/workflow'

/**
 * 测试Agent - 继承BaseAgent
 */
class TestAgent extends BaseAgent {
  async executeWithContext(agentContext: AgentContext): Promise<AgentResult> {
    const startTime = Date.now()

    // 模拟一些工作
    await new Promise(resolve => setTimeout(resolve, 100))

    // 设置一些变量
    agentContext.setVariable('testResult', 'Hello from TestAgent!')

    return this.createSuccessResult('Test execution completed successfully', Date.now() - startTime, {
      testMetadata: 'test value',
    })
  }
}

/**
 * 测试重构后的架构
 */
const testRefactoredArchitecture = async () => {
  console.log('🚀 开始测试重构后的架构...')

  try {
    // 1. 创建测试工作流
    const workflow: WorkflowDefinition = {
      taskId: 'test-task-001',
      name: '测试工作流',
      agents: [
        {
          id: 'test-agent-1',
          name: '测试Agent',
          task: '执行测试任务',
          type: 'test',
        } as WorkflowAgent,
      ],
    }

    // 2. 创建TaskContext
    console.log('📝 创建TaskContext...')
    const taskContext = new TaskContext(workflow.taskId, { maxRetries: 3 }, workflow, { initialParam: 'test value' })

    // 3. 创建AgentContext
    console.log('🤖 创建AgentContext...')
    const agentContext = new AgentContext(workflow.agents[0], taskContext)

    // 4. 测试记忆管理
    console.log('🧠 测试记忆管理...')
    await agentContext.addMessage('user', '这是一条测试消息')
    await agentContext.addMessage('assistant', '我收到了您的消息')

    const memory = taskContext.memory.getMemory()
    console.log(`✅ 记忆中有 ${memory.chatHistory.length} 条消息`)

    // 5. 测试Agent执行
    console.log('⚡ 测试Agent执行...')
    const testAgent = new TestAgent()
    const result = await testAgent.executeWithRetry(agentContext)

    console.log(`✅ Agent执行结果: ${result.success ? '成功' : '失败'}`)
    console.log(`📊 执行时间: ${result.executionTime}ms`)
    console.log(`📄 结果数据: ${result.data}`)

    // 6. 测试变量管理
    console.log('🔧 测试变量管理...')
    agentContext.setVariable('agentVar', 'agent level value')
    taskContext.setVariable('taskVar', 'task level value')

    console.log(`✅ Agent变量: ${agentContext.getVariable('agentVar')}`)
    console.log(`✅ Task变量: ${agentContext.getVariable('taskVar')}`)
    console.log(`✅ 初始参数: ${agentContext.getVariable('initialParam')}`)

    // 7. 测试快照功能
    console.log('📸 测试快照功能...')
    const snapshotManager = new TaskSnapshotManager()
    const snapshot = await snapshotManager.createSnapshot(taskContext, [agentContext], 'manual')

    console.log(`✅ 快照创建成功: ${snapshot.taskId}`)
    console.log(`📊 快照包含 ${snapshot.metadata.messageCount} 条消息`)
    console.log(`🔢 快照Token数: ${snapshot.metadata.totalTokens}`)

    // 8. 测试Agent状态管理
    console.log('📈 测试Agent状态管理...')
    const stats = agentContext.getExecutionStats()
    console.log(`✅ Agent状态: ${stats.status}`)
    console.log(`✅ 执行次数: ${stats.totalExecutions}`)
    console.log(`✅ 变量数量: ${stats.variableCount}`)

    // 9. 测试记忆压缩
    console.log('🗜️ 测试记忆压缩...')

    // 添加更多消息以触发压缩
    for (let i = 0; i < 20; i++) {
      await agentContext.addMessage('user', `测试消息 ${i}`)
    }

    const memoryBefore = taskContext.memory.getMemory()
    console.log(`📊 压缩前消息数: ${memoryBefore.chatHistory.length}`)

    // 手动触发压缩
    await taskContext.memory.compressMessages()

    const memoryAfter = taskContext.memory.getMemory()
    console.log(`📊 压缩后消息数: ${memoryAfter.chatHistory.length}`)

    console.log('\n🎉 所有测试通过！重构后的架构工作正常！')

    return {
      success: true,
      taskContext,
      agentContext,
      snapshot,
      stats,
    }
  } catch (error) {
    console.error('❌ 测试失败:', error)
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    }
  }
}

/**
 * 性能测试
 */
const performanceTest = async () => {
  console.log('\n🏃‍♂️ 开始性能测试...')

  const startTime = Date.now()

  // 创建多个并发任务
  const tasks = Array.from({ length: 10 }, (_, i) => testRefactoredArchitecture())

  const results = await Promise.all(tasks)
  const successCount = results.filter(r => r.success).length

  const totalTime = Date.now() - startTime

  console.log(`✅ 性能测试完成: ${successCount}/10 成功`)
  console.log(`⏱️ 总耗时: ${totalTime}ms`)
  console.log(`📊 平均耗时: ${totalTime / 10}ms/任务`)
}

// 如果直接运行此文件，执行测试
if (require.main === module) {
  ;(async () => {
    await testRefactoredArchitecture()
    await performanceTest()
  })()
}

export { testRefactoredArchitecture, performanceTest }
