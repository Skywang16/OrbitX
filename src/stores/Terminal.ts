import { shell as shellAPI } from '@/api/shell'
import { terminal as terminalAPI } from '@/api/terminal'
import type { ShellInfo, ShellManagerState, TerminalSession } from '@/types'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

// 组件可以注册的回调函数类型
interface TerminalEventListeners {
  onOutput: (data: string) => void
  onExit: (exitCode: number | null) => void
}

export const useTerminalStore = defineStore('Terminal', () => {
  // --- 状态 ---
  const terminals = ref<TerminalSession[]>([])
  const activeTerminalId = ref<string | null>(null)

  // Shell管理状态
  const shellManager = ref<ShellManagerState>({
    availableShells: [],
    isLoading: false,
    error: null,
  })

  // 存储组件注册的回调函数的映射表
  const _listeners = ref<Map<string, TerminalEventListeners>>(new Map())

  let _globalListenersUnlisten: UnlistenFn[] = []
  let _isListenerSetup = false

  let nextId = 0

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
    console.log('正在设置全局 Mux 终端监听器...')

    const findTerminalByBackendId = (backendId: number): TerminalSession | undefined => {
      return terminals.value.find(t => t.backendId === backendId)
    }

    // 监听终端输出
    const unlistenOutput = await listen<{ paneId: number; data: string }>('terminal_output', event => {
      try {
        const terminal = findTerminalByBackendId(event.payload.paneId)
        if (terminal) {
          const callbacks = _listeners.value.get(terminal.id)
          callbacks?.onOutput(event.payload.data)
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
          const callbacks = _listeners.value.get(terminal.id)
          callbacks?.onExit(event.payload.exitCode)

          // 自动清理已关闭的终端会话
          closeTerminal(terminal.id)
        }
      } catch (error) {
        console.error('处理终端退出事件时发生错误:', error)
      }
    })

    _globalListenersUnlisten = [unlistenOutput, unlistenExit]
    _isListenerSetup = true
    console.log('全局 Mux 终端监听器已激活。')
  }

  /**
   * 关闭全局监听器。
   */
  const teardownGlobalListeners = () => {
    _globalListenersUnlisten.forEach(unlisten => unlisten())
    _globalListenersUnlisten = []
    _isListenerSetup = false
    console.log('全局 Mux 终端监听器已关闭。')
  }

  /**
   * 由终端组件调用，用于注册其事件处理程序。
   */
  const registerTerminalCallbacks = (id: string, callbacks: TerminalEventListeners) => {
    _listeners.value.set(id, callbacks)
  }

  /**
   * 当终端组件卸载时调用，用于清理资源。
   */
  const unregisterTerminalCallbacks = (id: string) => {
    _listeners.value.delete(id)
  }

  /**
   * 创建一个新的终端会话（使用系统默认shell）。
   */
  const createTerminal = async (initialDirectory?: string): Promise<string> => {
    const id = generateId()

    // 先创建一个临时标题，后面会更新
    const terminal: TerminalSession = {
      id,
      backendId: null,
      title: 'Terminal',
      isActive: false, // 保留以兼容类型定义
    }
    terminals.value.push(terminal)

    try {
      const backendId = await terminalAPI.create({
        rows: 24,
        cols: 80,
        cwd: initialDirectory, // 传入初始目录
      })

      // 获取系统默认shell信息来更新标题
      const defaultShell = await shellAPI.getDefault()

      const t = terminals.value.find(term => term.id === id)
      if (t) {
        t.backendId = backendId
        t.title = defaultShell.name // 使用shell名称作为标题
        t.shellInfo = defaultShell as ShellInfo // 保存shell信息
      }

      setActiveTerminal(id)
      return id
    } catch (error) {
      console.error(`创建终端 '${id}' 失败:`, error)
      const index = terminals.value.findIndex(t => t.id === id)
      if (index !== -1) {
        terminals.value.splice(index, 1)
      }
      throw error
    }
  }

  /**
   * 关闭终端会话。
   */
  const closeTerminal = async (id: string) => {
    const terminal = terminals.value.find(t => t.id === id)
    if (!terminal) return

    unregisterTerminalCallbacks(id)

    if (terminal.backendId !== null) {
      try {
        await terminalAPI.close(terminal.backendId)
      } catch (error) {
        console.error(`关闭终端 '${id}' 的后端失败:`, error)
      }
    }

    const index = terminals.value.findIndex(t => t.id === id)
    if (index !== -1) {
      terminals.value.splice(index, 1)
    }

    if (activeTerminalId.value === id) {
      if (terminals.value.length > 0) {
        setActiveTerminal(terminals.value[0].id)
      } else {
        activeTerminalId.value = null
        await createTerminal()
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

    const terminal: TerminalSession = {
      id,
      backendId: null,
      title,
      isActive: false,
      shellInfo,
    }
    terminals.value.push(terminal)

    try {
      const backendId = await terminalAPI.createWithShell({
        shellName,
        rows: 24,
        cols: 80,
      })

      const t = terminals.value.find(term => term.id === id)
      if (t) {
        t.backendId = backendId
      }

      setActiveTerminal(id)
      return id
    } catch (error) {
      console.error(`创建终端 '${id}' 失败:`, error)
      const index = terminals.value.findIndex(t => t.id === id)
      if (index !== -1) {
        terminals.value.splice(index, 1)
      }
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
    createTerminal,
    closeTerminal,
    setActiveTerminal,
    writeToTerminal,
    resizeTerminal,

    // Shell管理方法
    loadAvailableShells,
    createTerminalWithShell,
    validateShellPath,
    initializeShellManager,
  }
})
