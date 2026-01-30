import { invoke } from '@/utils/request'
import type { AgentTerminal } from '@/types'

export class AgentTerminalApi {
  list = async (sessionId?: number): Promise<AgentTerminal[]> => {
    return await invoke<AgentTerminal[]>('agent_terminal_list', {
      sessionId,
    })
  }

  abort = async (terminalId: string): Promise<void> => {
    await invoke<void>('agent_terminal_abort', { terminalId })
  }

  remove = async (terminalId: string): Promise<void> => {
    await invoke<void>('agent_terminal_remove', { terminalId })
  }
}

export const agentTerminalApi = new AgentTerminalApi()
export default agentTerminalApi
