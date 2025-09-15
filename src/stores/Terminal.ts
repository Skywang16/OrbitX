import { shellApi, terminalApi, terminalContextApi } from '@/api'
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
  backendId: number | null
  shellInfo?: ShellInfo
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

  const _commandEventListeners = ref<Array<(terminalId: string, event: 'started' | 'finished', data?: any) => void>>([])

  const subscribeToCommandEvents = (
    callback: (terminalId: string, event: 'started' | 'finished', data?: any) => void
  ) => {
    _commandEventListeners.value.push(callback)
    return () => {
      const index = _commandEventListeners.value.indexOf(callback)
      if (index > -1) {
        _commandEventListeners.value.splice(index, 1)
      }
    }
  }

  const emitCommandEvent = (terminalId: string, event: 'started' | 'finished', data?: any) => {
    _commandEventListeners.value.forEach(callback => {
      try {
        callback(terminalId, event, data)
      } catch (error) {
        console.error('Command event callback error:', error)
      }
    })
  }

  let _globalListenersUnlisten: UnlistenFn[] = []
  let _isListenerSetup = false

  let nextId = 0

  const _pendingOperations = ref<Set<string>>(new Set())
  const _operationQueue = ref<Array<() => Promise<void>>>([])
  let _isProcessingQueue = false
  const MAX_CONCURRENT_OPERATIONS = 2

  const _performanceStats = ref({
    totalTerminalsCreated: 0,
    totalTerminalsClosed: 0,
    averageCreationTime: 0,
    maxConcurrentTerminals: 0,
    creationTimes: [] as number[],
  })

  const sessionStore = useSessionStore()

  // 保存持久化：使用轻量防抖合并短时间内的多次更新，避免保存风暴
  const debouncedPersist = debounce(() => {
    sessionStore.saveSessionState().catch(() => {})
  }, 80)

  const saveTerminalState = async () => {
    syncToSessionStore()
    await sessionStore.saveSessionState()
  }

  const immediateSync = () => {
    // 仅同步到 SessionStore，由具体的调用方（如 create/close/setActive）控制何时保存，避免重复/竞态保存
    syncToSessionStore()
    debouncedPersist()
  }

  watch(
    [terminals, activeTerminalId],
    () => {
      immediateSync()
    },
    { deep: true }
  )

  const activeTerminal = computed(() => terminals.value.find(t => t.id === activeTerminalId.value))
  const currentWorkingDirectory = computed(() => activeTerminal.value?.cwd || null)

  const generateId = (): string => {
    return `terminal-${nextId++}`
  }

  const queueOperation = async <T>(operation: () => Promise<T>): Promise<T> => {
    return new Promise(resolve => {
      const wrappedOperation = async () => {
        const result = await operation()
        resolve(result)
      }

      _operationQueue.value.push(wrappedOperation)
      processQueue()
    })
  }

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

  const recordPerformanceMetric = (type: 'create' | 'close', duration?: number) => {
    const stats = _performanceStats.value

    if (type === 'create') {
      stats.totalTerminalsCreated++
      if (duration) {
        stats.creationTimes.push(duration)
        if (stats.creationTimes.length > 100) {
          stats.creationTimes.shift()
        }
        stats.averageCreationTime = stats.creationTimes.reduce((a, b) => a + b, 0) / stats.creationTimes.length
      }
    } else if (type === 'close') {
      stats.totalTerminalsClosed++
    }

    const currentCount = terminals.value.length
    if (currentCount > stats.maxConcurrentTerminals) {
      stats.maxConcurrentTerminals = currentCount
    }
  }

  const setupGlobalListeners = async () => {
    listen('pane_cwd_changed', (event: any) => {
      const { pane_id, cwd } = event.payload
      if (pane_id && cwd) {
        const terminal = terminals.value.find(t => t.backendId === pane_id)
        if (terminal) {
          terminal.cwd = cwd
        }
      }
    })
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
        console.error('Error handling terminal CWD change event:', error)
      }
    })

    _globalListenersUnlisten = [unlistenOutput, unlistenExit, unlistenCwdChanged]
    _isListenerSetup = true
  }

  const teardownGlobalListeners = () => {
    _globalListenersUnlisten.forEach(unlisten => unlisten())
    _globalListenersUnlisten = []
    _isListenerSetup = false
  }

  const registerTerminalCallbacks = (id: string, callbacks: TerminalEventListeners) => {
    const listeners = _listeners.value.get(id) || []
    const entry: ListenerEntry = {
      id: `${id}-${Date.now()}`,
      callbacks,
    }
    listeners.push(entry)
    _listeners.value.set(id, listeners)
  }

  const unregisterTerminalCallbacks = (id: string, callbacks?: TerminalEventListeners) => {
    if (!callbacks) {
      _listeners.value.delete(id)
    } else {
      const listeners = _listeners.value.get(id) || []
      const filtered = listeners.filter(listener => listener.callbacks !== callbacks)
      if (filtered.length > 0) {
        _listeners.value.set(id, filtered)
      } else {
        _listeners.value.delete(id)
      }
    }
  }

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

  const unregisterResizeCallback = (terminalId: string) => {
    _resizeCallbacks.value.delete(terminalId)

    if (_resizeCallbacks.value.size === 0 && _globalResizeListener) {
      window.removeEventListener('resize', _globalResizeListener)
      _globalResizeListener = null
    }
  }

  const createTerminal = async (initialDirectory?: string): Promise<string> => {
    return queueOperation(async () => {
      const id = generateId()
      const startTime = Date.now()

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
      await setActiveTerminal(id)

      immediateSync()

      const duration = Date.now() - startTime
      recordPerformanceMetric('create', duration)

      return id
    })
  }

  const closeTerminal = async (id: string) => {
    return queueOperation(async () => {
      const terminal = terminals.value.find(t => t.id === id)
      if (!terminal) {
        console.warn(`尝试关闭不存在的终端: ${id}`)
        return
      }

      if (terminal.backendId === null) {
        await cleanupTerminalState(id)
        // 依赖 watch + 轻量防抖合并保存，避免重复保存
        immediateSync()
        return
      }

      unregisterTerminalCallbacks(id)

      const backendId = terminal.backendId
      terminal.backendId = null

      await terminalApi.closeTerminal(backendId)

      await cleanupTerminalState(id)
      // 依赖 watch + 轻量防抖合并保存，避免重复保存
      immediateSync()
      recordPerformanceMetric('close')
    })
  }

  const cleanupTerminalState = async (id: string) => {
    const index = terminals.value.findIndex(t => t.id === id)
    if (index !== -1) {
      terminals.value.splice(index, 1)
    }

    if (activeTerminalId.value === id) {
      if (terminals.value.length > 0) {
        await setActiveTerminal(terminals.value[0].id)
      } else {
        activeTerminalId.value = null
      }
    }
  }

  const setActiveTerminal = async (id: string) => {
    const targetTerminal = terminals.value.find(t => t.id === id)
    if (!targetTerminal) {
      console.warn(`尝试激活不存在的终端: ${id}`)
      return
    }

    activeTerminalId.value = id

    if (targetTerminal.backendId !== null) {
      await terminalContextApi.setActivePaneId(targetTerminal.backendId)
    }

    sessionStore.setActiveTabId(id)
    immediateSync()
  }

  const writeToTerminal = async (id: string, data: string) => {
    const terminal = terminals.value.find(t => t.id === id)
    if (!terminal || terminal.backendId === null) {
      console.error(`无法写入终端 '${id}': 未找到或无后端ID。`)
      return
    }

    await terminalApi.writeToTerminal({ paneId: terminal.backendId, data })
  }

  const resizeTerminal = async (id: string, rows: number, cols: number) => {
    const terminalSession = terminals.value.find(t => t.id === id)
    if (!terminalSession || terminalSession.backendId === null) {
      console.error(`无法调整终端 '${id}' 大小: 未找到或无后端ID。`)
      return
    }

    await terminalApi.resizeTerminal({
      paneId: terminalSession.backendId,
      rows,
      cols,
    })
  }

  const updateTerminalCwd = (id: string, cwd: string) => {
    const terminal = terminals.value.find(t => t.id === id)
    if (!terminal) {
      console.warn(`终端 ${id} 不存在，无法更新CWD`)
      return
    }

    if (terminal.cwd === cwd) {
      return
    }

    terminal.cwd = cwd

    updateTerminalTitle(terminal, cwd)
    immediateSync()
  }

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

      if (displayPath === '~') {
        newTitle = displayPath
      } else {
        const parts = displayPath === '/' ? [] : pathParts
        if (parts.length === 0) {
          newTitle = '/'
        } else {
          newTitle = parts.slice(-2).join('/')
          if (parts.length > 3) {
            newTitle = `…/${newTitle}`
          }
        }
      }

      if (newTitle.length > 30) {
        newTitle = `…${newTitle.slice(-27)}`
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

  const loadAvailableShells = async () => {
    shellManager.value.isLoading = true
    shellManager.value.error = null
    const shells = await shellApi.getAvailableShells()
    shellManager.value.availableShells = shells as ShellInfo[]
    shellManager.value.isLoading = false
  }

  const createAgentTerminal = async (agentName: string = 'AI Agent', initialDirectory?: string): Promise<string> => {
    return queueOperation(async () => {
      const id = generateId()
      const agentTerminalTitle = agentName

      // 检查是否已存在Agent专属终端（精确匹配Agent名称）
      const existingAgentTerminal = terminals.value.find(terminal => terminal.title === agentName)

      if (existingAgentTerminal) {
        await setActiveTerminal(existingAgentTerminal.id)
        existingAgentTerminal.title = agentTerminalTitle
        return existingAgentTerminal.id
      }

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
      await setActiveTerminal(id)
      await saveTerminalState()
      return id
    })
  }

  const createTerminalWithShell = async (shellName: string): Promise<string> => {
    return queueOperation(async () => {
      const id = generateId()
      const title = shellName

      const shellInfo = shellManager.value.availableShells.find(s => s.name === shellName)
      if (!shellInfo) {
        throw new Error(`未找到shell: ${shellName}`)
      }

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
      await setActiveTerminal(id)
      await saveTerminalState()

      return id
    })
  }

  const initializeShellManager = async () => {
    await loadAvailableShells()
  }

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

  const restoreFromSessionState = async () => {
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
      const id = await createTerminal(terminalState.cwd)

      const terminal = terminals.value.find(t => t.id === id)
      if (terminal) {
        terminal.title = terminalState.title
      }

      if (terminalState.active && shouldActivateTerminalId === null) {
        shouldActivateTerminalId = id
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
      await setActiveTerminal(terminalToActivate)
    }

    if (terminals.value.length === 0) {
      await createTerminal()
    }
    return true
  }

  const saveSessionState = async () => {
    syncToSessionStore()
    await sessionStore.saveSessionState()
  }

  const initializeTerminalStore = async () => {
    await initializeShellManager()
    await restoreFromSessionState()
    await setupGlobalListeners()
  }

  return {
    terminals,
    activeTerminalId,
    activeTerminal,
    currentWorkingDirectory,
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
    subscribeToCommandEvents,
    emitCommandEvent,
  }
})
