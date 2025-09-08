/**
 * 向量索引状态管理 Composable
 *
 * 检查当前目录是否启用了向量索引功能
 */

import { ref, computed } from 'vue'
import { vectorIndexAppSettingsApi } from '@/api/vector-index/app-settings'
import { useTerminalStore } from '@/stores/Terminal'

const isCurrentDirectoryIndexed = ref(false)
const isLoading = ref(false)

export function useVectorIndexStatus() {
  const terminalStore = useTerminalStore()

  // 计算当前终端的工作目录
  const currentDirectory = computed(() => {
    // 获取当前激活的终端
    const activeTerminal = terminalStore.terminals.find(t => t.active)
    return activeTerminal?.cwd || ''
  })

  // 检查当前目录是否启用了向量索引
  const checkCurrentDirectoryIndex = async () => {
    if (!currentDirectory.value) {
      isCurrentDirectoryIndexed.value = false
      return
    }

    isLoading.value = true
    try {
      const isIndexed = await vectorIndexAppSettingsApi.isDirectoryIndexed(currentDirectory.value)
      isCurrentDirectoryIndexed.value = isIndexed
    } catch (error) {
      console.warn('检查当前目录向量索引状态失败:', error)
      isCurrentDirectoryIndexed.value = false
    } finally {
      isLoading.value = false
    }
  }

  return {
    isCurrentDirectoryIndexed: computed(() => isCurrentDirectoryIndexed.value),
    isLoading: computed(() => isLoading.value),
    currentDirectory,
    checkCurrentDirectoryIndex,
  }
}
