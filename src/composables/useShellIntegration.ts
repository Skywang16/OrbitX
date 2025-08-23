import { invoke } from '@tauri-apps/api/core'
import type { Terminal } from '@xterm/xterm'

export interface ShellIntegrationOptions {
  terminalId: string
  backendId: number | null
  workingDirectory: string
  onCwdUpdate: (cwd: string) => void
  onTerminalCwdUpdate: (terminalId: string, cwd: string) => void
}

export const useShellIntegration = (options: ShellIntegrationOptions) => {
  const parseOSCSequences = (data: string) => {
    // eslint-disable-next-line
    const oscPattern = /\x1b]633;([A-Za-z]);?([^\x07\x1b]*?)(?:\x07|\x1b\\)/g
    let match

    while ((match = oscPattern.exec(data)) !== null) {
      const command = match[1].toUpperCase()
      const payload = match[2]

      switch (command) {
        case 'A':
          break
        case 'B':
          break
        case 'C':
          break
        case 'D':
          break
        case 'P':
          handlePropertyUpdate(payload)
          break
      }
    }
    // eslint-disable-next-line
    const cwdPattern = /\x1b]7;([^\x07\x1b]*?)(?:\x07|\x1b\\)/g
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
            options.onCwdUpdate(newCwd)
            options.onTerminalCwdUpdate(options.terminalId, newCwd)

            if (options.backendId != null) {
              invoke('update_pane_cwd', {
                paneId: options.backendId,
                cwd: newCwd,
              }).catch(err => {
                console.warn('同步CWD到后端失败:', err)
              })
            }
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
            options.onCwdUpdate(decodedCwd)
            options.onTerminalCwdUpdate(options.terminalId, decodedCwd)
            if (options.backendId != null) {
              invoke('update_pane_cwd', {
                paneId: options.backendId,
                cwd: decodedCwd,
              }).catch(() => {})
            }
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
    try {
      if (options.backendId != null) {
        await invoke('setup_shell_integration', {
          paneId: options.backendId,
          silent: true,
        })
      }
    } catch {
      // 静默失败
    }
  }

  return {
    processTerminalOutput,
    initShellIntegration,
  }
}
