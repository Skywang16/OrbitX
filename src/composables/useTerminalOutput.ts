import { ref } from 'vue'
import type { Terminal } from '@xterm/xterm'
import { useI18n } from 'vue-i18n'

export const useTerminalOutput = () => {
  const OUTPUT_FLUSH_INTERVAL = 0
  const MAX_BUFFER_LENGTH = 1024

  let outputBuffer = ''
  const outputFlushTimer = ref<number | null>(null)

  const flushOutputBuffer = (terminal: Terminal | null) => {
    if (outputBuffer.length === 0 || !terminal) return

    try {
      terminal.write(outputBuffer)
      outputBuffer = ''
    } catch {
      outputBuffer = ''
    }

    if (outputFlushTimer.value) {
      clearTimeout(outputFlushTimer.value)
      outputFlushTimer.value = null
    }
  }

  const scheduleOutputFlush = (terminal: Terminal | null) => {
    if (OUTPUT_FLUSH_INTERVAL === 0) {
      flushOutputBuffer(terminal)
      return
    }

    if (outputFlushTimer.value) return

    outputFlushTimer.value = window.setTimeout(() => {
      flushOutputBuffer(terminal)
    }, OUTPUT_FLUSH_INTERVAL)
  }

  const handleOutput = (terminal: Terminal | null, data: string, processOutput?: (data: string) => void) => {
    try {
      if (terminal && typeof data === 'string') {
        if (processOutput) {
          processOutput(data)
        }

        if (OUTPUT_FLUSH_INTERVAL === 0) {
          terminal.write(data)
          return
        }

        outputBuffer += data

        if (outputBuffer.length >= MAX_BUFFER_LENGTH) {
          flushOutputBuffer(terminal)
        } else {
          scheduleOutputFlush(terminal)
        }
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

  const cleanup = () => {
    if (outputFlushTimer.value) {
      clearTimeout(outputFlushTimer.value)
      outputFlushTimer.value = null
    }
    outputBuffer = ''
  }

  return {
    handleOutput,
    handleExit,
    cleanup,
  }
}
