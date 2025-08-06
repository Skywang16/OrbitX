/**
 * 自主Agent使用示例
 *
 * 展示如何像与真人助手对话一样使用Agent
 */

import { AgentFramework } from '../index'

// 创建Agent实例
const agent = new AgentFramework()

// ===== 基本使用方式 =====

async function basicUsage() {
  // 就像对助手说话一样自然
  const result1 = await agent.execute('帮我看看当前目录有什么文件')
  console.log('文件列表:', result1.result)

  const result2 = await agent.execute('创建一个名为test.txt的文件，内容是Hello World')
  console.log('创建结果:', result2.result)

  const result3 = await agent.execute('检查系统内存使用情况')
  console.log('内存信息:', result3.result)
}

// ===== 带进度反馈的使用方式 =====

async function withProgress() {
  const result = await agent.execute('分析当前项目的依赖并生成报告', {
    onProgress: message => {
      console.log(`[${message.type}] ${message.content}`)
    },
  })

  console.log('分析结果:', result.result)
}

// ===== 流式反馈（适用于聊天界面）=====

async function streamingUsage() {
  await agent.executeWithStream('帮我优化这个项目的性能', async message => {
    // 这里可以实时更新UI
    console.log(`${message.timestamp}: ${message.content}`)
  })
}

// ===== 复杂任务示例 =====

async function complexTasks() {
  // Agent会自主分解复杂任务
  await agent.execute('创建一个React项目，安装必要依赖，并设置基本的文件结构')

  await agent.execute('分析代码质量，找出潜在问题，并生成改进建议')

  await agent.execute('备份重要文件到指定目录，并压缩打包')
}

// ===== 错误处理 =====

async function errorHandling() {
  const result = await agent.execute('执行一个不可能的任务')

  if (!result.success) {
    console.log('Agent说:', result.error)
    // 输出类似: "抱歉，我无法理解或完成这个任务: 任务描述可能不够清晰"
  }
}

// 运行示例
async function runExamples() {
  console.log('=== 基本使用 ===')
  await basicUsage()

  console.log('\n=== 带进度反馈 ===')
  await withProgress()

  console.log('\n=== 流式反馈 ===')
  await streamingUsage()

  console.log('\n=== 复杂任务 ===')
  await complexTasks()

  console.log('\n=== 错误处理 ===')
  await errorHandling()
}

// 导出示例函数
export { runExamples, basicUsage, withProgress, streamingUsage, complexTasks, errorHandling }
