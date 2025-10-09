import type { Terminal } from '@xterm/xterm'
import { useI18n } from 'vue-i18n'

export const useTerminalOutput = () => {
  // Batching state for smoother rendering and fewer JS<->renderer hops
  let queuedBytes: Uint8Array[] = []
  let queuedText: string[] = []
  let scheduled = false
  const utf8Decoder = new TextDecoder('utf-8', { fatal: false })

  const scheduleFlush = (terminal: Terminal | null) => {
    if (scheduled) return
    scheduled = true
    const flush = () => {
      scheduled = false
      try {
        if (!terminal) {
          queuedBytes = []
          queuedText = []
          return
        }

        // Prefer writeUtf8 if available, else decode bytes and merge into text batch
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const writeUtf8 = (terminal as any).writeUtf8 as ((data: Uint8Array) => void) | undefined
        if (queuedBytes.length) {
          if (typeof writeUtf8 === 'function') {
            for (const chunk of queuedBytes) {
              writeUtf8(chunk)
            }
          } else {
            // Fallback: decode bytes and append to text queue
            let decoded = ''
            for (const chunk of queuedBytes) {
              decoded += utf8Decoder.decode(chunk, { stream: true })
            }
            decoded += utf8Decoder.decode()
            if (decoded) queuedText.push(decoded)
          }
          queuedBytes.length = 0
        }

        // Then flush any queued text in one go when possible
        if (queuedText.length) {
          const merged = queuedText.join('')
          queuedText.length = 0
          terminal.write(merged)
        }
      } catch {
        // ignore
      }
    }

    if (typeof requestAnimationFrame === 'function') {
      requestAnimationFrame(flush)
    } else {
      setTimeout(flush, 16)
    }
  }

  const handleOutput = (terminal: Terminal | null, data: string, processOutput?: (data: string) => void) => {
    if (terminal && typeof data === 'string') {
      if (processOutput) {
        processOutput(data)
      }
      queuedText.push(data)
      scheduleFlush(terminal)
    }
  }

  const handleOutputBinary = (terminal: Terminal | null, bytes: Uint8Array) => {
    if (terminal && bytes && bytes.length > 0) {
      queuedBytes.push(bytes)
      scheduleFlush(terminal)
    }
  }

  const handleExit = (terminal: Terminal | null, exitCode: number | null) => {
    if (terminal) {
      const { t } = useI18n()
      const exitCodeText = exitCode ?? t('process.unknown_exit_code')
      const message = `\r\n[${t('process.exited', { code: exitCodeText })}]\r\n`
      queuedText.push(message)
      scheduleFlush(terminal)
    }
  }

  return {
    handleOutput,
    handleOutputBinary,
    handleExit,
  }
}
