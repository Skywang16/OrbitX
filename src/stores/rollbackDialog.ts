import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { CheckpointSummary, FileDiff } from '@/types/domain/checkpoint'
import { checkpointApi } from '@/api/checkpoint'

export interface RollbackDialogState {
  checkpoint: CheckpointSummary | null
  messageId: number
  workspacePath: string
}

export const useRollbackDialogStore = defineStore('rollbackDialog', () => {
  const visible = ref(false)
  const loading = ref(false)
  const files = ref<FileDiff[]>([])
  const state = ref<RollbackDialogState | null>(null)

  const open = async (data: RollbackDialogState) => {
    state.value = data
    visible.value = true
    loading.value = true
    files.value = []

    try {
      if (data.checkpoint && data.checkpoint.parentId !== null) {
        files.value = await checkpointApi.diff(data.checkpoint.parentId, data.checkpoint.id)
      }
    } catch (error) {
      console.error('[RollbackDialog] Failed to load file diffs:', error)
      files.value = []
    } finally {
      loading.value = false
    }
  }

  const close = () => {
    visible.value = false
    state.value = null
    files.value = []
  }

  return {
    visible,
    loading,
    files,
    state,
    open,
    close,
  }
})
