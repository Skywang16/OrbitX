import { defineStore } from 'pinia'
import { ref, computed, readonly } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import {
  type SessionState,
  type WindowState,
  type TerminalState,
  type UiState,
  type AiState,
} from '@/types/domain/storage'
import { createDefaultSessionState } from '@/types/utils/helpers'
import { handleErrorWithMessage } from '@/utils/errorHandler'

/**
 * 精简版会话状态管理Store
 */
export const useSessionStore = defineStore('session', () => {
  // ============================================================================
  // 状态定义
  // ============================================================================

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

  /** 自动保存定时器 */
  let autoSaveTimer: NodeJS.Timeout | null = null

  /** 自动保存间隔（毫秒） */
  const AUTO_SAVE_INTERVAL = 30000 // 30秒

  // ============================================================================
  // 计算属性
  // ============================================================================

  /** 是否正在执行操作 */
  const isOperating = computed(() => isLoading.value || isSaving.value)

  /** 当前窗口状态 */
  const windowState = computed(() => sessionState.value.window)

  /** 终端状态列表 */
  const terminals = computed(() => sessionState.value.terminals)

  /** 活跃的终端 */
  const activeTerminal = computed(() => sessionState.value.terminals.find(t => t.active) || null)

  /** UI状态 */
  const uiState = computed(() => sessionState.value.ui)

  /** AI状态 */
  const aiState = computed(() => sessionState.value.ai)

  // ============================================================================
  // 核心方法
  // ============================================================================

  /**
   * 保存会话状态到后端
   */
  const saveSessionState = async (): Promise<void> => {
    if (isSaving.value) return

    try {
      isSaving.value = true
      error.value = null

      // 更新时间戳
      sessionState.value.timestamp = new Date().toISOString()

      console.log('💾 [SessionStore] 保存会话状态:', sessionState.value)

      await invoke('storage_save_session_state', {
        sessionState: sessionState.value,
      })

      console.log('✅ [SessionStore] 会话状态保存成功')
    } catch (err) {
      const message = handleErrorWithMessage(err, '保存会话状态失败')
      error.value = message
      console.error('❌ [SessionStore] 保存会话状态失败:', err)
      throw err
    } finally {
      isSaving.value = false
    }
  }

  /**
   * 从后端加载会话状态
   */
  const loadSessionState = async (): Promise<void> => {
    if (isLoading.value) return

    try {
      isLoading.value = true
      error.value = null

      console.log('📥 [SessionStore] 加载会话状态')

      const state = await invoke<SessionState | null>('storage_load_session_state')

      if (state) {
        sessionState.value = state
        console.log('✅ [SessionStore] 会话状态加载成功:', state)
      } else {
        console.log('ℹ️ [SessionStore] 没有找到保存的会话状态，使用默认状态')
      }
    } catch (err) {
      const message = handleErrorWithMessage(err, '加载会话状态失败')
      error.value = message
      console.error('❌ [SessionStore] 加载会话状态失败:', err)
      // 加载失败时使用默认状态
      sessionState.value = createDefaultSessionState()
    } finally {
      isLoading.value = false
    }
  }

  // ============================================================================
  // 状态更新方法
  // ============================================================================

  /**
   * 更新窗口状态
   */
  const updateWindowState = (updates: Partial<WindowState>): void => {
    sessionState.value.window = {
      ...sessionState.value.window,
      ...updates,
    }
    scheduleAutoSave()
  }

  /**
   * 更新终端状态
   */
  const updateTerminals = (terminals: TerminalState[]): void => {
    sessionState.value.terminals = terminals
    scheduleAutoSave()
  }

  /**
   * 添加终端
   */
  const addTerminal = (terminal: TerminalState): void => {
    // 先将其他终端设为非活跃
    sessionState.value.terminals.forEach(t => (t.active = false))
    sessionState.value.terminals.push(terminal)
    scheduleAutoSave()
  }

  /**
   * 移除终端
   */
  const removeTerminal = (terminalId: string): void => {
    const index = sessionState.value.terminals.findIndex(t => t.id === terminalId)
    if (index !== -1) {
      sessionState.value.terminals.splice(index, 1)

      // 如果移除的是活跃终端，激活第一个终端
      if (!sessionState.value.terminals.some(t => t.active) && sessionState.value.terminals.length > 0) {
        sessionState.value.terminals[0].active = true
      }

      scheduleAutoSave()
    }
  }

  /**
   * 激活终端
   */
  const activateTerminal = (terminalId: string): void => {
    sessionState.value.terminals.forEach(t => {
      t.active = t.id === terminalId
    })
    // 同时更新活跃标签页ID
    sessionState.value.activeTabId = terminalId
    scheduleAutoSave()
  }

  /**
   * 设置活跃标签页ID
   */
  const setActiveTabId = (tabId: string | null | undefined): void => {
    sessionState.value.activeTabId = tabId || undefined
    scheduleAutoSave()
  }

  /**
   * 更新UI状态
   */
  const updateUiState = (updates: Partial<UiState>): void => {
    sessionState.value.ui = {
      ...sessionState.value.ui,
      ...updates,
    }
    scheduleAutoSave()
  }

  /**
   * 更新AI状态
   */
  const updateAiState = (updates: Partial<AiState>): void => {
    sessionState.value.ai = {
      ...sessionState.value.ai,
      ...updates,
    }
    scheduleAutoSave()
  }

  // ============================================================================
  // 自动保存
  // ============================================================================

  /**
   * 调度自动保存
   */
  const scheduleAutoSave = (): void => {
    if (autoSaveTimer) {
      clearTimeout(autoSaveTimer)
    }

    autoSaveTimer = setTimeout(() => {
      saveSessionState().catch(err => {
        console.warn('自动保存会话状态失败:', err)
      })
    }, AUTO_SAVE_INTERVAL)
  }

  /**
   * 立即保存（用于重要状态变化）
   */
  const saveImmediately = async (): Promise<void> => {
    if (autoSaveTimer) {
      clearTimeout(autoSaveTimer)
      autoSaveTimer = null
    }
    await saveSessionState()
  }

  /**
   * 开始自动保存
   */
  const startAutoSave = (): void => {
    scheduleAutoSave()
  }

  /**
   * 停止自动保存
   */
  const stopAutoSave = (): void => {
    if (autoSaveTimer) {
      clearTimeout(autoSaveTimer)
      autoSaveTimer = null
    }
  }

  /**
   * 清除错误
   */
  const clearError = (): void => {
    error.value = null
  }

  /**
   * 初始化会话状态管理
   */
  const initialize = async (): Promise<void> => {
    if (initialized.value) return

    try {
      await loadSessionState()
      startAutoSave()
      initialized.value = true
      console.log('✅ [SessionStore] 初始化完成')
    } catch (err) {
      console.error('❌ [SessionStore] 初始化失败:', err)
      throw err
    }
  }

  // ============================================================================
  // 返回Store接口
  // ============================================================================

  return {
    // 状态
    sessionState: readonly(sessionState),
    isLoading: readonly(isLoading),
    isSaving: readonly(isSaving),
    error: readonly(error),
    initialized: readonly(initialized),

    // 计算属性
    isOperating,
    windowState,
    terminals,
    activeTerminal,
    uiState,
    aiState,

    // 核心方法
    saveSessionState,
    loadSessionState,
    initialize,

    // 状态更新方法
    updateWindowState,
    updateTerminals,
    addTerminal,
    removeTerminal,
    activateTerminal,
    setActiveTabId,
    updateUiState,
    updateAiState,

    // 工具方法
    startAutoSave,
    stopAutoSave,
    saveImmediately,
    clearError,
  }
})
