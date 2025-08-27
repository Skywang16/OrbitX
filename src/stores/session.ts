import { defineStore } from 'pinia'
import { ref, computed, readonly } from 'vue'
import { restoreStateCurrent, StateFlags } from '@tauri-apps/plugin-window-state'
import { type SessionState, type TerminalState, type UiState, type AiState } from '@/types/domain/storage'
import { createDefaultSessionState } from '@/types/utils/helpers'
import { handleErrorWithMessage } from '@/utils/errorHandler'
import { storageApi } from '@/api/storage'

/**
 * 精简版会话状态管理Store
 */
export const useSessionStore = defineStore('session', () => {
  /** 当前会话状态 */
  const sessionState = ref<SessionState>(createDefaultSessionState())

  /** 是否正在加载 */
  const isLoading = ref(false)

  /** 是否正在保存 */
  const isSaving = ref(false)

  /** 错误信息 */
  const error = ref<string | null>(null)

  /** 是否已初始化 */
  const initialized = ref(false)

  /** 是否正在执行操作 */
  const isOperating = computed(() => isLoading.value || isSaving.value)

  /** 终端状态列表 */
  const terminals = computed(() => sessionState.value.terminals)

  /** 活跃的终端 */
  const activeTerminal = computed(() => sessionState.value.terminals.find(t => t.active) || null)

  /** UI状态 */
  const uiState = computed(() => sessionState.value.ui)

  /** AI状态 */
  const aiState = computed(() => sessionState.value.ai)

  const saveSessionState = async (): Promise<void> => {
    if (isSaving.value) return

    try {
      isSaving.value = true
      error.value = null

      sessionState.value.timestamp = new Date().toISOString()

      await storageApi.saveSessionState(sessionState.value)
    } catch (err) {
      const message = handleErrorWithMessage(err, '保存会话状态失败')
      error.value = message
      throw err
    } finally {
      isSaving.value = false
    }
  }

  const loadSessionState = async (): Promise<void> => {
    if (isLoading.value) return

    try {
      isLoading.value = true
      error.value = null

      const state = await storageApi.loadSessionState()

      if (state) {
        sessionState.value = state
      }

      await restoreWindowState()
    } catch (err) {
      const message = handleErrorWithMessage(err, '加载会话状态失败')
      error.value = message
      sessionState.value = createDefaultSessionState()
    } finally {
      isLoading.value = false
    }
  }

  const updateTerminals = (terminals: TerminalState[]): void => {
    sessionState.value.terminals = terminals
    saveSessionState().catch(() => {})
  }

  const addTerminal = (terminal: TerminalState): void => {
    sessionState.value.terminals.forEach(t => (t.active = false))
    sessionState.value.terminals.push(terminal)
    saveSessionState().catch(() => {})
  }

  const removeTerminal = (terminalId: string): void => {
    const index = sessionState.value.terminals.findIndex(t => t.id === terminalId)
    if (index !== -1) {
      sessionState.value.terminals.splice(index, 1)

      if (!sessionState.value.terminals.some(t => t.active) && sessionState.value.terminals.length > 0) {
        sessionState.value.terminals[0].active = true
      }

      saveSessionState().catch(() => {})
    }
  }

  const activateTerminal = (terminalId: string): void => {
    sessionState.value.terminals.forEach(t => {
      t.active = t.id === terminalId
    })
    sessionState.value.activeTabId = terminalId
    saveSessionState().catch(() => {})
  }

  const setActiveTabId = (tabId: string | null | undefined): void => {
    sessionState.value.activeTabId = tabId || undefined
    saveSessionState().catch(() => {})
  }

  const updateUiState = (updates: Partial<UiState>): void => {
    sessionState.value.ui = {
      ...sessionState.value.ui,
      ...updates,
    }
    saveSessionState().catch(() => {})
  }

  const updateAiState = (updates: Partial<AiState>): void => {
    sessionState.value.ai = {
      ...sessionState.value.ai,
      ...updates,
    }
    saveSessionState().catch(() => {})
  }

  const clearError = (): void => {
    error.value = null
  }

  const restoreWindowState = async (): Promise<void> => {
    try {
      await restoreStateCurrent(StateFlags.ALL)
    } catch (error) {
      console.warn('窗口状态恢复失败:', error)
    }
  }

  const cleanup = (): void => {}
  const initialize = async (): Promise<void> => {
    if (initialized.value) return

    try {
      await loadSessionState()
      initialized.value = true
    } catch (err) {
      console.error('会话状态管理初始化失败:', err)
      throw err
    }
  }

  return {
    // 状态
    sessionState: readonly(sessionState),
    isLoading: readonly(isLoading),
    isSaving: readonly(isSaving),
    error: readonly(error),
    initialized: readonly(initialized),

    // 计算属性
    isOperating,
    terminals,
    activeTerminal,
    uiState,
    aiState,

    // 核心方法
    saveSessionState,
    loadSessionState,
    restoreWindowState,
    initialize,
    cleanup,

    // 状态更新方法
    updateTerminals,
    addTerminal,
    removeTerminal,
    activateTerminal,
    setActiveTabId,
    updateUiState,
    updateAiState,

    // 工具方法
    clearError,
  }
})
