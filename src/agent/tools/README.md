# 混合工具系统 - 完全重构版

## 概述

这是一个全新重构的Agent工具系统，实现了Function Calling与内置工具的智能混合架构。系统能够根据上下文和工具特性自主决策使用最佳的执行方式。

## 核心特性

### 🤖 智能决策引擎

- **自动策略选择**: LLM根据工具特性、执行环境和历史数据智能选择执行方式
- **混合执行**: 同时支持内置工具和Function Calling，发挥各自优势
- **动态优化**: 基于执行统计和成功率持续优化决策

### ⚡ 终端专业化

- **终端命令执行**: 安全的命令执行，支持实时输出捕获
- **会话管理**: 多终端会话的创建、切换和管理
- **环境监控**: 系统状态、进程、资源使用情况监控
- **文件操作**: 完整的文件系统操作支持

### 📊 性能监控

- **执行统计**: 详细的工具使用统计和性能指标
- **健康检查**: 系统健康状态监控和异常处理
- **决策分析**: 智能决策过程的可视化和分析

## 架构设计

```
HybridToolManager (核心管理器)
├── 智能决策引擎
│   ├── 工具特征分析
│   ├── 上下文评估
│   └── 执行策略选择
├── 执行引擎
│   ├── 内置工具执行
│   ├── Function Calling执行
│   └── 结果处理
└── 监控统计
    ├── 性能指标收集
    ├── 执行历史记录
    └── 健康状态检查

TerminalToolKit (终端工具套件)
├── 命令执行工具
├── 会话管理工具
├── 环境监控工具
└── 文件操作工具

Integration Layer (集成层)
├── NewToolAgent (新工具Agent)
├── ToolIntegrationManager (集成管理器)
└── 兼容性适配器
```

## 使用示例

### 基础使用

```typescript
import { globalToolManager, executeTool } from '@/agent/tools'

// 执行终端命令
const result = await executeTool(
  'terminal_execute',
  {
    command: 'ls -la',
    workingDirectory: '/home/user',
    timeout: 30,
  },
  'agent-001'
)

console.log(result.data) // 命令执行结果
```

### 高级配置

```typescript
import { HybridToolManager, createTerminalExecuteTool } from '@/agent/tools'

// 创建自定义工具管理器
const manager = new HybridToolManager()

// 设置执行策略
manager.setStrategy('intelligent_auto') // 智能自动
// manager.setStrategy('prefer_builtin') // 偏向内置
// manager.setStrategy('prefer_function_calling') // 偏向Function Calling

// 注册工具
manager.registerTool(createTerminalExecuteTool())

// 执行工具
const result = await manager.execute('terminal_execute', {
  agentId: 'my-agent',
  parameters: { command: 'pwd' },
  metadata: { preference: 'speed' },
})
```

### Agent集成

```typescript
import { NewToolAgent } from '@/agent/agents/NewToolAgent'

const toolAgent = new NewToolAgent({
  decisionStrategy: 'intelligent_auto',
  maxExecutionTime: 60000,
  enableExecutionStats: true,
})

// 执行Agent任务
const result = await toolAgent.execute(workflowAgent, executionContext)
```

## 工具定义

### 工具结构

```typescript
interface ToolDefinition {
  id: string
  name: string
  description: string
  category?: string
  type: 'builtin' | 'function_calling' | 'hybrid'
  parameters: ToolParameter[]
  functionCallSchema?: FunctionCallSchema
  builtinImplementation?: (params: Record<string, any>, context: ExecutionContext) => Promise<ToolResult>
  metadata?: Record<string, any>
}
```

### 创建自定义工具

```typescript
import { defineToolDefinition } from '@/agent/tools'

const myTool = {
  id: 'my_custom_tool',
  name: 'my_custom_tool',
  description: '我的自定义工具',
  category: 'custom',
  type: 'hybrid' as const,
  parameters: [
    {
      name: 'input',
      type: 'string' as const,
      description: '输入参数',
      required: true,
    },
  ],
  builtinImplementation: async (params, context) => {
    // 内置实现
    return {
      success: true,
      data: { result: `处理了: ${params.input}` },
    }
  },
  functionCallSchema: {
    type: 'function' as const,
    function: {
      name: 'my_custom_tool',
      description: '我的自定义工具',
      parameters: {
        type: 'object',
        properties: {
          input: { type: 'string', description: '输入参数' },
        },
        required: ['input'],
      },
    },
  },
}

globalToolManager.registerTool(myTool)
```

## 终端工具详解

### 1. terminal_execute - 命令执行

```typescript
await executeTool(
  'terminal_execute',
  {
    command: 'npm run build',
    workingDirectory: '/project',
    timeout: 300,
    captureStreaming: true,
  },
  agentId
)
```

### 2. terminal_session - 会话管理

```typescript
// 创建新会话
await executeTool(
  'terminal_session',
  {
    action: 'create',
    workingDirectory: '/project',
    sessionName: 'build-session',
  },
  agentId
)

// 切换会话
await executeTool(
  'terminal_session',
  {
    action: 'switch',
    sessionId: 'session-123',
  },
  agentId
)
```

### 3. terminal_monitor - 环境监控

```typescript
// 监控进程
await executeTool(
  'terminal_monitor',
  {
    monitorType: 'processes',
    detailed: true,
    filterPattern: 'node',
  },
  agentId
)

// 监控资源
await executeTool(
  'terminal_monitor',
  {
    monitorType: 'resources',
    detailed: false,
  },
  agentId
)
```

### 4. terminal_file_ops - 文件操作

```typescript
// 读取文件
await executeTool(
  'terminal_file_ops',
  {
    operation: 'read',
    path: '/path/to/file.txt',
  },
  agentId
)

// 写入文件
await executeTool(
  'terminal_file_ops',
  {
    operation: 'write',
    path: '/path/to/output.txt',
    content: 'Hello World',
  },
  agentId
)
```

## 决策机制

系统通过以下因素智能决策执行方式:

### 内置工具优势场景

- ✅ 终端操作 (+0.5分)
- ✅ 实时执行需求 (+0.3分)
- ✅ 高历史成功率 (+0.2分)
- ✅ 有内置实现 (+0.4分)

### Function Calling优势场景

- 🔄 复杂参数结构 (+0.3分)
- 🔄 需要自然语言处理 (+0.4分)
- 🔄 上下文感知处理 (+0.3分)
- 🔄 无内置实现 (+0.6分)

### 用户偏好调整

- **speed**: 内置工具权重 ×1.2
- **intelligence**: Function Calling权重 ×1.2
- **balanced**: 均衡执行

## 监控与统计

### 执行统计

```typescript
// 获取工具统计
const stats = globalToolManager.getExecutionStats('terminal_execute')
console.log(stats) // { totalExecutions, builtinExecutions, functionCallingExecutions, ... }

// 获取决策统计
const decisions = globalToolManager.getDecisionStats()
console.log(decisions) // 决策分布和平均分数
```

### 健康检查

```typescript
import { globalIntegrationManager } from '@/agent/integration/ToolIntegrationManager'

const health = globalIntegrationManager.getHealthStatus()
console.log(health.status) // 'healthy' | 'degraded' | 'unhealthy'
console.log(health.issues) // 问题列表
```

## 配置选项

### 工具管理器配置

```typescript
const manager = new HybridToolManager()

// 设置LLM提供商
manager.setLLMProvider(customLLMProvider)

// 设置执行策略
manager.setStrategy('intelligent_auto')

// 设置决策阈值
manager.setHybridDecisionThreshold(0.7)
```

### 集成配置

```typescript
const integrationManager = new ToolIntegrationManager({
  enableHybridTools: true,
  enableLegacyFallback: true,
  migrationMode: 'gradual',
  performanceMonitoring: true,
})
```

## 安全特性

### 命令安全

- 危险命令模式检测
- 路径安全验证
- 执行超时保护
- 权限级别检查

### 错误处理

- 详细错误分类
- 自动重试机制
- 优雅降级
- 异常监控

## 性能优化

### 执行优化

- 智能缓存机制
- 并发执行控制
- 资源使用监控
- 性能基准测试

### 内存管理

- 执行历史清理
- 统计数据压缩
- 内存泄漏检测
- 资源回收机制

## 扩展指南

### 添加新工具

1. 定义工具结构
2. 实现内置逻辑
3. 创建Function Call Schema
4. 注册到管理器
5. 编写测试用例

### 自定义决策策略

1. 继承HybridToolManager
2. 重写makeExecutionDecision方法
3. 实现自定义评分逻辑
4. 注册新策略

### 集成第三方LLM

1. 实现LLMProvider接口
2. 处理工具调用格式
3. 适配响应解析
4. 配置到工具管理器

## 故障排除

### 常见问题

**Q: 工具执行失败**
A: 检查参数有效性、权限设置和网络连接

**Q: Function Calling不工作**
A: 验证LLM配置和工具Schema格式

**Q: 性能问题**
A: 调整决策阈值和执行策略

**Q: 内存使用过高**
A: 启用统计数据清理和调整历史记录保留时间

### 调试模式

```typescript
// 启用详细日志
process.env.TOOL_DEBUG = 'true'

// 查看决策过程
const result = await manager.execute(toolId, context)
console.log(result.metadata.decision)
```

## 版本历史

### v2.0.0 - 混合架构重构

- 🆕 智能决策引擎
- 🆕 Function Calling集成
- 🆕 终端工具套件
- 🆕 性能监控系统
- 🔄 完全重构架构

### v1.x.x - 传统版本（已废弃）

- 基础工具注册和执行
- 简单权限控制
- 基础统计功能

## 许可证

MIT License - 详见项目根目录的LICENSE文件
