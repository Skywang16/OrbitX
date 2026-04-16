export type SubagentStatus = 'pending' | 'running' | 'completed' | 'cancelled' | 'error'

export interface SubagentRecord {
  id: string
  parentThreadId: number
  childThreadId: number
  parentMessageId: number
  name: string
  profile: string
  taskTitle: string
  status: SubagentStatus
  latestActivity?: string | null
  finalSummary?: string | null
  errorMessage?: string | null
  createdAt: string
  updatedAt: string
  finishedAt?: string | null
}
