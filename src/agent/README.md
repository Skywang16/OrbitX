# 自主Agent框架

## 设计理念

这是一个真正的**自主Agent系统**，用户只需要用自然语言描述任务，Agent会自主完成：

1. **理解任务** - 分析用户意图
2. **制定计划** - 自主规划执行步骤
3. **执行计划** - 调用必要的工具和资源
4. **返回结果** - 提供清晰的执行结果

## 核心特点

### ✅ 真正的自主性

- 用户不需要了解内部实现
- 不需要手动规划或配置
- 就像与真人助手对话一样自然

### ✅ 简洁的API

- 主要API只有一个：`agent.execute(taskDescription)`
- 不暴露复杂的内部接口
- 专注于用户体验

### ✅ 智能决策

- Agent自主分析任务复杂度
- 自动选择合适的工具和方法
- 处理错误和异常情况

## 快速开始

```typescript
import { AgentFramework } from './agent'

// 创建Agent
const agent = new AgentFramework()

// 像对助手说话一样使用
const result = await agent.execute('帮我看看当前目录有什么文件')
console.log(result.result)
```

## 使用示例

### 基本任务

```typescript
// 文件操作
await agent.execute('创建一个名为test.txt的文件')
await agent.execute('删除所有.log文件')

// 系统查询
await agent.execute('检查系统内存使用情况')
await agent.execute('查看当前运行的进程')

// 开发任务
await agent.execute('安装项目依赖')
await agent.execute('运行测试用例')
```

### 复杂任务

```typescript
// Agent会自主分解复杂任务
await agent.execute('创建一个React项目，安装必要依赖，并设置基本的文件结构')

await agent.execute('分析代码质量，找出潜在问题，并生成改进建议')

await agent.execute('备份重要文件到指定目录，并压缩打包')
```

### 带进度反馈

```typescript
const result = await agent.execute('分析项目依赖', {
  onProgress: message => {
    console.log(`[${message.type}] ${message.content}`)
  },
})
```

### 流式反馈（适用于聊天界面）

```typescript
await agent.executeWithStream('优化项目性能', async message => {
  // 实时更新UI
  updateChatUI(message.content)
})
```

## 与传统框架的区别

### ❌ 传统方式（复杂）

```typescript
// 用户需要了解内部概念
const planner = new Planner()
const engine = new ExecutionEngine()

// 手动规划
const plan = await planner.planTask(task)

// 手动执行
const result = await engine.execute(plan.workflow)

// 处理复杂的状态管理
if (plan.needsReplanning) {
  const newPlan = await planner.replanTask(...)
  // ...更多复杂逻辑
}
```

### ✅ 自主方式（简单）

```typescript
// 用户只需要描述任务
const agent = new AgentFramework()
const result = await agent.execute('完成我的任务')
```

## 架构设计

```
用户任务描述
       ↓
   AgentFramework
       ↓
   [内部自主决策]
   ├── 任务理解
   ├── 计划制定
   ├── 工具选择
   ├── 执行监控
   └── 结果整理
       ↓
   返回最终结果
```

## 开发者调试

虽然用户不需要了解内部细节，但开发者可以使用调试方法：

```typescript
// 查看Agent的思考过程（仅用于调试）
const thinking = await agent.debugThinking('复杂任务')
console.log(thinking.workflow) // 查看生成的执行计划
```

## 配置选项

```typescript
const agent = new AgentFramework({
  maxConcurrency: 3, // 最大并发数
  defaultTimeout: 30000, // 默认超时时间
  enableParallelExecution: true, // 启用并行执行
})
```

## 错误处理

Agent会友好地处理各种错误情况：

```typescript
const result = await agent.execute('不可能的任务')

if (!result.success) {
  console.log(result.error)
  // 输出: "抱歉，我无法理解或完成这个任务: 任务描述可能不够清晰"
}
```

## 最佳实践

1. **清晰描述任务** - 像对真人助手说话一样
2. **一次一个任务** - 避免在单个请求中包含多个不相关的任务
3. **提供上下文** - 必要时说明当前环境或约束条件
4. **处理错误** - 检查返回结果的success字段

## 扩展性

框架内部使用模块化设计，支持：

- 自定义工具集成
- 新的Agent类型
- 自定义规划策略
- 插件系统

但这些都是内部实现细节，不影响用户的简洁体验。
