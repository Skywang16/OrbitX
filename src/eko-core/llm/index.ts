import Log from '../common/log'
import {
  GenerateResult,
  LLMRequest,
  LLMs,
  StreamResult,
  NativeLLMRequest,
  FinishReason,
  LLMConfig,
} from '../types/llm.types'
import { NativeLLMService } from './native-service'
import { LLMError, ErrorHandler, RetryConfig, DEFAULT_RETRY_CONFIG } from '../common/error'

// Export conversion utilities and native service
export * from './conversion-utils'
export * from './stream-processor'
export * from './stream-buffer'
export * from './performance-monitor'
export { NativeLLMService }
export { LLMError, ErrorHandler, DEFAULT_RETRY_CONFIG } from '../common/error'
export type { RetryConfig } from '../common/error'

/**
 * Completely refactored RetryLanguageModel using native backend
 * Removes all ai-sdk dependencies and implements intelligent retry with model failover
 */
export class RetryLanguageModel {
  private nativeService: NativeLLMService
  private llms: LLMs
  private names: string[]
  private retryConfig: RetryConfig
  private modelFailureCount: Map<string, number> = new Map()
  private lastFailureTime: Map<string, number> = new Map()

  constructor(llms: LLMs, names?: string[], retryConfig?: Partial<RetryConfig>) {
    this.nativeService = new NativeLLMService()
    this.llms = llms
    this.names = names || []
    if (this.names.indexOf('default') === -1) {
      this.names.push('default')
    }
    this.retryConfig = { ...DEFAULT_RETRY_CONFIG, ...retryConfig }
  }

  async call(request: LLMRequest): Promise<GenerateResult> {
    const errors: Array<{ modelName: string; error: LLMError; retryCount: number }> = []
    const availableModels = this.getAvailableModels()

    for (const name of availableModels) {
      const config = this.llms[name]
      if (!config) continue

      // Skip models that have failed recently (circuit breaker pattern)
      if (this.isModelTemporarilyDisabled(name)) {
        Log.info(`Skipping temporarily disabled model: ${name}`)
        continue
      }

      let retryCount = 0
      let lastError: LLMError | null = null

      while (retryCount <= this.retryConfig.maxRetries) {
        try {
          const nativeRequest = this.buildNativeRequest(request, config)
          const response = await this.nativeService.call(nativeRequest)

          // Reset failure count on success
          this.modelFailureCount.delete(name)
          this.lastFailureTime.delete(name)

          return {
            modelId: config.modelId,
            content: response.content,
            finishReason: response.finishReason as FinishReason,
            usage: response.usage || { promptTokens: 0, completionTokens: 0, totalTokens: 0 },
            toolCalls: response.toolCalls,
          }
        } catch (error) {
          const llmError = error instanceof LLMError ? error : ErrorHandler.handleError(error)
          lastError = llmError

          Log.error(`LLM call failed for ${name} (attempt ${retryCount + 1}):`, {
            error: llmError.message,
            category: llmError.category,
            retryable: llmError.retryable,
          })

          // Track model failures
          this.recordModelFailure(name, llmError)

          // Check if we should retry this model
          if (!ErrorHandler.shouldRetry(llmError, retryCount, this.retryConfig)) {
            Log.info(`Not retrying ${name} due to error type: ${llmError.category}`)
            break
          }

          // Check if we should switch models instead of retrying
          if (ErrorHandler.shouldSwitchModel(llmError)) {
            Log.info(`Switching models due to ${llmError.category} error`)
            break
          }

          if (retryCount < this.retryConfig.maxRetries) {
            const delay = ErrorHandler.getRetryDelay(retryCount, this.retryConfig, llmError.category)
            Log.info(`Retrying ${name} after ${delay}ms... (${retryCount + 1}/${this.retryConfig.maxRetries})`)
            await this.sleep(delay)
            retryCount++
            continue
          }

          break // Exit retry loop for this provider
        }
      }

      if (lastError) {
        errors.push({ modelName: name, error: lastError, retryCount })
      }
    }

    // All models failed, throw comprehensive error
    const errorSummary = this.createFailureSummary(errors)
    throw new LLMError(`All LLM providers failed. ${errorSummary}`, 'unknown', false, errors)
  }

  async callStream(request: LLMRequest, optimized: boolean = false): Promise<StreamResult> {
    const errors: Array<{ modelName: string; error: LLMError; retryCount: number }> = []
    const availableModels = this.getAvailableModels()

    for (const name of availableModels) {
      const config = this.llms[name]
      if (!config) continue

      // Skip models that have failed recently
      if (this.isModelTemporarilyDisabled(name)) {
        Log.info(`Skipping temporarily disabled model for streaming: ${name}`)
        continue
      }

      let retryCount = 0
      let lastError: LLMError | null = null

      while (retryCount <= this.retryConfig.maxRetries) {
        try {
          const nativeRequest = this.buildNativeRequest(request, config)
          nativeRequest.stream = true // Ensure stream is enabled

          // Use optimized stream processing if requested
          const streamConfig = optimized
            ? {
                maxBufferSize: 1000,
                backpressureThreshold: 800,
                batchSize: 10,
                flushInterval: 0, // Immediate processing for real-time streaming
                enableMetrics: true,
              }
            : undefined

          const stream = await this.nativeService.callStream(nativeRequest, streamConfig)

          // Reset failure count on success
          this.modelFailureCount.delete(name)
          this.lastFailureTime.delete(name)

          return {
            modelId: config.modelId,
            stream,
          }
        } catch (error) {
          const llmError = error instanceof LLMError ? error : ErrorHandler.handleError(error)
          lastError = llmError

          Log.error(`LLM stream call failed for ${name} (attempt ${retryCount + 1}):`, {
            error: llmError.message,
            category: llmError.category,
            retryable: llmError.retryable,
          })

          // Track model failures
          this.recordModelFailure(name, llmError)

          // Check if we should retry this model
          if (!ErrorHandler.shouldRetry(llmError, retryCount, this.retryConfig)) {
            Log.info(`Not retrying stream ${name} due to error type: ${llmError.category}`)
            break
          }

          // Check if we should switch models instead of retrying
          if (ErrorHandler.shouldSwitchModel(llmError)) {
            Log.info(`Switching models for streaming due to ${llmError.category} error`)
            break
          }

          if (retryCount < this.retryConfig.maxRetries) {
            const delay = ErrorHandler.getRetryDelay(retryCount, this.retryConfig, llmError.category)
            Log.info(`Retrying stream ${name} after ${delay}ms... (${retryCount + 1}/${this.retryConfig.maxRetries})`)
            await this.sleep(delay)
            retryCount++
            continue
          }

          break // Exit retry loop for this provider
        }
      }

      if (lastError) {
        errors.push({ modelName: name, error: lastError, retryCount })
      }
    }

    // All models failed, throw comprehensive error
    const errorSummary = this.createFailureSummary(errors)
    throw new LLMError(`All LLM stream providers failed. ${errorSummary}`, 'unknown', false, errors)
  }

  /**
   * Build native request from LLMRequest
   */
  private buildNativeRequest(request: LLMRequest, config: LLMConfig): NativeLLMRequest {
    return {
      model: config.modelId,
      messages: request.messages,
      temperature: request.temperature ?? config.temperature,
      maxTokens: request.maxTokens ?? config.maxTokens,
      tools: request.tools,
      toolChoice: request.toolChoice,
      stream: false, // Will be overridden for stream calls
      abortSignal: request.abortSignal,
    }
  }

  public get Llms(): LLMs {
    return this.llms
  }

  public get Names(): string[] {
    return this.names
  }

  /**
   * Get available models sorted by priority (least failed first)
   */
  private getAvailableModels(): string[] {
    return [...this.names].sort((a, b) => {
      const aFailures = this.modelFailureCount.get(a) || 0
      const bFailures = this.modelFailureCount.get(b) || 0
      return aFailures - bFailures
    })
  }

  /**
   * Record model failure for circuit breaker pattern
   */
  private recordModelFailure(modelName: string, error: LLMError): void {
    const currentCount = this.modelFailureCount.get(modelName) || 0
    this.modelFailureCount.set(modelName, currentCount + 1)
    this.lastFailureTime.set(modelName, Date.now())

    // Log failure tracking
    Log.info(`Model ${modelName} failure count: ${currentCount + 1}, category: ${error.category}`)
  }

  /**
   * Check if model should be temporarily disabled (circuit breaker)
   */
  private isModelTemporarilyDisabled(modelName: string): boolean {
    const failureCount = this.modelFailureCount.get(modelName) || 0
    const lastFailure = this.lastFailureTime.get(modelName) || 0

    // Disable model if it has failed too many times recently
    if (failureCount >= 5) {
      const timeSinceLastFailure = Date.now() - lastFailure
      const cooldownPeriod = Math.min(60000 * Math.pow(2, failureCount - 5), 300000) // Max 5 minutes

      if (timeSinceLastFailure < cooldownPeriod) {
        return true
      } else {
        // Reset failure count after cooldown
        this.modelFailureCount.delete(modelName)
        this.lastFailureTime.delete(modelName)
      }
    }

    return false
  }

  /**
   * Create comprehensive failure summary for error reporting
   */
  private createFailureSummary(errors: Array<{ modelName: string; error: LLMError; retryCount: number }>): string {
    if (errors.length === 0) {
      return 'No models available.'
    }

    const summary = errors
      .map(({ modelName, error, retryCount }) => {
        const userFriendlyError = ErrorHandler.formatErrorForUser(error)
        return `${modelName}: ${userFriendlyError} (${retryCount + 1} attempts)`
      })
      .join('; ')

    return summary
  }

  /**
   * Get retry statistics for monitoring
   */
  public getRetryStats(): { modelName: string; failureCount: number; lastFailure?: Date }[] {
    return Array.from(this.modelFailureCount.entries()).map(([modelName, failureCount]) => ({
      modelName,
      failureCount,
      lastFailure: this.lastFailureTime.has(modelName) ? new Date(this.lastFailureTime.get(modelName)!) : undefined,
    }))
  }

  /**
   * Reset failure tracking for a specific model or all models
   */
  public resetFailureTracking(modelName?: string): void {
    if (modelName) {
      this.modelFailureCount.delete(modelName)
      this.lastFailureTime.delete(modelName)
      Log.info(`Reset failure tracking for model: ${modelName}`)
    } else {
      this.modelFailureCount.clear()
      this.lastFailureTime.clear()
      Log.info('Reset failure tracking for all models')
    }
  }

  /**
   * Update retry configuration
   */
  public updateRetryConfig(config: Partial<RetryConfig>): void {
    this.retryConfig = { ...this.retryConfig, ...config }
    Log.info('Updated retry configuration:', this.retryConfig)
  }

  /**
   * Sleep utility for retry delays
   */
  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms))
  }
}
