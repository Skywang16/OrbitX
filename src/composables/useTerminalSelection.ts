import { ref, computed } from 'vue'

interface TerminalSelection {
  text: string
  startLine: number
  endLine: number
  path?: string
}

const selectedTerminalData = ref<TerminalSelection | null>(null)

export const useTerminalSelection = () => {
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

  // 清除选择 - 简化
  const clearSelection = () => {
    selectedTerminalData.value = null
  }

  return {
    // 状态
    selectedText,
    hasSelection,
    selectionInfo,
    // 方法
    setSelectedText,
    clearSelection,
    getSelectedText: () => selectedText.value, // 直接返回计算属性值
  }
}
