/**
 * UI布局相关类型定义
 */

// ===== 标签项类型 =====

export interface TabItem {
  id: string
  title: string
  isActive: boolean
  closable?: boolean
}

// ===== 窗口控制按钮类型 =====

export interface ButtonGroup {
  minimize?: boolean
  maximize?: boolean
  close?: boolean
  alwaysOnTop?: boolean
}
