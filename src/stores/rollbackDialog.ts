import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { CheckpointSummary, FileDiff } from '@/types/domain/checkpoint'
import { checkpointApi } from '@/api/checkpoint'
import { useCheckpoint } from '@/composables/useCheckpoint'

export interface RollbackDialogState {
  checkpoint: CheckpointSummary
  workspacePath: string
  messageContent: string
}

export const useRollbackDialogStore = defineStore('rollbackDialog', () => {
  const visible = ref(false)
  const loading = ref(false)
  const files = ref<FileDiff[]>([])
  const state = ref<RollbackDialogState | null>(null)

  const { getChildCheckpoint } = useCheckpoint()

  const open = async (data: RollbackDialogState) => {
    state.value = data
    visible.value = true
    loading.value = true
    files.value = []

    try {
      if (!data.workspacePath || data.workspacePath.trim().length === 0) {
        console.warn('[RollbackDialog] Missing workspace path, skip diff load')
        files.value = []
        return
      }

      const childCheckpoint = getChildCheckpoint(data.checkpoint.sessionId, data.workspacePath, data.checkpoint.id)

      if (childCheckpoint) {
        files.value = await checkpointApi.diff(data.checkpoint.id, childCheckpoint.id, data.workspacePath)
      } else {
        files.value = await checkpointApi.diffWithWorkspace(data.checkpoint.id, data.workspacePath)
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
