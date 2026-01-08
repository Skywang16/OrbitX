import type { TerminalConfig } from '@/types'

// 终端配置 - 针对性能和渲染优化
export const TERMINAL_CONFIG: TerminalConfig = {
  // BaseConfig properties
  version: '1.0.0',
  lastModified: new Date().toISOString(),
  enabled: true,

  fontFamily:
    '"JetBrainsMono Nerd Font", "FiraCode Nerd Font", "Fira Code", "JetBrains Mono", Menlo, Monaco, "SF Mono", "Microsoft YaHei UI", "PingFang SC", "Hiragino Sans GB", "Source Han Sans CN", "WenQuanYi Micro Hei", "Apple Color Emoji", "Segoe UI Emoji", "Noto Color Emoji", "Courier New", monospace',
  fontSize: 14,
  allowProposedApi: true,
  allowTransparency: true,
  cursorBlink: true,
  theme: {
    background: '#1e1e1e',
    foreground: '#f0f0f0',
  },
  scrollback: 1000, // 减少滚动缓冲区，提升性能

  // Required configuration objects
  shell: {
    default: '/bin/zsh',
    args: [],
    workingDirectory: '~',
  },
  cursor: {
    style: 'block',
    blink: true,
    color: '#f0f0f0',
    thickness: 1,
  },
  behavior: {
    closeOnExit: false,
    confirmOnExit: true,
    scrollOnOutput: true,
    copyOnSelect: false,
  },

  convertEol: false,
  cursorStyle: 'block',
  drawBoldTextInBrightColors: true,
  fontWeight: 400,
  fontWeightBold: 700,
  letterSpacing: 0,
  lineHeight: 1.2,

  // 中文和国际化优化
  macOptionIsMeta: false, // 在Mac上，Option键不作为Meta键，避免中文输入问题
  minimumContrastRatio: 1, // 使用原始颜色，避免在浅色主题下被强制提亮为白色
  rightClickSelectsWord: false, // 避免右键选择干扰中文词语
  wordSeparator: ' ()[]{}\'",;', // 为中文优化的词分隔符

  // 滚动和缓冲区优化 - 针对 Canvas 渲染器性能调优
  scrollSensitivity: 1, // 降低滚动灵敏度，减少事件频率
  fastScrollSensitivity: 5, // 快速滚动时跳过更多行，减少渲染次数
  smoothScrollDuration: 0, // 禁用平滑滚动，减少渲染开销

  // 其他性能优化
  tabStopWidth: 4, // 减少tab宽度，更符合现代编程习惯
  screenReaderMode: false, // 禁用屏幕阅读器模式，提高性能
  windowsMode: false, // 禁用Windows模式，减少兼容性开销
}

// 终端事件常量
export const TERMINAL_EVENTS = {
  EXIT: 'terminal_exit',
} as const
