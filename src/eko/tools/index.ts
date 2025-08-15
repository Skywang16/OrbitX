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
 * - web-search: 🔍 网络搜索（支持多引擎、语言地区、安全搜索）
 *
 * 🧠 内存管理工具：
 * - memory: 🧠 会话内存管理（支持TTL、标签、模式匹配）
 *
 * 🔍 搜索工具：
 * - orbit-context: 🔍 智能代码库搜索（动态探索、多模式搜索、上下文理解）
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
export { readFileTool } from './read-file'
export { readManyFilesTool } from './read-many-files'
export { createFileTool } from './create-file'
export { editFileTool } from './edit-file'
export { shellTool } from './shell'
export { webFetchTool } from './web-fetch'
export { webSearchTool } from './web-search'
export { memoryTool } from './memoryTool'
export { orbitContextTool } from './orbit-context'

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
