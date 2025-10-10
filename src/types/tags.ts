/**
 * AI输入框标签系统类型定义
 */

export enum TagType {
  TERMINAL_SELECTION = 'terminal_selection',
  TERMINAL_TAB = 'terminal_tab',
  NODE_VERSION = 'node_version',
}

export interface BaseTag {
  id: string
  type: TagType
  removable: boolean
}

export interface TerminalSelectionTag extends BaseTag {
  type: TagType.TERMINAL_SELECTION
  selectedText: string
  selectionInfo: string
  startLine?: number
  endLine?: number
  path?: string
}

export interface TerminalTabTag extends BaseTag {
  type: TagType.TERMINAL_TAB
  terminalId: number
  shell: string
  cwd: string
  displayPath: string
}

export interface NodeVersionTag extends BaseTag {
  type: TagType.NODE_VERSION
  version: string
  manager: string
  cwd: string
}

export type AIChatTag = TerminalSelectionTag | TerminalTabTag | NodeVersionTag

export interface TagState {
  terminalSelection: TerminalSelectionTag | null
  terminalTab: TerminalTabTag | null
  nodeVersion: NodeVersionTag | null
}

export interface TagContextInfo {
  hasTerminalTab: boolean
  hasTerminalSelection: boolean
  hasNodeVersion: boolean
  terminalTabInfo?: {
    terminalId: number
    shell: string
    cwd: string
  }
  terminalSelectionInfo?: {
    selectedText: string
    selectionInfo: string
    startLine?: number
    endLine?: number
    path?: string
  }
  nodeVersionInfo?: {
    version: string
    manager: string
    cwd: string
  }
}
