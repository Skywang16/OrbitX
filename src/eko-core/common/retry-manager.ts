/**
 * Advanced retry manager with circuit breaker pattern and intelligent backoff
 * Provides centralized retry logic for all LLM operations
 */

import Log from './log'
import { LLMError, ErrorHandler, RetryConfig, DEFAULT_RETRY_CONFIG } from './error'

export interface RetryAttempt {
  attemptNumber: number
  error: LLMError
  delay: number
  timestamp: number
}

export interface RetryStats {
  totalAttempts: number
  successfulAttempts: number
  failedAttempts: number
  averageDelay: number
  lastAttemptTime: number
  errorCategories: Record<string, number>
}

export class RetryManager {
  private retryConfig: RetryConfig
  private attemptHistory: Map<string, RetryAttempt[]> = new Map()
  private circuitBreakerState: Map<string, { isOpen: boolean; lastFailure: number; failureCount: number }> = new Map()

  constructor(config?: Partial<RetryConfig>) {
    this.retryConfig = { ...DEFAULT_RETRY_CONFIG, ...config }
  }

  /**
   * Execute a function with intelligent retry logic
   */
  async executeWithRetry<T>(
    operation: () => Promise<T>,
    operationId: string,
    context?: { modelName?: string; operationType?: string }
  ): Promise<T> {
    const attempts: RetryAttempt[] = []
    let lastError: LLMError | null = null

    // Check circuit breaker
    if (this.isCircuitOpen(operationId)) {
      throw new LLMError(`Circuit breaker is open for operation: ${operationId}`, 'network', false)
    }

    for (let attemptNumber = 0; attemptNumber <= this.retryConfig.maxRetries; attemptNumber++) {
      try {
        const result = await operation()

        // Success - reset circuit breaker and record success
        this.recordSuccess(operationId)
        this.recordAttemptHistory(operationId, attempts)

        if (attemptNumber > 0) {
          Log.info(`Operation ${operationId} succeeded after ${attemptNumber} retries`)
        }

        return result
      } catch (error) {
        const llmError = error instanceof LLMError ? error : ErrorHandler.handleError(error)
        lastError = llmError

        const delay = ErrorHandler.getRetryDelay(attemptNumber, this.retryConfig, llmError.category)

        attempts.push({
          attemptNumber,
          error: llmError,
          delay,
          timestamp: Date.now(),
        })

        Log.error(`Operation ${operationId} failed (attempt ${attemptNumber + 1}):`, {
          error: llmError.message,
          category: llmError.category,
          retryable: llmError.retryable,
          context,
        })

        // Check if we should continue retrying
        if (!ErrorHandler.shouldRetry(llmError, attemptNumber, this.retryConfig)) {
          Log.info(`Stopping retries for ${operationId} due to non-retryable error: ${llmError.category}`)
          break
        }

        if (attemptNumber < this.retryConfig.maxRetries) {
          Log.info(`Retrying ${operationId} after ${delay}ms... (${attemptNumber + 1}/${this.retryConfig.maxRetries})`)
          await this.sleep(delay)
        }
      }
    }

    // All retries failed - update circuit breaker and record failure
    this.recordFailure(operationId, lastError!)
    this.recordAttemptHistory(operationId, attempts)

    throw new LLMError(
      `Operation ${operationId} failed after ${attempts.length} attempts. Last error: ${lastError?.message}`,
      lastError?.category || 'unknown',
      false,
      { attempts, context }
    )
  }

  /**
   * Check if circuit breaker is open for an operation
   */
  private isCircuitOpen(operationId: string): boolean {
    const state = this.circuitBreakerState.get(operationId)
    if (!state || !state.isOpen) {
      return false
    }

    // Check if circuit should be half-open (allow one test request)
    const timeSinceLastFailure = Date.now() - state.lastFailure
    const cooldownPeriod = Math.min(60000 * Math.pow(2, state.failureCount - 5), 300000) // Max 5 minutes

    if (timeSinceLastFailure > cooldownPeriod) {
      // Move to half-open state
      state.isOpen = false
      Log.info(`Circuit breaker for ${operationId} moved to half-open state`)
      return false
    }

    return true
  }

  /**
   * Record successful operation
   */
  private recordSuccess(operationId: string): void {
    const state = this.circuitBreakerState.get(operationId)
    if (state) {
      // Reset circuit breaker on success
      state.isOpen = false
      state.failureCount = 0
      Log.info(`Circuit breaker for ${operationId} reset after successful operation`)
    }
  }

  /**
   * Record failed operation and update circuit breaker
   */
  private recordFailure(operationId: string, _error: LLMError): void {
    let state = this.circuitBreakerState.get(operationId)
    if (!state) {
      state = { isOpen: false, lastFailure: 0, failureCount: 0 }
      this.circuitBreakerState.set(operationId, state)
    }

    state.failureCount++
    state.lastFailure = Date.now()

    // Open circuit breaker if failure threshold is reached
    if (state.failureCount >= 5 && !state.isOpen) {
      state.isOpen = true
      Log.warn(`Circuit breaker opened for ${operationId} after ${state.failureCount} failures`)
    }
  }

  /**
   * Record attempt history for analysis
   */
  private recordAttemptHistory(operationId: string, attempts: RetryAttempt[]): void {
    // Keep only recent history to prevent memory leaks
    const maxHistorySize = 100
    let history = this.attemptHistory.get(operationId) || []
    history.push(...attempts)

    if (history.length > maxHistorySize) {
      history = history.slice(-maxHistorySize)
    }

    this.attemptHistory.set(operationId, history)
  }

  /**
   * Get retry statistics for monitoring
   */
  public getRetryStats(operationId?: string): Map<string, RetryStats> | RetryStats | null {
    if (operationId) {
      return this.calculateStats(operationId)
    }

    const allStats = new Map<string, RetryStats>()
    const operationIds = Array.from(this.attemptHistory.keys())
    for (const id of operationIds) {
      const stats = this.calculateStats(id)
      if (stats) {
        allStats.set(id, stats)
      }
    }
    return allStats
  }

  /**
   * Calculate statistics for an operation
   */
  private calculateStats(operationId: string): RetryStats | null {
    const attempts = this.attemptHistory.get(operationId)
    if (!attempts || attempts.length === 0) {
      return null
    }

    const totalAttempts = attempts.length
    const failedAttempts = attempts.length
    const successfulAttempts = 0 // Success doesn't create retry attempts
    const averageDelay = attempts.reduce((sum, attempt) => sum + attempt.delay, 0) / totalAttempts
    const lastAttemptTime = Math.max(...attempts.map(a => a.timestamp))

    const errorCategories: Record<string, number> = {}
    attempts.forEach(attempt => {
      const category = attempt.error.category
      errorCategories[category] = (errorCategories[category] || 0) + 1
    })

    return {
      totalAttempts,
      successfulAttempts,
      failedAttempts,
      averageDelay,
      lastAttemptTime,
      errorCategories,
    }
  }

  /**
   * Reset circuit breaker for specific operation or all operations
   */
  public resetCircuitBreaker(operationId?: string): void {
    if (operationId) {
      this.circuitBreakerState.delete(operationId)
      Log.info(`Reset circuit breaker for operation: ${operationId}`)
    } else {
      this.circuitBreakerState.clear()
      Log.info('Reset all circuit breakers')
    }
  }

  /**
   * Clear retry history
   */
  public clearHistory(operationId?: string): void {
    if (operationId) {
      this.attemptHistory.delete(operationId)
    } else {
      this.attemptHistory.clear()
    }
  }

  /**
   * Update retry configuration
   */
  public updateConfig(config: Partial<RetryConfig>): void {
    this.retryConfig = { ...this.retryConfig, ...config }
    Log.info('Updated retry manager configuration:', this.retryConfig)
  }

  /**
   * Get current circuit breaker states
   */
  public getCircuitBreakerStates(): Map<string, { isOpen: boolean; lastFailure: number; failureCount: number }> {
    return new Map(this.circuitBreakerState)
  }

  /**
   * Sleep utility
   */
  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms))
  }
}

// Global retry manager instance
export const globalRetryManager = new RetryManager()
