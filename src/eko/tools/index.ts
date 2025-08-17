/**
 * 工具模块主入口
 *
 * 🛠️ 工具模块重构版本 2.0
 *
 * 新的工具架构：
 *
 * 📁 文件操作工具：
 * - read-file: 📖 单文件读取（支持行号、范围、文件信息）
 * - read-many-files: 📚 批量文件读取（支持模式匹配、大小限制）
 * - create-file: 📄 文件创建（创建新文件或覆盖现有文件）
 * - edit-file: 📝 文件编辑（精确替换、行号定位、多种编辑模式）
 *
 * 🖥️ 系统工具：
 * - shell: 🔧 Shell命令执行（支持工作目录、环境变量、超时）
 *
 * 🌐 网络工具：
 * - web-fetch: 🌐 HTTP请求（支持各种方法、头部、超时）
 *
 * 🔍 搜索工具：
 * - semantic-search: 🧠 智能语义搜索（融合文本搜索、AST分析、语义理解）
 *
 * 🔧 基础设施：
 * - tool-error: 错误处理和类型定义
 * - tool-registry: 工具注册和管理系统
 * - modifiable-tool: 可扩展的工具基类
 */

// 导出所有工具
export * from './tools'

// 导出工具注册系统
export * from './tool-registry'

// 导出基础工具类
export * from './modifiable-tool'

// 导出错误类型
export * from './tool-error'

// 导出具体工具实例
export { readFileTool } from './toolList/read-file'
export { readManyFilesTool } from './toolList/read-many-files'
export { createFileTool } from './toolList/create-file'
export { editFileTool } from './toolList/edit-file'
export { shellTool } from './toolList/shell'
export { webFetchTool } from './toolList/web-fetch'

export { semanticSearchTool } from './toolList/semantic-search'

// 导出主要的工具集合
export {
  allTools,
  coreTools,
  networkTools,
  fileTools,
  searchTools,
  toolsByCategory,
  registerAllTools,
  getToolsForMode,
} from './tools'

// 默认导出核心工具
export { coreTools as default } from './tools'
