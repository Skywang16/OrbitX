/**
 * Simplified error handling system for native backend
 * Replaces complex ai-sdk error handling with streamlined approach
 */

export class LLMError extends Error {
  constructor(
    message: string,
    public readonly category: 'network' | 'auth' | 'rate_limit' | 'model' | 'unknown' = 'unknown',
    public readonly retryable: boolean = false,
    public readonly originalError?: unknown
  ) {
    super(message)
    this.name = 'LLMError'
  }
}

export class ToolError extends Error {
  constructor(
    message: string,
    public readonly toolName: string,
    public readonly toolCallId?: string,
    public readonly originalError?: unknown
  ) {
    super(message)
    this.name = 'ToolError'
  }
}

export class StreamError extends Error {
  constructor(
    message: string,
    public readonly streamId?: string,
    public readonly originalError?: unknown
  ) {
    super(message)
    this.name = 'StreamError'
  }
}

export interface RetryConfig {
  maxRetries: number
  baseDelay: number
  maxDelay: number
  backoffMultiplier: number
  jitterEnabled: boolean
}

export const DEFAULT_RETRY_CONFIG: RetryConfig = {
  maxRetries: 3,
  baseDelay: 1000,
  maxDelay: 30000,
  backoffMultiplier: 2,
  jitterEnabled: true,
}

export class ErrorHandler {
  static handleError(error: string | Error | unknown): LLMError {
    const errorMessage = typeof error === 'string' ? error : error instanceof Error ? error.message : String(error)
    const lowerMessage = errorMessage.toLowerCase()

    // Rate limiting errors - highly retryable with longer delays
    if (
      lowerMessage.includes('rate limit') ||
      lowerMessage.includes('429') ||
      lowerMessage.includes('quota exceeded')
    ) {
      return new LLMError(errorMessage, 'rate_limit', true, error)
    }

    // Authentication errors - not retryable
    if (
      lowerMessage.includes('unauthorized') ||
      lowerMessage.includes('invalid api key') ||
      lowerMessage.includes('authentication failed') ||
      lowerMessage.includes('401') ||
      lowerMessage.includes('403')
    ) {
      return new LLMError(errorMessage, 'auth', false, error)
    }

    // Network errors - retryable with standard backoff
    if (
      lowerMessage.includes('network') ||
      lowerMessage.includes('timeout') ||
      lowerMessage.includes('connection') ||
      lowerMessage.includes('econnrefused') ||
      lowerMessage.includes('enotfound') ||
      lowerMessage.includes('502') ||
      lowerMessage.includes('503') ||
      lowerMessage.includes('504')
    ) {
      return new LLMError(errorMessage, 'network', true, error)
    }

    // Model errors - not retryable on same model, but can try different model
    if (
      lowerMessage.includes('model not found') ||
      lowerMessage.includes('invalid model') ||
      lowerMessage.includes('model unavailable') ||
      lowerMessage.includes('404')
    ) {
      return new LLMError(errorMessage, 'model', false, error)
    }

    // Server errors - retryable
    if (lowerMessage.includes('500') || lowerMessage.includes('internal server error')) {
      return new LLMError(errorMessage, 'network', true, error)
    }

    // Context length errors - not retryable without compression
    if (
      lowerMessage.includes('context length') ||
      lowerMessage.includes('too long') ||
      lowerMessage.includes('maximum context') ||
      lowerMessage.includes('token limit')
    ) {
      return new LLMError(errorMessage, 'model', false, error)
    }

    // Default to retryable unknown error
    return new LLMError(errorMessage, 'unknown', true, error)
  }

  static shouldRetry(error: LLMError, retryCount: number, config: RetryConfig = DEFAULT_RETRY_CONFIG): boolean {
    if (!error.retryable || retryCount >= config.maxRetries) {
      return false
    }

    // Special handling for rate limit errors - allow more retries with longer delays
    if (error.category === 'rate_limit') {
      return retryCount < Math.min(config.maxRetries * 2, 6)
    }

    return true
  }

  static getRetryDelay(retryCount: number, config: RetryConfig = DEFAULT_RETRY_CONFIG, errorCategory?: string): number {
    let delay = config.baseDelay * Math.pow(config.backoffMultiplier, retryCount)

    // Special handling for rate limit errors - use longer delays
    if (errorCategory === 'rate_limit') {
      delay = Math.max(delay, 5000) // Minimum 5 seconds for rate limits
    }

    // Apply maximum delay cap
    delay = Math.min(delay, config.maxDelay)

    // Add jitter to prevent thundering herd
    if (config.jitterEnabled) {
      const jitter = delay * 0.1 * Math.random()
      delay += jitter
    }

    return Math.floor(delay)
  }

  static isRetryableError(error: unknown): boolean {
    if (error instanceof LLMError) {
      return error.retryable
    }
    return false
  }

  static categorizeError(error: unknown): 'network' | 'auth' | 'rate_limit' | 'model' | 'unknown' {
    if (error instanceof LLMError) {
      return error.category
    }
    return this.handleError(error).category
  }

  static getErrorSeverity(error: LLMError): 'low' | 'medium' | 'high' | 'critical' {
    switch (error.category) {
      case 'auth':
        return 'critical' // Requires immediate user intervention
      case 'model':
        return 'high' // May require model switching
      case 'rate_limit':
        return 'medium' // Temporary issue, will resolve with time
      case 'network':
        return 'medium' // Usually temporary
      case 'unknown':
      default:
        return 'low' // May be transient
    }
  }

  static shouldSwitchModel(error: LLMError): boolean {
    // Switch model for model-specific errors or repeated failures
    return error.category === 'model' || error.category === 'auth'
  }

  static formatErrorForUser(error: LLMError): string {
    switch (error.category) {
      case 'auth':
        return 'Authentication failed. Please check your API key configuration.'
      case 'rate_limit':
        return 'Rate limit exceeded. The system will automatically retry with delays.'
      case 'network':
        return 'Network connection failed. Retrying automatically...'
      case 'model':
        return 'Model is unavailable. Trying alternative models...'
      default:
        return `An error occurred: ${error.message}`
    }
  }
}

export class ValidationError extends Error {
  constructor(
    message: string,
    public readonly field?: string,
    public readonly value?: unknown
  ) {
    super(message)
    this.name = 'ValidationError'
  }
}

export function validateLLMRequest(request: any): void {
  if (!request.model) {
    throw new ValidationError('Model is required', 'model', request.model)
  }
  if (!request.messages || !Array.isArray(request.messages)) {
    throw new ValidationError('Messages must be an array', 'messages', request.messages)
  }
  if (request.messages.length === 0) {
    throw new ValidationError('At least one message is required', 'messages', request.messages)
  }
}

export function validateToolCall(toolCall: any): void {
  if (!toolCall.id) {
    throw new ValidationError('Tool call ID is required', 'id', toolCall.id)
  }
  if (!toolCall.name) {
    throw new ValidationError('Tool call name is required', 'name', toolCall.name)
  }
}
