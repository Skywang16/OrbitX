import type { Terminal } from '@xterm/xterm'
import { shellIntegrationApi } from '@/api'
import { terminalApi } from '@/api/terminal'
import { useTerminalStore } from '@/stores/Terminal'

export interface ShellIntegrationOptions {
  terminalId: number
  workingDirectory: string
  onCwdUpdate: (cwd: string) => void
  onCommandFinished?: (exitCode: number, isSuccess: boolean) => void
  onCommandStarted?: (commandId: string) => void
}

export const useShellIntegration = (options: ShellIntegrationOptions) => {
  const terminalStore = useTerminalStore()

  let currentCommandId: string | null = null
  let isCommandActive: boolean = false
  let disposed = false
  let paneId = options.terminalId

  const handleCommandStart = () => {
    currentCommandId = `cmd_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`
    isCommandActive = true

    if (options.onCommandStarted && currentCommandId) {
      options.onCommandStarted(currentCommandId)
    }

    terminalStore.emitCommandEvent(paneId, 'started', { commandId: currentCommandId })
  }

  const handleCommandFinished = (payload: string) => {
    if (currentCommandId && isCommandActive) {
      let exitCode = 0
      if (payload && payload.trim()) {
        const parsed = parseInt(payload.trim(), 10)
        if (!isNaN(parsed)) {
          exitCode = parsed
        }
      }
      const isSuccess = exitCode === 0

      if (options.onCommandFinished) {
        try {
          options.onCommandFinished(exitCode, isSuccess)
        } catch (error) {
          console.error('Error in onCommandFinished callback:', error)
        }
      }

      terminalStore.emitCommandEvent(paneId, 'finished', {
        commandId: currentCommandId,
        exitCode,
        isSuccess,
      })

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
          break
        case 'B':
          handleCommandStart()
          break
        case 'C':
          break
        case 'D':
          handleCommandFinished(payload)
          break
        case 'P':
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
          }
          break
        }
        case 'OSType':
          break
      }
    } catch (error) {
      console.warn('Shell integration processing failed:', error)
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
      if (disposed) return
      await silentShellIntegration()
    } catch (error) {
      console.warn('Retry shell integration failed:', error)
    }
  }

  const silentShellIntegration = async () => {
    if (paneId != null) {
      // 在调用前确认面板仍然存在，避免快速删除后的竞态导致 500 错误
      try {
        const exists = await terminalApi.terminalExists(paneId)
        if (!exists || disposed) return
        await shellIntegrationApi.setupShellIntegration(paneId, true)
      } catch (e) {
        console.error('Silent shell integration failed:', e)
      }
    }
  }

  const resetState = () => {
    currentCommandId = null
    isCommandActive = false
  }

  const dispose = () => {
    disposed = true
  }

  const updateTerminalId = (newPaneId: number) => {
    paneId = newPaneId
  }

  return {
    processTerminalOutput,
    initShellIntegration,
    resetState,
    dispose,
    updateTerminalId,
  }
}
