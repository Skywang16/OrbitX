export type AgentTerminalExecutionMode = 'blocking' | 'background'

export type AgentTerminalStatus =
  | { type: 'initializing' }
  | { type: 'running' }
  | { type: 'completed'; exitCode?: number | null }
  | { type: 'failed'; error: string }
  | { type: 'aborted' }

export interface AgentTerminal {
  id: string
  command: string
  paneId: number
  mode: AgentTerminalExecutionMode
  status: AgentTerminalStatus
  sessionId: number
  createdAtMs: number
  completedAtMs?: number | null
  label?: string | null
}
