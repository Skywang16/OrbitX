/**
 * 新一代混合工具系统 - 统一导出
 *
 * 完全重构的工具架构，支持Function Calling与内置工具的智能混合
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
 * 创建配置好的混合工具管理器
 */
export function createConfiguredToolManager(): HybridToolManager {
  const manager = new HybridToolManager()

  // 注册所有终端工具
  manager.registerTools(getAllTerminalTools())

  // 设置智能自动策略
  manager.setStrategy('intelligent_auto')

  return manager
}

/**
 * 全局工具管理器实例
 */
export const globalToolManager = createConfiguredToolManager()

/**
 * 便捷的工具执行函数
 */
export async function executeTool(
  toolId: string,
  parameters: Record<string, any>,
  agentId: string,
  metadata?: Record<string, any>
) {
  return globalToolManager.execute(toolId, {
    agentId,
    parameters,
    metadata,
  })
}

/**
 * 获取所有可用工具
 */
export function getAvailableTools() {
  return globalToolManager.getTools()
}

/**
 * 获取工具执行统计
 */
export function getToolStats(toolId?: string) {
  return globalToolManager.getExecutionStats(toolId)
}
