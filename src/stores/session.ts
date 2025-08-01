/**
 * 会话状态管理Store
 *
 * 管理应用的会话状态，包括窗口状态、标签页、终端会话等
 * 支持自动保存、恢复和状态同步
 */

import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { storage } from '@/api/storage'
import {
  createDefaultSessionState,
  type SessionState,
  type WindowState,
  type TabState,
  type TerminalSession,
} from '@/types/storage'
import { handleErrorWithMessage } from '@/utils/errorHandler'

/**
 * 会话状态管理Store
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

  /** 是否正在恢复 */
  const isRestoring = ref(false)

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

  /** 是否有任何操作正在进行 */
  const isOperating = computed(() => isLoading.value || isSaving.value || isRestoring.value)

  /** 当前窗口状态 */
  const windowState = computed(() => sessionState.value.windowState)

  /** 当前标签页列表 */
  const tabs = computed(() => sessionState.value.tabs)

  /** 活跃的标签页 */
  const activeTab = computed(() => sessionState.value.tabs.find(tab => tab.isActive) || null)

  /** 终端会话列表 */
  const terminalSessions = computed(() => sessionState.value.terminalSessions)

  /** 活跃的终端会话 */
  const activeTerminalSessions = computed(() =>
    Object.values(sessionState.value.terminalSessions).filter(session => session.isActive)
  )

  /** UI状态 */
  const uiState = computed(() => sessionState.value.uiState)

  // ============================================================================
  // 核心方法
  // ============================================================================

  /**
   * 保存会话状态
   */
  async function saveSessionState(): Promise<void> {
    if (isSaving.value) return

    console.log('🔄 [前端] 开始保存会话状态')
    console.log('📊 [前端] 会话状态统计:', {
      终端会话数量: Object.keys(sessionState.value.terminalSessions).length,
      标签页数量: sessionState.value.tabs.length,
      版本: sessionState.value.version,
    })

    isSaving.value = true
    error.value = null

    try {
      // 更新时间戳和校验和
      const stateToSave = {
        ...sessionState.value,
        createdAt: new Date().toISOString(),
        checksum: generateChecksum(sessionState.value),
      }

      console.log('📤 [前端] 调用后端保存接口')
      await storage.saveSessionState(stateToSave)
      sessionState.value = stateToSave
      console.log('✅ [前端] 会话状态保存成功')
    } catch (err) {
      error.value = handleErrorWithMessage(err, '保存会话状态失败')
      console.error('❌ [前端] 保存会话状态失败:', err)
      throw err
    } finally {
      isSaving.value = false
    }
  }

  /**
   * 加载会话状态
   */
  async function loadSessionState(): Promise<SessionState | null> {
    if (isLoading.value) return null

    console.log('🔍 [前端] 开始加载会话状态')
    isLoading.value = true
    error.value = null

    try {
      console.log('📥 [前端] 调用后端加载接口')
      const savedState = await storage.loadSessionState()

      if (savedState) {
        console.log('✅ [前端] 会话状态加载成功')
        console.log('📊 [前端] 加载的会话状态统计:', {
          终端会话数量: Object.keys(savedState.terminalSessions).length,
          标签页数量: savedState.tabs.length,
          版本: savedState.version,
        })

        // 验证状态完整性
        if (validateSessionState(savedState)) {
          sessionState.value = savedState
          return savedState
        } else {
          console.warn('⚠️ [前端] 会话状态验证失败，使用默认状态')
          sessionState.value = createDefaultSessionState()
          return null
        }
      } else {
        console.log('ℹ️ [前端] 没有找到保存的会话状态，使用默认状态')
        sessionState.value = createDefaultSessionState()
        return null
      }
    } catch (err) {
      error.value = handleErrorWithMessage(err, '加载会话状态失败')
      console.error('❌ [前端] 加载会话状态失败:', err)
      sessionState.value = createDefaultSessionState()
      return null
    } finally {
      isLoading.value = false
    }
  }

  /**
   * 恢复会话状态
   */
  async function restoreSession(): Promise<boolean> {
    if (isRestoring.value) return false

    isRestoring.value = true
    error.value = null

    try {
      const restoredState = await loadSessionState()
      if (restoredState) {
        console.log('会话状态恢复成功')
        return true
      } else {
        console.log('没有找到可恢复的会话状态')
        return false
      }
    } catch (err) {
      error.value = handleErrorWithMessage(err, '恢复会话状态失败')
      console.error('恢复会话状态失败:', err)
      return false
    } finally {
      isRestoring.value = false
    }
  }

  // ============================================================================
  // 状态更新方法
  // ============================================================================

  /**
   * 更新窗口状态
   */
  function updateWindowState(newWindowState: Partial<WindowState>): void {
    sessionState.value.windowState = {
      ...sessionState.value.windowState,
      ...newWindowState,
    }
    scheduleAutoSave()
  }

  /**
   * 添加标签页
   */
  function addTab(tab: TabState): void {
    // 如果是第一个标签页，设为活跃
    if (sessionState.value.tabs.length === 0) {
      tab.isActive = true
    }

    sessionState.value.tabs.push(tab)
    scheduleAutoSave()
  }

  /**
   * 移除标签页
   */
  function removeTab(tabId: string): void {
    const tabIndex = sessionState.value.tabs.findIndex(tab => tab.id === tabId)
    if (tabIndex === -1) return

    const removedTab = sessionState.value.tabs[tabIndex]
    sessionState.value.tabs.splice(tabIndex, 1)

    // 如果移除的是活跃标签页，激活下一个
    if (removedTab.isActive && sessionState.value.tabs.length > 0) {
      const nextIndex = Math.min(tabIndex, sessionState.value.tabs.length - 1)
      sessionState.value.tabs[nextIndex].isActive = true
    }

    scheduleAutoSave()
  }

  /**
   * 激活标签页
   */
  function activateTab(tabId: string): void {
    sessionState.value.tabs.forEach(tab => {
      tab.isActive = tab.id === tabId
    })
    scheduleAutoSave()
  }

  /**
   * 更新标签页
   */
  function updateTab(tabId: string, updates: Partial<TabState>): void {
    const tab = sessionState.value.tabs.find(tab => tab.id === tabId)
    if (tab) {
      Object.assign(tab, updates)
      scheduleAutoSave()
    }
  }

  /**
   * 添加终端会话
   */
  function addTerminalSession(session: TerminalSession): void {
    sessionState.value.terminalSessions[session.id] = session
    scheduleAutoSave()
  }

  /**
   * 移除终端会话
   */
  function removeTerminalSession(sessionId: string): void {
    delete sessionState.value.terminalSessions[sessionId]
    scheduleAutoSave()
  }

  /**
   * 更新终端会话
   */
  function updateTerminalSession(sessionId: string, updates: Partial<TerminalSession>): void {
    const session = sessionState.value.terminalSessions[sessionId]
    if (session) {
      Object.assign(session, updates)
      scheduleAutoSave()
    }
  }

  /**
   * 更新UI状态
   */
  function updateUiState(updates: Partial<typeof sessionState.value.uiState>): void {
    sessionState.value.uiState = {
      ...sessionState.value.uiState,
      ...updates,
    }
    scheduleAutoSave()
  }

  // ============================================================================
  // 工具方法
  // ============================================================================

  /**
   * 生成状态校验和
   */
  function generateChecksum(state: SessionState): string {
    // 简单的校验和生成，实际项目中可以使用更复杂的算法
    const stateString = JSON.stringify({
      version: state.version,
      tabs: state.tabs.length,
      sessions: Object.keys(state.terminalSessions).length,
      timestamp: state.createdAt,
    })

    let hash = 0
    for (let i = 0; i < stateString.length; i++) {
      const char = stateString.charCodeAt(i)
      hash = (hash << 5) - hash + char
      hash = hash & hash // 转换为32位整数
    }

    return hash.toString(16)
  }

  /**
   * 验证会话状态
   */
  function validateSessionState(state: SessionState): boolean {
    try {
      // 基本结构验证
      if (
        typeof state.version !== 'number' ||
        !Array.isArray(state.tabs) ||
        typeof state.terminalSessions !== 'object' ||
        state.terminalSessions === null ||
        typeof state.uiState !== 'object' ||
        state.uiState === null ||
        typeof state.windowState !== 'object' ||
        state.windowState === null
      ) {
        console.warn('会话状态基本结构验证失败')
        return false
      }

      // 验证终端会话结构
      for (const [sessionId, session] of Object.entries(state.terminalSessions)) {
        if (
          !session ||
          typeof session.id !== 'string' ||
          typeof session.title !== 'string' ||
          typeof session.isActive !== 'boolean' ||
          typeof session.workingDirectory !== 'string' ||
          typeof session.createdAt !== 'string' ||
          typeof session.lastActive !== 'string' ||
          !Array.isArray(session.commandHistory) ||
          typeof session.environment !== 'object' ||
          session.environment === null
        ) {
          console.warn(`终端会话 ${sessionId} 结构验证失败:`, session)
          return false
        }
      }

      // 验证标签页结构
      for (const tab of state.tabs) {
        if (
          !tab ||
          typeof tab.id !== 'string' ||
          typeof tab.title !== 'string' ||
          typeof tab.isActive !== 'boolean' ||
          typeof tab.workingDirectory !== 'string' ||
          typeof tab.customData !== 'object'
        ) {
          console.warn('标签页结构验证失败:', tab)
          return false
        }
      }

      // 验证窗口状态结构
      const ws = state.windowState
      if (
        !Array.isArray(ws.position) ||
        ws.position.length !== 2 ||
        !Array.isArray(ws.size) ||
        ws.size.length !== 2 ||
        typeof ws.isMaximized !== 'boolean' ||
        typeof ws.isFullscreen !== 'boolean' ||
        typeof ws.isAlwaysOnTop !== 'boolean'
      ) {
        console.warn('窗口状态结构验证失败:', ws)
        return false
      }

      // 验证UI状态结构
      const ui = state.uiState
      if (
        typeof ui.sidebarVisible !== 'boolean' ||
        typeof ui.sidebarWidth !== 'number' ||
        typeof ui.currentTheme !== 'string' ||
        typeof ui.fontSize !== 'number' ||
        typeof ui.zoomLevel !== 'number' ||
        typeof ui.panelLayout !== 'object' ||
        ui.panelLayout === null
      ) {
        console.warn('UI状态结构验证失败:', ui)
        return false
      }

      return true
    } catch (error) {
      console.error('会话状态验证过程中发生错误:', error)
      return false
    }
  }

  /**
   * 调度自动保存
   */
  function scheduleAutoSave(): void {
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
   * 启动自动保存
   */
  function startAutoSave(): void {
    scheduleAutoSave()
  }

  /**
   * 停止自动保存
   */
  function stopAutoSave(): void {
    if (autoSaveTimer) {
      clearTimeout(autoSaveTimer)
      autoSaveTimer = null
    }
  }

  /**
   * 清除错误
   */
  function clearError(): void {
    error.value = null
  }

  /**
   * 初始化会话Store
   */
  async function initialize(): Promise<void> {
    if (initialized.value) return

    try {
      await restoreSession()
      startAutoSave()
      initialized.value = true
    } catch (err) {
      console.error('会话Store初始化失败:', err)
      throw err
    }
  }

  return {
    // 状态
    sessionState,
    isLoading,
    isSaving,
    isRestoring,
    error,
    initialized,

    // 计算属性
    isOperating,
    windowState,
    tabs,
    activeTab,
    terminalSessions,
    activeTerminalSessions,
    uiState,

    // 核心方法
    saveSessionState,
    loadSessionState,
    restoreSession,

    // 状态更新方法
    updateWindowState,
    addTab,
    removeTab,
    activateTab,
    updateTab,
    addTerminalSession,
    removeTerminalSession,
    updateTerminalSession,
    updateUiState,

    // 工具方法
    startAutoSave,
    stopAutoSave,
    clearError,
    initialize,
  }
})

export default useSessionStore
