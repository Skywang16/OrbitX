import type { TerminalConfig } from '@/types'

// 终端配置 - 针对性能和渲染优化
export const TERMINAL_CONFIG: TerminalConfig = {
  fontFamily:
    'Menlo, Monaco, "SF Mono", "Microsoft YaHei UI", "PingFang SC", "Hiragino Sans GB", "Source Han Sans CN", "WenQuanYi Micro Hei", "Courier New", monospace',
  fontSize: 14,
  cursorBlink: true,
  theme: {
    background: '#1e1e1e',
    foreground: '#f0f0f0',
  },

  convertEol: true, // 自动转换行尾符，有助于处理不同系统的换行符
  cursorStyle: 'block',
  drawBoldTextInBrightColors: true,
  fontWeight: 400,
  fontWeightBold: 700,
  letterSpacing: 0,
  lineHeight: 1.2,

  // 中文和国际化优化
  macOptionIsMeta: false, // 在Mac上，Option键不作为Meta键，避免中文输入问题
  minimumContrastRatio: 1, // 保持原始颜色，避免对比度调整影响中文显示
  rightClickSelectsWord: false, // 避免右键选择干扰中文词语
  wordSeparator: ' ()[]{}\'",;', // 为中文优化的词分隔符

  // 滚动和缓冲区优化
  scrollback: 2000, // 增加滚动缓冲区，提高实用性
  scrollSensitivity: 3, // 提高滚动灵敏度，改善用户体验
  fastScrollSensitivity: 5, // 快速滚动灵敏度
  smoothScrollDuration: 0, // 禁用平滑滚动，减少渲染开销

  // 其他性能优化
  tabStopWidth: 4, // 减少tab宽度，更符合现代编程习惯
  screenReaderMode: false, // 禁用屏幕阅读器模式，提高性能
  windowsMode: false, // 禁用Windows模式，减少兼容性开销
}

// 终端事件常量
export const TERMINAL_EVENTS = {
  OUTPUT: 'terminal_output',
  EXIT: 'terminal_exit',
} as const
