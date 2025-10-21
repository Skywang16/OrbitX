import { shellApi, storageApi, terminalApi, terminalContextApi, windowApi, workspaceApi } from '@/api'
import type { ShellInfo } from '@/api'
import { useSessionStore } from '@/stores/session'
import type { RuntimeTerminalState, TerminalTabState } from '@/types'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { defineStore } from 'pinia'
import { computed, ref, nextTick } from 'vue'
import { getHandler } from './TabHandlers'
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

export const useTerminalStore = defineStore('Terminal', () => {
  const terminals = ref<RuntimeTerminalState[]>([])
  const activeTerminalId = ref<number | null>(null)

  const shellManager = ref<ShellManagerState>({
    availableShells: [],
    isLoading: false,
    error: null,
  })

  const _listeners = ref<Map<number, ListenerEntry[]>>(new Map())

  const _resizeCallbacks = ref<Map<number, ResizeCallback>>(new Map())
  let _globalResizeListener: (() => void) | null = null

  type CommandEventType = 'started' | 'finished'
  interface CommandEventStartedPayload {
    commandId: string
  }
  interface CommandEventFinishedPayload {
    commandId: string
    exitCode: number
    isSuccess: boolean
  }
  type CommandEventPayload = CommandEventStartedPayload | CommandEventFinishedPayload
  type CommandEventCallback = (terminalId: number, event: CommandEventType, data?: CommandEventPayload) => void
  const _commandEventListeners = ref<CommandEventCallback[]>([])

  const subscribeToCommandEvents = (callback: CommandEventCallback) => {
    _commandEventListeners.value.push(callback)
    return () => {
      const index = _commandEventListeners.value.indexOf(callback)
      if (index > -1) {
        _commandEventListeners.value.splice(index, 1)
      }
    }
  }

  type CommandEventPayloadMap = {
    started: CommandEventStartedPayload
    finished: CommandEventFinishedPayload
  }
  const emitCommandEvent = <E extends CommandEventType>(
    terminalId: number,
    event: E,
    data: CommandEventPayloadMap[E]
  ): void => {
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

  const _pendingOperations = ref<Set<string>>(new Set())
  const _operationQueue = ref<Array<() => Promise<void>>>([])
  let _isProcessingQueue = false
  const MAX_CONCURRENT_OPERATIONS = 2

  const sessionStore = useSessionStore()

  // 缓存 home 目录，避免重复请求
  let _homeDirectory: string | null = null
  const getHomeDirectory = async (): Promise<string> => {
    if (!_homeDirectory) {
      _homeDirectory = await windowApi.getHomeDirectory()
    }
    return _homeDirectory
  }

  // 跟踪每个终端的初始目录，避免记录
  const _terminalInitialCwd = ref<Map<number, string>>(new Map())

  const activeTerminal = computed(() => terminals.value.find(t => t.id === activeTerminalId.value))
  const currentWorkingDirectory = computed(() => activeTerminal.value?.cwd || null)

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

  const setupGlobalListeners = async () => {
    if (_isListenerSetup) return

    const unlistenExit = await terminalApi.onTerminalExit(payload => {
      try {
        const terminal = terminals.value.find(t => t.id === payload.paneId)
        if (terminal) {
          const listeners = _listeners.value.get(terminal.id) || []
          listeners.forEach(listener => listener.callbacks.onExit(payload.exitCode))

          closeTerminal(terminal.id)
        }
      } catch (error) {
        console.error('处理终端退出事件时发生错误:', error)
      }
    })

    const unlistenCwdChanged = await terminalApi.onCwdChanged(payload => {
      try {
        const terminal = terminals.value.find(t => t.id === payload.paneId)
        if (terminal) {
          const previousCwd = terminal.cwd
          terminal.cwd = payload.cwd

          // CWD 变化，更新 SessionStore（仅保存 cwd，不同步 title）
          const existingTab = sessionStore.tabs.find(t => t.type === 'terminal' && t.id === terminal.id)
          if (existingTab && existingTab.type === 'terminal') {
            existingTab.data.cwd = payload.cwd
            sessionStore.updateTabs([...sessionStore.tabs])
          }

          // 记录工作区到最近列表
          // 排除：1) ~ 目录  2) home 目录  3) 终端的初始目录（首次 CWD 变化）
          const initialCwd = _terminalInitialCwd.value.get(payload.paneId)
          const isFirstCwdChange = initialCwd === undefined

          if (isFirstCwdChange) {
            // 记录初始目录，下次就不会记录了
            _terminalInitialCwd.value.set(payload.paneId, payload.cwd)
          } else if (payload.cwd && payload.cwd !== '~' && previousCwd !== payload.cwd) {
            // 只有在 CWD 真正变化时才记录
            getHomeDirectory()
              .then(homeDir => {
                if (payload.cwd !== homeDir) {
                  return workspaceApi.addRecentWorkspace(payload.cwd)
                }
              })
              .catch(error => {
                console.warn('Failed to record recent workspace:', error)
              })
          }
        }
      } catch (error) {
        console.error('Error handling terminal CWD change event:', error)
      }
    })

    _globalListenersUnlisten = [unlistenExit, unlistenCwdChanged]
    _isListenerSetup = true
  }

  const teardownGlobalListeners = () => {
    _globalListenersUnlisten.forEach(unlisten => unlisten())
    _globalListenersUnlisten = []
    _isListenerSetup = false
  }

  const registerTerminalCallbacks = (id: number, callbacks: TerminalEventListeners) => {
    const listeners = _listeners.value.get(id) || []
    const entry: ListenerEntry = {
      id: `${id}-${Date.now()}`,
      callbacks,
    }
    listeners.push(entry)
    _listeners.value.set(id, listeners)
  }

  const unregisterTerminalCallbacks = (id: number, callbacks?: TerminalEventListeners) => {
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

  // 由 Channel 订阅直接分发输出给已注册回调
  const dispatchOutputForPaneId = (paneId: number, data: string) => {
    const listeners = _listeners.value.get(paneId) || []
    listeners.forEach(listener => {
      try {
        listener.callbacks.onOutput(data)
      } catch (error) {
        console.error('分发终端输出时发生错误:', error)
      }
    })
  }

  const registerResizeCallback = (terminalId: number, callback: ResizeCallback) => {
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

  const unregisterResizeCallback = (terminalId: number) => {
    _resizeCallbacks.value.delete(terminalId)

    if (_resizeCallbacks.value.size === 0 && _globalResizeListener) {
      window.removeEventListener('resize', _globalResizeListener)
      _globalResizeListener = null
    }
  }

  const createTerminal = async (initialDirectory?: string): Promise<number> => {
    return queueOperation(async () => {
      const paneId = await terminalApi.createTerminal({
        rows: 24,
        cols: 80,
        cwd: initialDirectory,
      })

      const defaultShell = await shellApi.getDefaultShell()

      const terminal: RuntimeTerminalState = {
        id: paneId,
        cwd: initialDirectory || '~',
        shell: defaultShell.name,
      }

      const existingIndex = terminals.value.findIndex(t => t.id === paneId)
      if (existingIndex !== -1) {
        terminals.value.splice(existingIndex, 1)
      }
      terminals.value.push(terminal)

      // 添加到 SessionStore（不包含 title，直接使用 paneId 作为 ID）
      sessionStore.addTab({
        type: 'terminal',
        id: paneId,
        isActive: false,
        data: {
          shell: terminal.shell,
          shellType: terminal.shell,
          cwd: terminal.cwd,
        },
      })

      await setActiveTerminal(paneId)

      return paneId
    })
  }

  const closeTerminal = async (id: number) => {
    return queueOperation(async () => {
      const terminal = terminals.value.find(t => t.id === id)
      if (!terminal) {
        console.warn(`尝试关闭不存在的终端: ${id}`)
        return
      }

      // 清理终端的初始目录跟踪
      _terminalInitialCwd.value.delete(id)

      unregisterTerminalCallbacks(id)

      await terminalApi.closeTerminal(id)

      // 从 SessionStore 删除（直接使用 paneId）
      sessionStore.removeTab(id)

      await cleanupTerminalState(id)
    })
  }

  const cleanupTerminalState = async (id: number) => {
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

  const setActiveTerminal = async (id: number) => {
    const targetTerminal = terminals.value.find(t => t.id === id)
    if (!targetTerminal) {
      console.warn(`尝试激活不存在的终端: ${id}`)
      return
    }

    activeTerminalId.value = id

    await terminalContextApi.setActivePaneId(id)

    // 更新 SessionStore 的活跃 tab（直接使用 paneId）
    sessionStore.setActiveTab(id)
  }

  const writeToTerminal = async (id: number, data: string) => {
    const terminal = terminals.value.find(t => t.id === id)
    if (!terminal) {
      console.error(`无法写入终端 '${id}': 未找到。`)
      return
    }

    await terminalApi.writeToTerminal({ paneId: terminal.id, data })
  }

  const resizeTerminal = async (id: number, rows: number, cols: number) => {
    const terminalSession = terminals.value.find(t => t.id === id)
    if (!terminalSession) {
      console.error(`无法调整终端 '${id}' 大小: 未找到。`)
      return
    }

    await terminalApi.resizeTerminal({
      paneId: terminalSession.id,
      rows,
      cols,
    })
  }
  const loadAvailableShells = async () => {
    shellManager.value.isLoading = true
    shellManager.value.error = null
    const shells = await shellApi.getAvailableShells()
    shellManager.value.availableShells = shells as ShellInfo[]
    shellManager.value.isLoading = false
  }

  const createAgentTerminal = async (initialDirectory?: string): Promise<number> => {
    return queueOperation(async () => {
      const existingAgentTerminal = terminals.value.find(terminal => terminal.shell === 'agent')

      if (existingAgentTerminal) {
        await setActiveTerminal(existingAgentTerminal.id)
        return existingAgentTerminal.id
      }

      const paneId = await terminalApi.createTerminal({
        rows: 24,
        cols: 80,
        cwd: initialDirectory,
      })

      const terminal: RuntimeTerminalState = {
        id: paneId,
        cwd: initialDirectory || '~',
        shell: 'agent',
      }

      const existingIndex = terminals.value.findIndex(t => t.id === paneId)
      if (existingIndex !== -1) {
        terminals.value.splice(existingIndex, 1)
      }
      terminals.value.push(terminal)

      sessionStore.addTab({
        type: 'terminal',
        id: paneId,
        isActive: false,
        data: {
          shell: terminal.shell,
          shellType: terminal.shell,
          cwd: terminal.cwd,
        },
      })

      await new Promise(resolve => setTimeout(resolve, 100))
      await setActiveTerminal(paneId)

      return paneId
    })
  }

  const createTerminalWithShell = async (shellName: string): Promise<number> => {
    return queueOperation(async () => {
      const shellInfo = shellManager.value.availableShells.find(s => s.name === shellName)
      if (!shellInfo) {
        throw new Error(`未找到shell: ${shellName}`)
      }

      const paneId = await terminalApi.createTerminalWithShell({
        shellName,
        rows: 24,
        cols: 80,
      })

      const terminal: RuntimeTerminalState = {
        id: paneId,
        cwd: shellInfo.path || '~',
        shell: shellInfo.name,
      }

      const existingIndex = terminals.value.findIndex(t => t.id === paneId)
      if (existingIndex !== -1) {
        terminals.value.splice(existingIndex, 1)
      }
      terminals.value.push(terminal)

      sessionStore.addTab({
        type: 'terminal',
        id: paneId,
        isActive: false,
        data: {
          shell: terminal.shell,
          shellType: terminal.shell,
          cwd: terminal.cwd,
        },
      })

      await setActiveTerminal(paneId)

      return paneId
    })
  }

  const initializeShellManager = async () => {
    await loadAvailableShells()
  }

  const restoreFromSessionState = async () => {
    await sessionStore.initialize()

    // 1. 从后端获取运行时终端状态（进程是否还活着）
    let runtimeStates: RuntimeTerminalState[] = []
    try {
      runtimeStates = await storageApi.getTerminalsState()
    } catch (error) {
      console.error('获取终端运行时状态失败:', error)
      return false
    }

    // 2. 从 SessionStore 获取保存的 tabs
    const allTabs = sessionStore.tabs
    const terminalTabs = allTabs.filter((t): t is TerminalTabState => t.type === 'terminal')

    // 3. 构建运行时 Map，快速查找
    const runtimeMap = new Map(runtimeStates.map(r => [r.id, r]))

    // 4. 恢复逻辑：按 session 顺序处理终端 tabs
    const restored: RuntimeTerminalState[] = []
    const restoredTabIds = new Set<number>()

    for (const tab of terminalTabs) {
      const runtime = runtimeMap.get(tab.id)

      if (runtime) {
        // 进程还活着 → 用运行时数据（cwd 可能已经变化）
        restored.push(runtime)
        restoredTabIds.add(tab.id)
        runtimeMap.delete(tab.id)
      } else {
        // 进程死了 → 从 session 数据重新创建终端进程
        console.warn(`终端 ${tab.id} 进程已终止，尝试重新创建`)
        try {
          const shellName = tab.data.shell || 'default'
          const cwd = tab.data.cwd

          const paneId = await terminalApi.createTerminal({
            rows: 24,
            cols: 80,
            cwd,
          })

          const newTerminal: RuntimeTerminalState = {
            id: paneId,
            cwd: cwd || '~',
            shell: shellName,
          }

          restored.push(newTerminal)
          restoredTabIds.add(paneId)

          // 更新 SessionStore 中的 tab ID（新进程新 paneId）
          tab.id = paneId
        } catch (error) {
          console.error(`重新创建终端失败:`, error)
        }
      }
    }

    // 5. 后端有但 session 没有的终端（孤儿进程）→ 也保留
    if (runtimeMap.size > 0) {
      console.warn(`发现 ${runtimeMap.size} 个孤儿终端进程，将保留`)
      for (const runtime of runtimeMap.values()) {
        restored.push(runtime)
        restoredTabIds.add(runtime.id)

        // 添加到 SessionStore
        allTabs.push({
          type: 'terminal',
          id: runtime.id,
          isActive: false,
          data: {
            shell: runtime.shell,
            shellType: runtime.shell,
            cwd: runtime.cwd,
          },
        })
      }
    }

    // 6. 更新 terminals 和 SessionStore
    terminals.value = restored

    // 重新构建 tabs：保持原始顺序，只过滤掉恢复失败的终端
    const finalTabs = allTabs.filter(tab => {
      if (tab.type === 'terminal') {
        // 终端 tab：只保留成功恢复的
        return restoredTabIds.has(tab.id)
      } else {
        // 非终端 tab（如 Settings）：直接保留
        return true
      }
    })

    sessionStore.updateTabs(finalTabs)

    const activeTab = finalTabs.find(t => t.isActive)
    if (activeTab) {
      await getHandler(activeTab.type).activate(activeTab.id)
    } else if (finalTabs.length > 0) {
      await getHandler(finalTabs[0].type).activate(finalTabs[0].id)
    }

    return finalTabs.length > 0
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
    dispatchOutputForPaneId,
    registerResizeCallback,
    unregisterResizeCallback,
    createTerminal,
    createAgentTerminal,
    closeTerminal,
    setActiveTerminal,
    writeToTerminal,
    resizeTerminal,
    createTerminalWithShell,
    initializeShellManager,
    restoreFromSessionState,
    initializeTerminalStore,
    subscribeToCommandEvents,
    emitCommandEvent,
  }
})
