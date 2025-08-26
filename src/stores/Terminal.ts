import { shellApi, terminalApi } from '@/api'
import type { ShellInfo } from '@/api'
import { useSessionStore } from '@/stores/session'
import type { TerminalState } from '@/types/domain/storage'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import { defineStore } from 'pinia'
import { computed, ref, watch, nextTick } from 'vue'
import { debounce } from 'lodash-es'
interface TerminalEventListeners {
  onOutput: (data: string) => void
  onExit: (exitCode: number | null) => void
}

interface ListenerEntry {
  id: string
  callbacks: TerminalEventListeners
}

type ResizeCallback = () => void

interface ShellManagerState {
  availableShells: ShellInfo[]
  isLoading: boolean
  error: string | null
}

export interface RuntimeTerminalState {
  id: string
  title: string
  cwd: string
  active: boolean
  shell?: string
  backendId: number | null // 后端进程ID
  shellInfo?: ShellInfo // Shell信息
}

export const useTerminalStore = defineStore('Terminal', () => {
  const terminals = ref<RuntimeTerminalState[]>([])
  const activeTerminalId = ref<string | null>(null)

  const shellManager = ref<ShellManagerState>({
    availableShells: [],
    isLoading: false,
    error: null,
  })

  const _listeners = ref<Map<string, ListenerEntry[]>>(new Map())

  const _resizeCallbacks = ref<Map<string, ResizeCallback>>(new Map())
  let _globalResizeListener: (() => void) | null = null

  let _globalListenersUnlisten: UnlistenFn[] = []
  let _isListenerSetup = false

  let nextId = 0

  const _pendingOperations = ref<Set<string>>(new Set())
  const _operationQueue = ref<Array<() => Promise<void>>>([])
  let _isProcessingQueue = false
  const MAX_CONCURRENT_OPERATIONS = 2 // 最多同时进行2个终端操作

  const _performanceStats = ref({
    totalTerminalsCreated: 0,
    totalTerminalsClosed: 0,
    averageCreationTime: 0,
    maxConcurrentTerminals: 0,
    creationTimes: [] as number[],
  })

  const sessionStore = useSessionStore()

  const debouncedSync = debounce(() => {
    syncToSessionStore()
  }, 500)

  watch(
    [terminals, activeTerminalId],
    () => {
      debouncedSync()
    },
    { deep: true }
  )

  const activeTerminal = computed(() => terminals.value.find(t => t.id === activeTerminalId.value))

  const generateId = (): string => {
    return `terminal-${nextId++}`
  }

  /**
   * 并发控制：将操作加入队列并按顺序执行
   */
  const queueOperation = async <T>(operation: () => Promise<T>): Promise<T> => {
    return new Promise((resolve, reject) => {
      const wrappedOperation = async () => {
        try {
          const result = await operation()
          resolve(result)
        } catch (error) {
          reject(error)
        }
      }

      _operationQueue.value.push(wrappedOperation)
      processQueue()
    })
  }

  /**
   * 处理操作队列
   */
  const processQueue = async () => {
    if (_isProcessingQueue || _operationQueue.value.length === 0) {
      return
    }

    if (_pendingOperations.value.size >= MAX_CONCURRENT_OPERATIONS) {
      return
    }

    _isProcessingQueue = true

    while (_operationQueue.value.length > 0 && _pendingOperations.value.size < MAX_CONCURRENT_OPERATIONS) {
      const operation = _operationQueue.value.shift()
      if (operation) {
        const operationId = `op-${Date.now()}-${Math.random()}`
        _pendingOperations.value.add(operationId)

        operation().finally(() => {
          _pendingOperations.value.delete(operationId)
          nextTick(() => processQueue())
        })
      }
    }

    _isProcessingQueue = false
  }

  /**
   * 记录性能指标
   */
  const recordPerformanceMetric = (type: 'create' | 'close', duration?: number) => {
    const stats = _performanceStats.value

    if (type === 'create') {
      stats.totalTerminalsCreated++
      if (duration) {
        stats.creationTimes.push(duration)
        // 保持最近100次的记录
        if (stats.creationTimes.length > 100) {
          stats.creationTimes.shift()
        }
        // 计算平均创建时间
        stats.averageCreationTime = stats.creationTimes.reduce((a, b) => a + b, 0) / stats.creationTimes.length
      }
    } else if (type === 'close') {
      stats.totalTerminalsClosed++
    }

    // 更新最大并发数
    const currentCount = terminals.value.length
    if (currentCount > stats.maxConcurrentTerminals) {
      stats.maxConcurrentTerminals = currentCount
    }
  }

  /**
   * 设置全局监听器，用于监听来自 Tauri 的所有终端事件。
   * 这个函数应该在应用启动时只调用一次。
   */
  const setupGlobalListeners = async () => {
    if (_isListenerSetup) return

    const findTerminalByBackendId = (backendId: number): RuntimeTerminalState | undefined => {
      return terminals.value.find(t => t.backendId === backendId)
    }

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

    const unlistenExit = await listen<{
      paneId: number
      exitCode: number | null
    }>('terminal_exit', event => {
      try {
        const terminal = findTerminalByBackendId(event.payload.paneId)
        if (terminal) {
          const listeners = _listeners.value.get(terminal.id) || []
          listeners.forEach(listener => listener.callbacks.onExit(event.payload.exitCode))

          closeTerminal(terminal.id)
        }
      } catch (error) {
        console.error('处理终端退出事件时发生错误:', error)
      }
    })

    const unlistenCwdChanged = await listen<{
      paneId: number
      cwd: string
    }>('pane_cwd_changed', event => {
      try {
        const terminal = findTerminalByBackendId(event.payload.paneId)
        if (terminal) {
          terminal.cwd = event.payload.cwd
          updateTerminalTitle(terminal, event.payload.cwd)
        }
      } catch (error) {
        console.error('处理终端CWD变化事件时发生错误:', error)
      }
    })

    _globalListenersUnlisten = [unlistenOutput, unlistenExit, unlistenCwdChanged]
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

  /** 注册终端resize回调，统一管理window resize监听器 */
  const registerResizeCallback = (terminalId: string, callback: ResizeCallback) => {
    _resizeCallbacks.value.set(terminalId, callback)

    if (_resizeCallbacks.value.size === 1 && !_globalResizeListener) {
      _globalResizeListener = () => {
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

  /** 注销终端resize回调 */
  const unregisterResizeCallback = (terminalId: string) => {
    _resizeCallbacks.value.delete(terminalId)

    if (_resizeCallbacks.value.size === 0 && _globalResizeListener) {
      window.removeEventListener('resize', _globalResizeListener)
      _globalResizeListener = null
    }
  }

  /**
   * 创建一个新的终端会话（使用系统默认shell）。
   */
  const createTerminal = async (initialDirectory?: string): Promise<string> => {
    return queueOperation(async () => {
      const id = generateId()
      const startTime = Date.now()

      try {
        const backendId = await terminalApi.createTerminal({
          rows: 24,
          cols: 80,
          cwd: initialDirectory,
        })

        const defaultShell = await shellApi.getDefaultShell()

        const terminal: RuntimeTerminalState = {
          id,
          title: defaultShell.name,
          cwd: initialDirectory || '~',
          active: false,
          shell: defaultShell.name,
          backendId,
          shellInfo: defaultShell as ShellInfo,
        }

        terminals.value.push(terminal)
        setActiveTerminal(id)

        const duration = Date.now() - startTime
        recordPerformanceMetric('create', duration)

        return id
      } catch (error) {
        console.error(`创建终端失败:`, error)
        throw error
      }
    })
  }

  /**
   * 关闭终端会话。
   */
  const closeTerminal = async (id: string) => {
    return queueOperation(async () => {
      const terminal = terminals.value.find(t => t.id === id)
      if (!terminal) {
        console.warn(`尝试关闭不存在的终端: ${id}`)
        return
      }

      if (terminal.backendId === null) {
        cleanupTerminalState(id)
        return
      }

      unregisterTerminalCallbacks(id)

      const backendId = terminal.backendId
      terminal.backendId = null

      try {
        await terminalApi.closeTerminal(backendId)
      } catch (error) {
        console.error(`关闭终端失败:`, error)
      }

      cleanupTerminalState(id)
      await saveSessionState()
      recordPerformanceMetric('close')
    })
  }

  /**
   * 清理终端的前端状态
   */
  const cleanupTerminalState = (id: string) => {
    const index = terminals.value.findIndex(t => t.id === id)
    if (index !== -1) {
      terminals.value.splice(index, 1)
    }

    // 如果关闭的是当前活动终端，需要切换到其他终端
    if (activeTerminalId.value === id) {
      if (terminals.value.length > 0) {
        setActiveTerminal(terminals.value[0].id)
      } else {
        activeTerminalId.value = null
        // 不再自动创建新终端，避免在应用关闭时产生竞态条件
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

    // 同步活跃标签页ID到会话状态
    sessionStore.setActiveTabId(id)
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
      await terminalApi.writeToTerminal({ paneId: terminal.backendId, data })
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
      await terminalApi.resizeTerminal({
        paneId: terminalSession.backendId,
        rows,
        cols,
      })
    } catch (error) {
      console.error(`调整终端 '${id}' 大小失败:`, error)
    }
  }

  /**
   * 更新终端的当前工作目录 - 增强版
   */
  const updateTerminalCwd = (id: string, cwd: string) => {
    const terminal = terminals.value.find(t => t.id === id)
    if (!terminal) {
      console.warn(`终端 ${id} 不存在，无法更新CWD`)
      return
    }

    if (terminal.cwd === cwd) {
      return // 路径没有变化，无需更新
    }

    terminal.cwd = cwd

    // 智能更新终端标题
    updateTerminalTitle(terminal, cwd)

    debouncedSync()
  }

  /** 根据当前工作目录生成更可读的终端标题 */
  const updateTerminalTitle = (terminal: RuntimeTerminalState, cwd: string) => {
    try {
      if (terminal.shell === 'agent') {
        return
      }

      let displayPath = cwd

      if (typeof window !== 'undefined' && (window as any).os && (window as any).os.homedir) {
        const homeDir = (window as any).os.homedir()
        if (homeDir && cwd.startsWith(homeDir)) {
          displayPath = cwd.replace(homeDir, '~')
        }
      }

      const pathParts = displayPath.split(/[/\\]/).filter(part => part.length > 0)

      let newTitle: string

      if (displayPath === '~' || displayPath === '/') {
        newTitle = displayPath
      } else if (pathParts.length === 0) {
        newTitle = '/'
      } else if (pathParts.length === 1) {
        newTitle = pathParts[0]
      } else {
        const lastTwo = pathParts.slice(-2)
        newTitle = lastTwo.join('/')
        if (pathParts.length > 3) {
          newTitle = `…/${newTitle}`
        }
      }

      if (newTitle.length > 30) {
        newTitle = '…' + newTitle.slice(-27)
      }

      if (terminal.title !== newTitle) {
        terminal.title = newTitle
      }
    } catch (error) {
      console.error('更新终端标题时发生错误:', error)
      const fallbackTitle = cwd.split(/[/\\]/).pop() || 'Terminal'
      if (terminal.title !== fallbackTitle) {
        terminal.title = fallbackTitle
      }
    }
  }

  /**
   * 获取可用的shell列表
   */
  const loadAvailableShells = async () => {
    shellManager.value.isLoading = true
    shellManager.value.error = null

    try {
      const shells = await shellApi.getAvailableShells()
      shellManager.value.availableShells = shells as ShellInfo[]
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
    return queueOperation(async () => {
      const id = generateId()
      const agentTerminalTitle = agentName

      // 检查是否已存在Agent专属终端（精确匹配Agent名称）
      const existingAgentTerminal = terminals.value.find(terminal => terminal.title === agentName)

      if (existingAgentTerminal) {
        setActiveTerminal(existingAgentTerminal.id)
        existingAgentTerminal.title = agentTerminalTitle
        return existingAgentTerminal.id
      }

      try {
        const backendId = await terminalApi.createTerminal({
          rows: 24,
          cols: 80,
          cwd: initialDirectory,
        })

        const terminal: RuntimeTerminalState = {
          id,
          title: agentTerminalTitle,
          cwd: initialDirectory || '~',
          active: false,
          shell: 'agent',
          backendId,
        }

        terminals.value.push(terminal)
        await new Promise(resolve => setTimeout(resolve, 100))
        setActiveTerminal(id)
        return id
      } catch (error) {
        console.error(`创建Agent终端失败:`, error)
        throw error
      }
    })
  }

  /** 使用指定shell创建终端 */
  const createTerminalWithShell = async (shellName: string): Promise<string> => {
    return queueOperation(async () => {
      const id = generateId()
      const title = shellName

      const shellInfo = shellManager.value.availableShells.find(s => s.name === shellName)
      if (!shellInfo) {
        throw new Error(`未找到shell: ${shellName}`)
      }

      try {
        const backendId = await terminalApi.createTerminalWithShell({
          shellName,
          rows: 24,
          cols: 80,
        })

        const terminal: RuntimeTerminalState = {
          id,
          title,
          cwd: shellInfo.path || '~',
          active: false,
          shell: shellInfo.name,
          backendId,
          shellInfo,
        }

        terminals.value.push(terminal)
        setActiveTerminal(id)
        await saveSessionState()

        return id
      } catch (error) {
        console.error(`创建终端失败:`, error)
        throw error
      }
    })
  }

  /**
   * 初始化shell管理器 */
  const initializeShellManager = async () => {
    await loadAvailableShells()
  }

  /**
   * 同步终端状态到会话存储
   */
  const syncToSessionStore = () => {
    const terminalStates: TerminalState[] = terminals.value.map(terminal => ({
      id: terminal.id,
      title: terminal.title,
      cwd: terminal.cwd,
      active: terminal.id === activeTerminalId.value,
      shell: terminal.shellInfo?.name,
    }))

    sessionStore.updateTerminals(terminalStates)
    sessionStore.setActiveTabId(activeTerminalId.value)
  }

  /**
   * 从会话状态恢复终端
   */
  const restoreFromSessionState = async () => {
    try {
      if (!sessionStore.initialized) {
        await sessionStore.initialize()
      }

      const terminalStates = sessionStore.terminals

      if (!terminalStates || terminalStates.length === 0) {
        return false
      }

      terminals.value = []
      activeTerminalId.value = null

      let shouldActivateTerminalId: string | null = null

      for (const terminalState of terminalStates) {
        try {
          const id = await createTerminal(terminalState.cwd)

          const terminal = terminals.value.find(t => t.id === id)
          if (terminal) {
            terminal.title = terminalState.title
          }

          if (terminalState.active && shouldActivateTerminalId === null) {
            shouldActivateTerminalId = id
          }
        } catch (error) {
          console.error(`恢复终端 ${terminalState.id} 失败:`, error)
        }
      }

      const savedActiveTabId = sessionStore.sessionState.activeTabId
      let terminalToActivate: string | null = null

      if (savedActiveTabId && terminals.value.find(t => t.id === savedActiveTabId)) {
        terminalToActivate = savedActiveTabId
      } else if (shouldActivateTerminalId) {
        terminalToActivate = shouldActivateTerminalId
      } else if (terminals.value.length > 0) {
        terminalToActivate = terminals.value[0].id
      }

      if (terminalToActivate) {
        setActiveTerminal(terminalToActivate)
      }

      if (terminals.value.length === 0) {
        await createTerminal()
      }
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
      syncToSessionStore()
      await sessionStore.saveSessionState()
    } catch (error) {
      console.error('❌ [Terminal Store] 保存终端会话状态失败:', error)
    }
  }

  /**
   * 初始化终端Store（包括会话恢复）
   */
  const initializeTerminalStore = async () => {
    try {
      await initializeShellManager()

      const restored = await restoreFromSessionState()

      if (!restored) {
        if (terminals.value.length === 0) {
          await createTerminal()
        }
      }

      await setupGlobalListeners()
    } catch (error) {
      console.error('终端Store初始化失败:', error)
      if (terminals.value.length === 0) {
        await createTerminal()
      }
    }
  }

  return {
    terminals,
    activeTerminalId,
    activeTerminal,
    shellManager,
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
    updateTerminalCwd,
    createTerminalWithShell,
    initializeShellManager,
    syncToSessionStore,
    restoreFromSessionState,
    saveSessionState,
    initializeTerminalStore,
  }
})
