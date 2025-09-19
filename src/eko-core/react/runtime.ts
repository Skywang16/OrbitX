import { uuidv4 } from '../common/utils'
import { FinishReason } from '../types'
import {
  ReactAction,
  ReactIteration,
  ReactObservation,
  ReactRuntimeConfig,
  ReactRuntimeSnapshot,
  ReactThought,
} from './types'

export class ReactRuntime {
  private readonly config: ReactRuntimeConfig
  private readonly iterations: ReactIteration[] = []
  private consecutiveErrors = 0
  private idleRounds = 0
  private finalResponse?: string
  private stopReason?: FinishReason | 'abort' | 'error'

  constructor(config: ReactRuntimeConfig) {
    this.config = config
  }

  startIteration(): ReactIteration {
    const iteration: ReactIteration = {
      id: uuidv4(),
      index: this.iterations.length,
      startedAt: Date.now(),
      status: 'reasoning',
    }
    this.iterations.push(iteration)
    return iteration
  }

  recordThought(iteration: ReactIteration, raw: string, normalized: string): ReactThought {
    const thought: ReactThought = {
      id: uuidv4(),
      iteration: iteration.index,
      raw,
      normalized,
      createdAt: Date.now(),
    }
    iteration.thought = thought
    iteration.status = 'reasoning'
    return thought
  }

  recordAction(iteration: ReactIteration, toolName: string, args: Record<string, unknown>): ReactAction {
    const action: ReactAction = {
      id: uuidv4(),
      iteration: iteration.index,
      toolName,
      arguments: args,
      issuedAt: Date.now(),
    }
    iteration.action = action
    iteration.status = 'action'
    this.idleRounds = 0
    return action
  }

  recordObservation(
    iteration: ReactIteration,
    toolName: string,
    outcome: ReactObservation['outcome']
  ): ReactObservation {
    const observation: ReactObservation = {
      id: uuidv4(),
      iteration: iteration.index,
      toolName,
      outcome,
      observedAt: Date.now(),
    }
    iteration.observation = observation
    iteration.status = 'observation'
    return observation
  }

  completeIteration(iteration: ReactIteration, response: string | undefined, finishReason?: FinishReason) {
    iteration.response = response
    iteration.finishReason = finishReason
    iteration.status = 'completion'
    this.finalResponse = response
    if (finishReason) {
      this.stopReason = finishReason
    }
    this.idleRounds = 0
    this.consecutiveErrors = 0
  }

  failIteration(iteration: ReactIteration, errorMessage: string) {
    iteration.errorMessage = errorMessage
    iteration.status = 'failed'
    this.consecutiveErrors += 1
  }

  markIdleRound() {
    this.idleRounds += 1
  }

  resetErrorCounter() {
    this.consecutiveErrors = 0
  }

  setStopReason(reason: FinishReason | 'abort' | 'error') {
    this.stopReason = reason
  }

  registerFinalResponse(response: string) {
    this.finalResponse = response
  }

  getIterations(): readonly ReactIteration[] {
    return this.iterations
  }

  getSnapshot(): ReactRuntimeSnapshot {
    return {
      iterations: [...this.iterations],
      finalResponse: this.finalResponse,
      stopReason: this.stopReason,
    }
  }

  shouldHalt(): boolean {
    if (this.iterations.length >= this.config.maxIterations) {
      return true
    }
    if (this.consecutiveErrors >= this.config.maxConsecutiveErrors) {
      return true
    }
    if (this.idleRounds >= this.config.maxIdleRounds) {
      return true
    }
    return false
  }
}

export default ReactRuntime
