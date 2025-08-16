/**
 * Eko工具系统 - 完全原生实现
 *
 * 🛠️ 工具模块 4.0 - 完全符合 Eko 框架规范
 *
 * Eko原生工具架构：
 *
 * 📁 文件操作工具：
 * - read-file: 📖 读取文件内容（支持行号、范围显示）
 *
 * 🔧 基础设施：
 * - EkoTool: Eko原生工具基类
 * - ParameterSchema: JSON Schema构建器
 * - EkoToolManager: 工具管理系统
 * - EkoToolRegistry: 工具注册表
 */

// 导出基础工具类
export * from './base/eko-tool'

// 导出工具管理系统
export * from './tool-manager'

// 导出具体工具实例
export { readFileTool } from './read-file'
export { createFileTool } from './create-file'

// 导出便捷函数
export {
  getAllTools,
  getTool,
  getToolsForMode,
  getToolsByCategory,
  registerTool,
  validateTools,
  generateToolsDocumentation,
  ekoToolManager,
  ToolCategory,
} from './tool-manager'

// 默认导出 - 所有工具
export { getAllTools as default } from './tool-manager'
