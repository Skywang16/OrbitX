import { ref, computed } from 'vue'
import { useTabManagerStore } from '@/stores/TabManager'
import { useTerminalStore } from '@/stores/Terminal'
import { TabType } from '@/types'
import {
  TagType,
  type TerminalSelectionTag,
  type TerminalTabTag,
  type TagState,
  type TagContextInfo,
} from '@/types/tags'

interface TerminalSelection {
  text: string
  startLine: number
  endLine: number
  path?: string
}

const selectedTerminalData = ref<TerminalSelection | null>(null)

export const useTerminalSelection = () => {
  const tabManagerStore = useTabManagerStore()
  const terminalStore = useTerminalStore()

  // 计算属性 - 自动响应数据变化
  const hasSelection = computed(() => !!selectedTerminalData.value)
  const selectedText = computed(() => selectedTerminalData.value?.text ?? '')

  const selectionInfo = computed(() => {
    const data = selectedTerminalData.value
    if (!data) return ''

    const { startLine, endLine, path } = data

    // 提取路径的最后一部分，类似TabManager的逻辑
    let pathDisplay = 'terminal'
    if (path) {
      const pathParts = path.replace(/\/$/, '').split('/')
      pathDisplay = pathParts[pathParts.length - 1] || '~'
    }

    return startLine === endLine ? `${pathDisplay} ${startLine}:${startLine}` : `${pathDisplay} ${startLine}:${endLine}`
  })

  // 终端标签页相关计算属性
  const hasTerminalTab = computed(() => tabManagerStore.tabs.some(tab => tab.type === TabType.TERMINAL))

  const currentTerminalTab = computed(() => {
    const activeTab = tabManagerStore.activeTab
    if (!activeTab || activeTab.type !== TabType.TERMINAL) return null

    const terminal = terminalStore.terminals.find(t => t.id === activeTab.id)
    if (!terminal) return null

    return {
      terminalId: terminal.id,
      shell: terminal.shell || 'shell',
      cwd: terminal.cwd || '~',
      displayPath: activeTab.path || '~',
    }
  })

  // 设置选中文本 - 简化逻辑
  const setSelectedText = (text: string, startLine = 1, endLine?: number, path?: string) => {
    if (!text.trim()) {
      selectedTerminalData.value = null
      return
    }

    const lineCount = text.split('\n').length
    const actualEndLine = endLine ?? startLine + lineCount - 1

    selectedTerminalData.value = { text, startLine, endLine: actualEndLine, path }
  }

  // 清除选择
  const clearSelection = () => {
    selectedTerminalData.value = null
  }

  // 标签状态管理
  const getTagState = (): TagState => {
    const terminalSelectionTag: TerminalSelectionTag | null = selectedTerminalData.value
      ? {
          id: 'terminal-selection',
          type: TagType.TERMINAL_SELECTION,
          removable: true,
          selectedText: selectedTerminalData.value.text,
          selectionInfo: selectionInfo.value,
          startLine: selectedTerminalData.value.startLine,
          endLine: selectedTerminalData.value.endLine,
          path: selectedTerminalData.value.path,
        }
      : null

    const terminalTabTag: TerminalTabTag | null = currentTerminalTab.value
      ? {
          id: 'terminal-tab',
          type: TagType.TERMINAL_TAB,
          removable: true,
          terminalId: currentTerminalTab.value.terminalId,
          shell: currentTerminalTab.value.shell,
          cwd: currentTerminalTab.value.cwd,
          displayPath: currentTerminalTab.value.displayPath,
        }
      : null

    return {
      terminalSelection: terminalSelectionTag,
      terminalTab: terminalTabTag,
    }
  }

  const getTagContextInfo = (): TagContextInfo => {
    const result: TagContextInfo = {
      hasTerminalTab: hasTerminalTab.value,
      hasTerminalSelection: hasSelection.value,
    }

    if (currentTerminalTab.value) {
      result.terminalTabInfo = {
        terminalId: currentTerminalTab.value.terminalId,
        shell: currentTerminalTab.value.shell,
        cwd: currentTerminalTab.value.cwd,
      }
    }

    if (selectedTerminalData.value) {
      result.terminalSelectionInfo = {
        selectedText: selectedTerminalData.value.text,
        selectionInfo: selectionInfo.value,
        startLine: selectedTerminalData.value.startLine,
        endLine: selectedTerminalData.value.endLine,
        path: selectedTerminalData.value.path,
      }
    }

    return result
  }

  return {
    // 状态
    selectedText,
    hasSelection,
    selectionInfo,
    hasTerminalTab,
    currentTerminalTab,
    // 方法
    setSelectedText,
    clearSelection,
    getSelectedText: () => selectedText.value,
    // 新的标签管理方法
    getTagState,
    getTagContextInfo,
  }
}
