/**
 * 内置工具导出 - 重构后
 *
 * 所有内置工具现在通过TerminalToolKit统一管理
 */

export * from '../TerminalToolKit'

// 导出工具创建函数
export { getAllTerminalTools as getBuiltinTools } from '../TerminalToolKit'
