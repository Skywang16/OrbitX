import type { Terminal } from '@xterm/xterm'
import { shellIntegrationApi } from '@/api'
import { useTerminalStore } from '@/stores/Terminal'

export interface ShellIntegrationOptions {
  terminalId: string
  backendId: number | null
  workingDirectory: string
  onCwdUpdate: (cwd: string) => void
  onTerminalCwdUpdate: (terminalId: string, cwd: string) => void
  onCommandFinished?: (exitCode: number, isSuccess: boolean) => void
  onCommandStarted?: (commandId: string) => void
}

export const useShellIntegration = (options: ShellIntegrationOptions) => {
  const terminalStore = useTerminalStore()

  // Shell Integration状态跟踪
  let currentCommandId: string | null = null
  let isCommandActive: boolean = false

  // 处理命令开始
  const handleCommandStart = () => {
    // 总是创建新的命令ID，因为每个B序列都表示新命令开始
    currentCommandId = `cmd_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`
    isCommandActive = true

    // 调用命令开始回调
    if (options.onCommandStarted && currentCommandId) {
      options.onCommandStarted(currentCommandId)
    }

    // 发布命令开始事件
    terminalStore.emitCommandEvent(options.terminalId, 'started', { commandId: currentCommandId })
  }

  // 处理命令结束
  const handleCommandFinished = (payload: string) => {
    if (currentCommandId && isCommandActive) {
      // 解析退出码，支持多种格式
      let exitCode = 0
      if (payload && payload.trim()) {
        const parsed = parseInt(payload.trim(), 10)
        if (!isNaN(parsed)) {
          exitCode = parsed
        }
      }
      const isSuccess = exitCode === 0

      // 调用命令完成回调
      if (options.onCommandFinished) {
        try {
          options.onCommandFinished(exitCode, isSuccess)
        } catch (error) {
          console.error('Error in onCommandFinished callback:', error)
        }
      }

      // 发布命令完成事件
      terminalStore.emitCommandEvent(options.terminalId, 'finished', {
        commandId: currentCommandId,
        exitCode,
        isSuccess,
      })

      // 重置状态
      currentCommandId = null
      isCommandActive = false
    }
  }

  const parseOSCSequences = (data: string) => {
    // 修复：使用标准的OSC 133序列，修复正则表达式
    // 支持两种格式：\e]133;D\e\\ 和 \e]133;D;0\e\\
    // eslint-disable-next-line no-control-regex
    const oscPattern = /\u001b]133;([A-Za-z])(?:;([^\u0007\u001b]*?))?(?:\u0007|\u001b\\)/g
    let match

    while ((match = oscPattern.exec(data)) !== null) {
      const command = match[1].toUpperCase()
      const payload = match[2] || '' // 如果没有payload，使用空字符串

      switch (command) {
        case 'A':
          // 提示符开始 - 无需处理
          break
        case 'B':
          // 命令开始（提示符结束）
          handleCommandStart()
          break
        case 'C':
          // 命令执行开始 - 无需处理，只是标记
          break
        case 'D':
          // 命令结束，可能包含退出码
          handleCommandFinished(payload)
          break
        case 'P':
          // 属性更新
          handlePropertyUpdate(payload)
          break
      }
    }

    // OSC 7序列用于CWD更新
    // eslint-disable-next-line no-control-regex
    const cwdPattern = /\u001b]7;([^\u0007\u001b]*?)(?:\u0007|\u001b\\)/g
    let cwdMatch

    while ((cwdMatch = cwdPattern.exec(data)) !== null) {
      const fullData = cwdMatch[1]
      let newCwd = ''

      if (fullData) {
        try {
          if (fullData.startsWith('file://')) {
            const url = new URL(fullData)
            newCwd = decodeURIComponent(url.pathname)

            if (
              navigator.userAgent.toLowerCase().includes('win') &&
              newCwd.startsWith('/') &&
              newCwd.length > 3 &&
              newCwd[2] === ':'
            ) {
              newCwd = newCwd.substring(1)
            }
          } else {
            newCwd = decodeURIComponent(fullData)
          }

          if (newCwd && newCwd !== options.workingDirectory) {
            // Only update UI-level state, do not write back to backend
            // Backend is the single source of truth for CWD
            options.onCwdUpdate(newCwd)
            options.onTerminalCwdUpdate(options.terminalId, newCwd)
          }
        } catch (error) {
          console.warn('CWD解析失败:', error, '原始数据:', fullData)
        }
      }
    }
  }

  const handlePropertyUpdate = (payload: string) => {
    try {
      const parts = payload.split('=')
      if (parts.length !== 2) return

      const [key, value] = parts
      switch (key) {
        case 'Cwd': {
          const decodedCwd = decodeURIComponent(value)
          if (decodedCwd && decodedCwd !== options.workingDirectory) {
            // Only update UI-level state, do not write back to backend
            // Backend is the single source of truth for CWD
            options.onCwdUpdate(decodedCwd)
            options.onTerminalCwdUpdate(options.terminalId, decodedCwd)
          }
          break
        }
        case 'OSType':
          break
      }
    } catch {
      // 静默忽略解析错误
    }
  }

  const processTerminalOutput = (data: string) => {
    if (data.includes('\x1b]')) {
      parseOSCSequences(data)
    }
  }

  const initShellIntegration = async (terminal: Terminal | null) => {
    if (!terminal) return

    try {
      await new Promise(resolve => setTimeout(resolve, 500))
      await silentShellIntegration()
    } catch {
      // 静默失败
    }
  }

  const silentShellIntegration = async () => {
    if (options.backendId != null) {
      await shellIntegrationApi.setupShellIntegration(options.backendId, true)
    }
  }

  const resetState = () => {
    currentCommandId = null
    isCommandActive = false
  }

  return {
    processTerminalOutput,
    initShellIntegration,
    resetState,
  }
}
