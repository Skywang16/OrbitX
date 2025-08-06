/**
 * 工具系统导出
 */

export { HybridToolManager } from './HybridToolManager'
export type {
  ToolDefinition,
  ToolParameter,
  FunctionCallSchema,
  ExecutionContext,
  ToolResult,
  DecisionMetrics,
  ToolType,
  ExecutionStrategy,
} from './HybridToolManager'

export {
  createTerminalExecuteTool,
  createTerminalSessionTool,
  createTerminalMonitorTool,
  createTerminalFileOpsTool,
  getAllTerminalTools,
} from './TerminalToolKit'

// 创建默认工具管理器实例
import { HybridToolManager } from './HybridToolManager'
import { getAllTerminalTools } from './TerminalToolKit'

/**
 * 创建工具管理器
 */
export const createConfiguredToolManager = (): HybridToolManager => {
  const manager = new HybridToolManager()

  // 注册所有终端工具
  manager.registerTools(getAllTerminalTools())

  // 设置智能自动策略
  manager.setStrategy('intelligent_auto')

  return manager
}

/**
 * 全局工具管理器
 */
export const globalToolManager = createConfiguredToolManager()
