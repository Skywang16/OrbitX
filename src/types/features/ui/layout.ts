export enum TabType {
  TERMINAL = 'terminal',
  SETTINGS = 'settings',
}

export interface TabItem {
  id: string
  title?: string // 只用于非终端标签
  type: TabType
  closable?: boolean
  icon?: string
  data?: any
  // 终端专用字段
  shell?: string
  path?: string
}
