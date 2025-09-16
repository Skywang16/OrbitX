/**
 * 工具模块主入口
 *
 * 🛠️ 工具模块
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
 * - orbit-search: 🔍 语义搜索（基于向量的代码片段搜索，支持自然语言查询）
 *
 * 🔧 基础设施：
 * - tool-error: 错误处理和类型定义
 * - tool-registry: 工具注册和管理系统
 * - modifiable-tool: 可扩展的工具基类
 */

export * from './tools'

export * from './tool-registry'

export * from './modifiable-tool'

export * from './tool-error'

export { readFileTool } from './toolList/read-file'
export { readManyFilesTool } from './toolList/read-many-files'
export { createFileTool } from './toolList/create-file'
export { editFileTool } from './toolList/edit-file'
export { shellTool } from './toolList/shell'
export { webFetchTool } from './toolList/web-fetch'

export { orbitSearchTool } from './toolList/orbit-search'

export { allTools, readOnlyTools, registerAllTools, getToolsForMode } from './tools'

// 默认导出所有工具
export { allTools as default } from './tools'
