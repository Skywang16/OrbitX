import type { TerminalConfig } from '@/types'

// 终端配置
export const TERMINAL_CONFIG: TerminalConfig = {
  fontFamily:
    'Menlo, Monaco, "SF Mono", "Microsoft YaHei UI", "PingFang SC", "Hiragino Sans GB", "Source Han Sans CN", "WenQuanYi Micro Hei", "Courier New", monospace',
  fontSize: 14,
  cursorBlink: true,
  theme: {
    background: '#1e1e1e',
    foreground: '#f0f0f0',
  },
  // 增强中文和UTF-8支持的配置
  allowTransparency: false,
  convertEol: true, // 自动转换行尾符，有助于处理不同系统的换行符
  cursorStyle: 'block',
  drawBoldTextInBrightColors: true,
  fontWeight: 'normal',
  fontWeightBold: 'bold',
  letterSpacing: 0,
  lineHeight: 1.2,
  macOptionIsMeta: false, // 在Mac上，Option键不作为Meta键，避免中文输入问题
  minimumContrastRatio: 1, // 保持原始颜色，避免对比度调整影响中文显示
  rightClickSelectsWord: false, // 避免右键选择干扰中文词语
  scrollback: 1000,
  scrollSensitivity: 1,
  tabStopWidth: 8,
  wordSeparator: ' ()[]{}\'",;', // 为中文优化的词分隔符
}

// 终端事件常量
export const TERMINAL_EVENTS = {
  OUTPUT: 'terminal_output',
  EXIT: 'terminal_exit',
} as const
