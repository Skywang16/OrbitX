import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { agentTerminalApi } from '@/api'
import type { AgentTerminal } from '@/types'
import { useTerminalStore } from '@/stores/Terminal'

export const useAgentTerminalStore = defineStore('agentTerminal', () => {
  const terminals = ref<AgentTerminal[]>([])
  const isListening = ref(false)

  const terminalStore = useTerminalStore()

  const upsertTerminal = (terminal: AgentTerminal) => {
    const index = terminals.value.findIndex(item => item.id === terminal.id)
    if (index >= 0) {
      terminals.value[index] = terminal
    } else {
      terminals.value.unshift(terminal)
    }
    registerRuntimeTerminalIfNeeded(terminal)
  }

  const registerRuntimeTerminalIfNeeded = (terminal: AgentTerminal) => {
    const exists = terminalStore.terminals.some(t => t.id === terminal.paneId)
    if (!exists) {
      terminalStore.registerRuntimeTerminal({
        id: terminal.paneId,
        cwd: '~',
        shell: 'agent',
      })
    }
  }

  const setupListeners = async () => {
    if (isListening.value) return
    isListening.value = true

    await listen<AgentTerminal>('agent_terminal_created', event => {
      upsertTerminal(event.payload)
    })

    await listen<AgentTerminal>('agent_terminal_updated', event => {
      upsertTerminal(event.payload)
    })

    await listen<AgentTerminal>('agent_terminal_completed', event => {
      upsertTerminal(event.payload)
    })

    await listen<{ terminalId: string }>('agent_terminal_removed', event => {
      terminals.value = terminals.value.filter(terminal => terminal.id !== event.payload.terminalId)
    })
  }

  const loadTerminals = async (sessionId?: number) => {
    const list = await agentTerminalApi.list(sessionId)
    terminals.value = list
    list.forEach(terminal => {
      registerRuntimeTerminalIfNeeded(terminal)
    })
  }

  const listForSession = (sessionId?: number): AgentTerminal[] => {
    if (typeof sessionId !== 'number') return []
    return terminals.value.filter(terminal => terminal.sessionId === sessionId)
  }

  const runningCountForSession = (sessionId?: number): number => {
    return listForSession(sessionId).filter(t => t.status.type === 'running').length
  }

  const latestRunning = (sessionId?: number): AgentTerminal | null => {
    return listForSession(sessionId).find(t => t.status.type === 'running') ?? null
  }

  const hasRunning = computed(() => runningCountForSession() > 0)

  const stopTerminal = async (terminalId: string) => {
    await agentTerminalApi.abort(terminalId)
  }

  const removeTerminal = async (terminalId: string) => {
    await agentTerminalApi.remove(terminalId)
    terminals.value = terminals.value.filter(terminal => terminal.id !== terminalId)
  }

  return {
    terminals,
    hasRunning,
    setupListeners,
    loadTerminals,
    listForSession,
    runningCountForSession,
    latestRunning,
    stopTerminal,
    removeTerminal,
  }
})
