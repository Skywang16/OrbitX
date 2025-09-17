import type { Terminal } from '@xterm/xterm'
import { useI18n } from 'vue-i18n'

export const useTerminalOutput = () => {
  const handleOutput = (terminal: Terminal | null, data: string, processOutput?: (data: string) => void) => {
    try {
      if (terminal && typeof data === 'string') {
        if (processOutput) {
          processOutput(data)
        }
        terminal.write(data)
      }
    } catch {
      // ignore
    }
  }

  const handleExit = (terminal: Terminal | null, exitCode: number | null) => {
    try {
      if (terminal) {
        const { t } = useI18n()
        const exitCodeText = exitCode ?? t('process.unknown_exit_code')
        const message = `\r\n[${t('process.exited', { code: exitCodeText })}]\r\n`
        terminal.write(message)
      }
    } catch {
      // ignore
    }
  }

  return {
    handleOutput,
    handleExit,
  }
}
