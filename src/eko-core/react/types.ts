import { FinishReason, ToolResult } from '../types'

export type ReactPhase = 'reasoning' | 'action' | 'observation' | 'completion' | 'failed'

export interface ReactThought {
  id: string
  iteration: number
  raw: string
  normalized: string
  createdAt: number
}

export interface ReactAction {
  id: string
  iteration: number
  toolName: string
  arguments: Record<string, unknown>
  issuedAt: number
}

export interface ReactObservation {
  id: string
  iteration: number
  toolName: string
  outcome: ToolResult
  observedAt: number
}

export interface ReactIteration {
  id: string
  index: number
  startedAt: number
  status: ReactPhase
  thought?: ReactThought
  action?: ReactAction
  observation?: ReactObservation
  response?: string
  finishReason?: FinishReason
  errorMessage?: string
}

export interface ReactRuntimeSnapshot {
  iterations: ReactIteration[]
  finalResponse?: string
  stopReason?: FinishReason | 'abort' | 'error'
  aborted?: boolean
}

export interface ReactRuntimeConfig {
  maxIterations: number
  maxConsecutiveErrors: number
  maxIdleRounds: number
}
