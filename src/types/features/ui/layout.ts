export enum TabType {
  TERMINAL = 'terminal',
  SETTINGS = 'settings',
}

export interface TabItem {
  id: string
  title: string
  type: TabType
  isActive: boolean
  closable?: boolean
  icon?: string
  data?: any
}
