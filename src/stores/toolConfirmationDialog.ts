import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface ToolConfirmationDialogState {
  requestId: string
  workspacePath: string
  toolName: string
  summary: string
}

export type ToolConfirmationDecision = 'allow_once' | 'allow_always' | 'deny'

export const useToolConfirmationDialogStore = defineStore('toolConfirmationDialog', () => {
  const visible = ref(false)
  const submitting = ref(false)
  const remember = ref(false)
  const state = ref<ToolConfirmationDialogState | null>(null)

  const open = (data: ToolConfirmationDialogState) => {
    state.value = data
    remember.value = false
    submitting.value = false
    visible.value = true
  }

  const close = () => {
    visible.value = false
    submitting.value = false
    remember.value = false
    state.value = null
  }

  return {
    visible,
    submitting,
    remember,
    state,
    open,
    close,
  }
})
