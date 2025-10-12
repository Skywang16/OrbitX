import { defineStore } from 'pinia'
import { ref, computed, readonly } from 'vue'
import { type SessionState, type TabState, type UiState, type AiState } from '@/types/domain/storage'
import { createDefaultSessionState } from '@/types/utils/helpers'
import { storageApi } from '@/api/storage'

/**
 * 会话状态管理Store - 统一 tab 管理
 */
export const useSessionStore = defineStore('session', () => {
  /** 当前会话状态 */
  const sessionState = ref<SessionState>(createDefaultSessionState())

  /** 是否正在加载 */
  const isLoading = ref(false)

  /** 是否正在保存 */
  const isSaving = ref(false)
  /** 有保存在进行时是否积压了新的保存请求 */
  const pendingSave = ref(false)

  /** 错误信息 */
  const error = ref<string | null>(null)

  /** 是否已初始化 */
  const initialized = ref(false)

  /** 是否正在执行操作 */
  const isOperating = computed(() => isLoading.value || isSaving.value)

  /** Tabs 列表 */
  const tabs = computed(() => sessionState.value.tabs)

  /** 活跃的 tab ID */
  const activeTabId = computed(() => sessionState.value.activeTabId)

  /** UI状态 */
  const uiState = computed(() => sessionState.value.ui)

  /** AI状态 */
  const aiState = computed(() => sessionState.value.ai)

  const saveSessionState = async (): Promise<void> => {
    if (isSaving.value) {
      pendingSave.value = true
      return
    }

    isSaving.value = true
    error.value = null

    try {
      do {
        pendingSave.value = false
        sessionState.value.timestamp = new Date().toISOString()
        await storageApi.saveSessionState(sessionState.value)
      } while (pendingSave.value)
    } finally {
      isSaving.value = false
    }
  }

  const loadSessionState = async (): Promise<void> => {
    if (isLoading.value) return
    isLoading.value = true
    error.value = null
    const state = await storageApi.loadSessionState().finally(() => {
      isLoading.value = false
    })
    if (state) {
      sessionState.value = state
    }
  }

  const updateTabs = (tabs: TabState[]): void => {
    sessionState.value.tabs = tabs
    saveSessionState().catch(() => {})
  }

  /**
   * 添加 tab
   */
  const addTab = (tab: TabState): void => {
    sessionState.value.tabs.push(tab)
    saveSessionState().catch(() => {})
  }

  /**
   * 删除 tab
   */
  const removeTab = (tabId: number | string): void => {
    sessionState.value.tabs = sessionState.value.tabs.filter(tab => tab.id !== tabId)
    saveSessionState().catch(() => {})
  }

  const setActiveTabId = (tabId: number | string | null | undefined): void => {
    sessionState.value.activeTabId = tabId ?? undefined
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
    sessionState: readonly(sessionState),
    isLoading: readonly(isLoading),
    isSaving: readonly(isSaving),
    error: readonly(error),
    initialized: readonly(initialized),

    isOperating,
    tabs,
    activeTabId,
    uiState,
    aiState,

    // 核心方法
    saveSessionState,
    loadSessionState,
    initialize,
    cleanup,

    // 状态更新方法
    updateTabs,
    addTab,
    removeTab,
    setActiveTabId,
    updateUiState,
    updateAiState,

    // 工具方法
    clearError,
  }
})
