import { reactive } from 'vue'

export interface TerminalInputState {
  currentLine: string
  cursorCol: number
  suggestion: string
}

export interface TerminalEnvironment {
  workingDirectory: string
  cursorPosition: { x: number; y: number }
  isMac: boolean
}

export interface ToastState {
  visible: boolean
  message: string
  type: 'success' | 'error'
}

export const useTerminalState = () => {
  const inputState = reactive<TerminalInputState>({
    currentLine: '',
    cursorCol: 0,
    suggestion: '',
  })

  const terminalEnv = reactive<TerminalEnvironment>({
    workingDirectory: '/tmp',
    cursorPosition: { x: 0, y: 0 },
    isMac: false,
  })

  const toast = reactive<ToastState>({
    visible: false,
    message: '',
    type: 'success',
  })

  const showToast = (message: string, type: 'success' | 'error' = 'success') => {
    toast.visible = true
    toast.message = message
    toast.type = type
    setTimeout(() => {
      toast.visible = false
    }, 3000)
  }

  const closeToast = () => {
    toast.visible = false
  }

  const updateInputLine = (data: string) => {
    if (data === '\r') {
      inputState.currentLine = ''
      inputState.cursorCol = 0
    } else if (data === '\x7f') {
      if (inputState.cursorCol > 0) {
        inputState.currentLine = inputState.currentLine.slice(0, -1)
        inputState.cursorCol--
      }
    } else if (data.length === 1 && data.charCodeAt(0) >= 32) {
      inputState.currentLine += data
      inputState.cursorCol++
    }
  }

  const handleSuggestionChange = (suggestion: string) => {
    inputState.suggestion = suggestion
  }

  return {
    inputState,
    terminalEnv,
    toast,
    showToast,
    closeToast,
    updateInputLine,
    handleSuggestionChange,
  }
}
