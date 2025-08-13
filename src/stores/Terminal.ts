import { shell as shellAPI } from '@/api/shell'
import { terminal as terminalAPI } from '@/api/terminal'
import type { ShellInfo } from '@/api/shell/types'
import { useSessionStore } from '@/stores/session'
import type { TabState, TerminalSession } from '@/types/storage'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'

// 组件可以注册的回调函数类型
interface TerminalEventListeners {
  onOutput: (data: string) => void
  onExit: (exitCode: number | null) => void
}

// 监听器条目类型
interface ListenerEntry {
  id: string
  callbacks: TerminalEventListeners
}

// Resize回调类型
type ResizeCallback = () => void

// Shell管理状态类型
interface ShellManagerState {
  availableShells: ShellInfo[]
  isLoading: boolean
  error: string | null
}

// 终端运行时会话类型，扩展存储型的 TerminalSession
export interface RuntimeTerminalSession extends TerminalSession {
  backendId: number | null // 后端进程ID
  shellInfo?: ShellInfo // Shell信息
}

export const useTerminalStore = defineStore('Terminal', () => {
  // --- 状态 ---
  const terminals = ref<RuntimeTerminalSession[]>([])
  const activeTerminalId = ref<string | null>(null)

  // Shell管理状态
  const shellManager = ref<ShellManagerState>({
    availableShells: [],
    isLoading: false,
    error: null,
  })

  // 存储组件注册的回调函数的映射表 - 支持多个监听器
  const _listeners = ref<Map<string, ListenerEntry[]>>(new Map())

  // Resize回调管理
  const _resizeCallbacks = ref<Map<string, ResizeCallback>>(new Map())
  let _globalResizeListener: (() => void) | null = null

  let _globalListenersUnlisten: UnlistenFn[] = []
  let _isListenerSetup = false

  let nextId = 0

  // 会话状态管理
  const sessionStore = useSessionStore()

  // 监听终端状态变化，同步到会话存储（但不立即保存到磁盘）
  watch(
    [terminals, activeTerminalId],
    () => {
      // 只同步到内存中的会话状态，不触发磁盘保存
      syncToSessionStore()
    },
    { deep: true }
  )

  // --- 计算属性 ---
  const activeTerminal = computed(() => terminals.value.find(t => t.id === activeTerminalId.value))

  const hasTerminals = computed(() => terminals.value.length > 0)

  // --- 操作方法 ---

  const generateId = (): string => {
    return `terminal-${nextId++}`
  }

  /**
   * 设置全局监听器，用于监听来自 Tauri 的所有终端事件。
   * 这个函数应该在应用启动时只调用一次。
   */
  const setupGlobalListeners = async () => {
    if (_isListenerSetup) return

    const findTerminalByBackendId = (backendId: number): RuntimeTerminalSession | undefined => {
      return terminals.value.find(t => t.backendId === backendId)
    }

    // 监听终端输出
    const unlistenOutput = await listen<{ paneId: number; data: string }>('terminal_output', event => {
      try {
        const terminal = findTerminalByBackendId(event.payload.paneId)
        if (terminal) {
          const listeners = _listeners.value.get(terminal.id) || []
          listeners.forEach(listener => listener.callbacks.onOutput(event.payload.data))
        }
      } catch (error) {
        console.error('处理终端输出事件时发生错误:', error)
      }
    })

    // 监听终端退出
    const unlistenExit = await listen<{
      paneId: number
      exitCode: number | null
    }>('terminal_exit', event => {
      try {
        const terminal = findTerminalByBackendId(event.payload.paneId)
        if (terminal) {
          const listeners = _listeners.value.get(terminal.id) || []
          listeners.forEach(listener => listener.callbacks.onExit(event.payload.exitCode))

          // 自动清理已关闭的终端会话
          closeTerminal(terminal.id)
        }
      } catch (error) {
        console.error('处理终端退出事件时发生错误:', error)
      }
    })

    _globalListenersUnlisten = [unlistenOutput, unlistenExit]
    _isListenerSetup = true
  }

  /**
   * 关闭全局监听器。
   */
  const teardownGlobalListeners = () => {
    _globalListenersUnlisten.forEach(unlisten => unlisten())
    _globalListenersUnlisten = []
    _isListenerSetup = false
  }

  /**
   * 由终端组件调用，用于注册其事件处理程序。
   */
  const registerTerminalCallbacks = (id: string, callbacks: TerminalEventListeners) => {
    const listeners = _listeners.value.get(id) || []
    const entry: ListenerEntry = {
      id: `${id}-${Date.now()}`,
      callbacks,
    }
    listeners.push(entry)
    _listeners.value.set(id, listeners)
  }

  /**
   * 当终端组件卸载时调用，用于清理资源。
   */
  const unregisterTerminalCallbacks = (id: string, callbacks?: TerminalEventListeners) => {
    if (!callbacks) {
      // 如果没有指定回调，清除所有监听器
      _listeners.value.delete(id)
    } else {
      // 只移除指定的监听器
      const listeners = _listeners.value.get(id) || []
      const filtered = listeners.filter(listener => listener.callbacks !== callbacks)
      if (filtered.length > 0) {
        _listeners.value.set(id, filtered)
      } else {
        _listeners.value.delete(id)
      }
    }
  }

  /**
   * 注册终端resize回调，统一管理window resize监听器
   */
  const registerResizeCallback = (terminalId: string, callback: ResizeCallback) => {
    _resizeCallbacks.value.set(terminalId, callback)

    // 如果是第一个回调，添加全局监听器
    if (_resizeCallbacks.value.size === 1 && !_globalResizeListener) {
      _globalResizeListener = () => {
        // 只对当前活跃的终端执行resize
        if (activeTerminalId.value) {
          const activeCallback = _resizeCallbacks.value.get(activeTerminalId.value)
          if (activeCallback) {
            activeCallback()
          }
        }
      }
      window.addEventListener('resize', _globalResizeListener)
    }
  }

  /**
   * 注销终端resize回调
   */
  const unregisterResizeCallback = (terminalId: string) => {
    _resizeCallbacks.value.delete(terminalId)

    // 如果没有回调了，移除全局监听器
    if (_resizeCallbacks.value.size === 0 && _globalResizeListener) {
      window.removeEventListener('resize', _globalResizeListener)
      _globalResizeListener = null
    }
  }

  /**
   * 创建一个新的终端会话（使用系统默认shell）。
   */
  const createTerminal = async (initialDirectory?: string): Promise<string> => {
    const id = generateId()

    try {
      // 先创建后端终端，确保成功后再添加到前端状态
      const backendId = await terminalAPI.create({
        rows: 24,
        cols: 80,
        cwd: initialDirectory,
      })

      // 获取系统默认shell信息
      const defaultShell = await shellAPI.getDefault()

      // 只有在后端创建成功后才创建前端会话记录
      const terminal: RuntimeTerminalSession = {
        id,
        title: defaultShell.name,
        workingDirectory: initialDirectory || '~',
        environment: {},
        commandHistory: [],
        isActive: false,
        createdAt: new Date().toISOString(),
        lastActive: new Date().toISOString(),
        backendId, // 直接设置有效的backendId
        shellInfo: defaultShell as ShellInfo,
      }

      // 添加到terminals数组，此时backendId已经有效
      terminals.value.push(terminal)
      setActiveTerminal(id)
      return id
    } catch (error) {
      console.error(`创建终端 '${id}' 失败:`, error)
      throw error
    }
  }

  /**
   * 关闭终端会话。
   */
  const closeTerminal = async (id: string) => {
    const terminal = terminals.value.find(t => t.id === id)
    if (!terminal) {
      console.warn(`尝试关闭不存在的终端: ${id}`)
      return
    }

    // 防止重复关闭：如果终端正在关闭过程中，直接返回
    if (terminal.backendId === null) {
      console.log(`终端 '${id}' 已经关闭或正在关闭中`)
      // 仍然需要清理前端状态
      cleanupTerminalState(id)
      return
    }

    unregisterTerminalCallbacks(id)

    // 先将 backendId 设为 null，防止重复关闭
    const backendId = terminal.backendId
    terminal.backendId = null

    try {
      await terminalAPI.close(backendId)
      console.log(`成功关闭终端后端: ${id} (backendId: ${backendId})`)
    } catch (error) {
      console.error(`关闭终端 '${id}' 的后端失败:`, error)
      // 即使后端关闭失败，也要清理前端状态
      // 这通常意味着后端面板已经不存在了
    }

    // 清理前端状态
    cleanupTerminalState(id)
  }

  /**
   * 清理终端的前端状态
   */
  const cleanupTerminalState = (id: string) => {
    const index = terminals.value.findIndex(t => t.id === id)
    if (index !== -1) {
      terminals.value.splice(index, 1)
      console.log(`已清理终端前端状态: ${id}`)
    }

    // 如果关闭的是当前活动终端，需要切换到其他终端
    if (activeTerminalId.value === id) {
      if (terminals.value.length > 0) {
        setActiveTerminal(terminals.value[0].id)
      } else {
        activeTerminalId.value = null
        // 异步创建新终端，避免阻塞当前操作
        createTerminal().catch(error => {
          console.error('自动创建新终端失败:', error)
        })
      }
    }
  }

  /**
   * 设置活动终端。
   */
  const setActiveTerminal = (id: string) => {
    // 确保终端存在
    const targetTerminal = terminals.value.find(t => t.id === id)
    if (!targetTerminal) {
      console.warn(`尝试激活不存在的终端: ${id}`)
      return
    }

    activeTerminalId.value = id
  }

  /**
   * 向终端写入数据。
   */
  const writeToTerminal = async (id: string, data: string) => {
    const terminal = terminals.value.find(t => t.id === id)
    if (!terminal || terminal.backendId === null) {
      console.error(`无法写入终端 '${id}': 未找到或无后端ID。`)
      return
    }

    try {
      await terminalAPI.write({ paneId: terminal.backendId, data })
    } catch (error) {
      console.error(`向终端 '${id}' 写入数据失败:`, error)
    }
  }

  /**
   * 调整终端大小。
   */
  const resizeTerminal = async (id: string, rows: number, cols: number) => {
    const terminalSession = terminals.value.find(t => t.id === id)
    if (!terminalSession || terminalSession.backendId === null) {
      console.error(`无法调整终端 '${id}' 大小: 未找到或无后端ID。`)
      return
    }

    try {
      await terminalAPI.resize({
        paneId: terminalSession.backendId,
        rows,
        cols,
      })
    } catch (error) {
      console.error(`调整终端 '${id}' 大小失败:`, error)
    }
  }

  // --- Shell管理方法 ---

  /**
   * 获取可用的shell列表
   */
  const loadAvailableShells = async () => {
    shellManager.value.isLoading = true
    shellManager.value.error = null

    try {
      const shells = await shellAPI.getAvailable()
      shellManager.value.availableShells = shells as ShellInfo[]
      console.log('已加载可用shell列表:', shells.length, '个')
    } catch (error) {
      console.error('获取可用shell列表失败:', error)
      shellManager.value.error = error instanceof Error ? error.message : '获取shell列表失败'
    } finally {
      shellManager.value.isLoading = false
    }
  }

  /**
   * 创建AI Agent专属终端
   */
  const createAgentTerminal = async (agentName: string = 'AI Agent', initialDirectory?: string): Promise<string> => {
    const id = generateId()
    const agentTerminalTitle = agentName

    // 检查是否已存在Agent专属终端（精确匹配Agent名称）
    const existingAgentTerminal = terminals.value.find(terminal => terminal.title === agentName)

    if (existingAgentTerminal) {
      // 如果已存在，静默激活现有终端
      setActiveTerminal(existingAgentTerminal.id)
      existingAgentTerminal.title = agentTerminalTitle
      existingAgentTerminal.lastActive = new Date().toISOString()

      // 不再输出重新激活信息，保持终端清洁

      return existingAgentTerminal.id
    }

    try {
      // 先创建后端终端，确保成功后再添加到前端状态
      const backendId = await terminalAPI.create({
        rows: 24,
        cols: 80,
        cwd: initialDirectory,
      })

      // 只有在后端创建成功后才创建前端会话记录
      const terminal: RuntimeTerminalSession = {
        id,
        title: agentTerminalTitle,
        workingDirectory: initialDirectory || '~',
        environment: {
          OrbitX_AGENT: agentName,
          OrbitX_TERMINAL_TYPE: 'agent',
        },
        commandHistory: [],
        isActive: false,
        createdAt: new Date().toISOString(),
        lastActive: new Date().toISOString(),
        backendId, // 直接设置有效的backendId
      }

      // 添加到terminals数组，此时backendId已经有效
      terminals.value.push(terminal)

      // 等待终端创建完成（可选的稳定性延迟）
      await new Promise(resolve => setTimeout(resolve, 100))

      setActiveTerminal(id)
      return id
    } catch (error) {
      console.error(`创建Agent终端 '${id}' 失败:`, error)
      throw error
    }
  }

  /**
   * 使用指定shell创建终端
   */
  const createTerminalWithShell = async (shellName: string): Promise<string> => {
    const id = generateId()
    const title = shellName

    // 查找shell信息
    const shellInfo = shellManager.value.availableShells.find(s => s.name === shellName)
    if (!shellInfo) {
      throw new Error(`未找到shell: ${shellName}`)
    }

    try {
      // 先创建后端终端，确保成功后再添加到前端状态
      const backendId = await terminalAPI.createWithShell({
        shellName,
        rows: 24,
        cols: 80,
      })

      // 只有在后端创建成功后才创建前端会话记录
      const terminal: RuntimeTerminalSession = {
        id,
        title,
        workingDirectory: shellInfo.path || '~',
        environment: {},
        commandHistory: [],
        isActive: false,
        createdAt: new Date().toISOString(),
        lastActive: new Date().toISOString(),
        backendId, // 直接设置有效的backendId
        shellInfo,
      }

      // 添加到terminals数组，此时backendId已经有效
      terminals.value.push(terminal)
      setActiveTerminal(id)
      return id
    } catch (error) {
      console.error(`创建终端 '${id}' 失败:`, error)
      throw error
    }
  }

  /**
   * 验证shell路径
   */
  const validateShellPath = async (path: string): Promise<boolean> => {
    try {
      return await shellAPI.validate(path)
    } catch (error) {
      console.error('验证shell路径失败:', error)
      return false
    }
  }

  /**
   * 初始化shell管理器
   */
  const initializeShellManager = async () => {
    await loadAvailableShells()
  }

  // ============================================================================
  // 会话状态管理
  // ============================================================================

  /**
   * 同步终端状态到会话存储（不触发自动保存）
   */
  const syncToSessionStore = () => {
    // 直接替换整个对象，避免触发 Session Store 的响应式更新
    const terminalSessions: Record<string, TerminalSession> = {}
    const tabs: TabState[] = []

    terminals.value.forEach(terminal => {
      // 创建终端会话记录
      const sessionData: TerminalSession = {
        id: terminal.id,
        title: terminal.title,
        workingDirectory: terminal.workingDirectory,
        environment: terminal.environment,
        commandHistory: terminal.commandHistory,
        isActive: terminal.id === activeTerminalId.value,
        createdAt: terminal.createdAt,
        lastActive: new Date().toISOString(),
      }

      terminalSessions[terminal.id] = sessionData

      // 创建标签页记录
      const tabData: TabState = {
        id: terminal.id,
        title: terminal.title,
        isActive: terminal.id === activeTerminalId.value,
        workingDirectory: terminal.workingDirectory,
        terminalSessionId: terminal.id,
        customData: {
          backendId: terminal.backendId,
          shellInfo: terminal.shellInfo,
        },
      }

      tabs.push(tabData)
    })

    // 直接替换，不使用 Session Store 的方法
    sessionStore.sessionState.terminalSessions = terminalSessions
    sessionStore.sessionState.tabs = tabs
  }

  /**
   * 从会话状态恢复终端
   */
  const restoreFromSessionState = async () => {
    try {
      const restored = await sessionStore.restoreSession()
      if (!restored) {
        console.log('没有找到可恢复的终端会话状态')
        return false
      }

      const { tabs, terminalSessions } = sessionStore.sessionState

      // 清空当前终端
      terminals.value = []
      activeTerminalId.value = null

      // 记录应该激活的终端ID
      let shouldActivateTerminalId: string | null = null

      // 恢复终端会话
      for (const tab of tabs) {
        if (tab.terminalSessionId && terminalSessions[tab.terminalSessionId]) {
          const sessionData = terminalSessions[tab.terminalSessionId]

          try {
            // 创建新的终端会话
            const id = await createTerminal(sessionData.workingDirectory)

            // 更新标题和其他元数据
            const terminal = terminals.value.find(t => t.id === id)
            if (terminal) {
              terminal.title = sessionData.title
              // 恢复命令历史
              terminal.commandHistory = [...sessionData.commandHistory]
              // 恢复环境变量
              terminal.environment = { ...sessionData.environment }
            }

            // 记录应该激活的终端（只记录第一个找到的活跃终端，避免被后续循环覆盖）
            if (tab.isActive && shouldActivateTerminalId === null) {
              shouldActivateTerminalId = id
              console.log(`🎯 [Terminal Store] 标记终端 ${id} 为应激活状态`)
            }
          } catch (error) {
            console.error(`恢复终端会话 ${tab.id} 失败:`, error)
          }
        }
      }

      // 现在激活正确的终端
      if (shouldActivateTerminalId) {
        setActiveTerminal(shouldActivateTerminalId)
        console.log(`✅ [Terminal Store] 激活恢复的终端: ${shouldActivateTerminalId}`)
      } else if (terminals.value.length > 0) {
        // 如果没有找到应该激活的终端，激活第一个
        setActiveTerminal(terminals.value[0].id)
        console.log(`⚠️ [Terminal Store] 未找到活跃标签，激活第一个终端: ${terminals.value[0].id}`)
      }

      // 如果没有任何终端，创建一个默认的
      if (terminals.value.length === 0) {
        await createTerminal()
        console.log('📝 [Terminal Store] 没有终端会话，创建默认终端')
      }

      console.log(
        `✅ [Terminal Store] 成功恢复 ${terminals.value.length} 个终端会话，活跃终端: ${activeTerminalId.value}`
      )
      return true
    } catch (error) {
      console.error('恢复终端会话状态失败:', error)
      return false
    }
  }

  /**
   * 保存当前终端状态到会话
   */
  const saveSessionState = async () => {
    try {
      console.log('💾 [Terminal Store] 开始保存终端会话状态')
      syncToSessionStore()
      await sessionStore.saveSessionState()
      console.log('✅ [Terminal Store] 终端会话状态保存完成')
    } catch (error) {
      console.error('❌ [Terminal Store] 保存终端会话状态失败:', error)
    }
  }

  /**
   * 初始化终端Store（包括会话恢复）
   */
  const initializeTerminalStore = async () => {
    try {
      // 首先初始化shell管理器
      await initializeShellManager()

      // 尝试恢复会话状态
      const restored = await restoreFromSessionState()

      if (!restored) {
        // 如果没有恢复成功，创建默认终端
        if (terminals.value.length === 0) {
          await createTerminal()
        }
      }

      // 设置全局监听器
      await setupGlobalListeners()

      console.log('终端Store初始化完成')
    } catch (error) {
      console.error('终端Store初始化失败:', error)
      // 确保至少有一个终端
      if (terminals.value.length === 0) {
        await createTerminal()
      }
    }
  }

  return {
    // 终端状态
    terminals,
    activeTerminalId,
    activeTerminal,
    hasTerminals,

    // Shell管理状态
    shellManager,

    // 终端管理方法
    setupGlobalListeners,
    teardownGlobalListeners,
    registerTerminalCallbacks,
    unregisterTerminalCallbacks,
    registerResizeCallback,
    unregisterResizeCallback,
    createTerminal,
    createAgentTerminal,
    closeTerminal,
    setActiveTerminal,
    writeToTerminal,
    resizeTerminal,

    // Shell管理方法
    loadAvailableShells,
    createTerminalWithShell,
    validateShellPath,
    initializeShellManager,

    // 会话状态管理方法
    syncToSessionStore,
    restoreFromSessionState,
    saveSessionState,
    initializeTerminalStore,
  }
})
