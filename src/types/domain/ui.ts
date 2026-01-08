/**
 * UI业务领域类型定义
 */

// ===== 组件基础类型 =====

export type ThemeMode = 'light' | 'dark' | 'auto'
export type Placement = 'top' | 'bottom' | 'left' | 'right'

export interface SelectOption {
  label: string
  value: string | number
  disabled?: boolean
}

// ===== 编辑器拖拽类型 =====

export type EditorDropZone = 'left' | 'right' | 'top' | 'bottom' | 'center'
export type EditorDragPhase = 'start' | 'move' | 'end'

export interface EditorDragPayload {
  phase: EditorDragPhase
  tabId: string
  sourceGroupId: string
  x: number
  y: number
}

export const EDITOR_TAB_DRAG_EVENT = 'orbitx-editor-tab-drag'
