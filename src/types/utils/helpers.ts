/**
 * 类型辅助工具
 */

// ===== 工具函数类型 =====

export const createDataQuery = (query: string) => ({
  query,
  params: {},
  desc: false,
})

export const createSaveOptions = (table?: string) => ({
  table,
  overwrite: false,
  backup: true,
  validate: true,
  metadata: {},
})

export const createDefaultSessionState = () => ({
  version: 1,
  terminals: [],
  activeTabId: undefined,
  ui: {
    theme: 'dark',
    fontSize: 14,
    sidebarWidth: 300,
  },
  ai: {
    visible: false,
    width: 350,
    mode: 'chat' as const,
    conversationId: undefined,
  },
  timestamp: new Date().toISOString(),
})

// ===== 格式化工具 =====

export const formatBytes = (bytes: number): string => {
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let size = bytes
  let unitIndex = 0

  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024
    unitIndex++
  }

  return `${size.toFixed(2)} ${units[unitIndex]}`
}
