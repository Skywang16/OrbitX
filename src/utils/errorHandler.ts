import { AIError, AIErrorType } from '@/types'
import { createMessage } from '@/ui'

export const handleError = (error: unknown, defaultMessage = 'Operation failed'): string => {
  if (typeof error === 'string') {
    return error
  }

  if (error instanceof AIError) {
    return getAIErrorMessage(error)
  }

  if (error instanceof Error) {
    return error.message
  }

  return defaultMessage
}

const getAIErrorMessage = (error: AIError): string => {
  switch (error.type) {
    case AIErrorType.AUTHENTICATION_ERROR:
      return 'Authentication failed, please check API key'
    case AIErrorType.NETWORK_ERROR:
      return 'Network connection failed, please check network settings'
    case AIErrorType.RATE_LIMIT_ERROR:
      return 'Too many requests, please try again later'
    case AIErrorType.TIMEOUT_ERROR:
      return 'Request timeout, please try again later'
    case AIErrorType.MODEL_ERROR:
      return 'Model configuration error, please check model settings'
    default:
      return error.message || 'Unknown error'
  }
}

export const handleErrorWithMessage = (error: unknown, defaultMessage = 'Operation failed') => {
  const errorMessage = handleError(error, defaultMessage)
  createMessage.error(errorMessage)
  return errorMessage
}

export const withErrorHandling = <T extends (...args: unknown[]) => Promise<unknown>>(
  fn: T,
  errorMessage?: string
): T => {
  return (async (...args: Parameters<T>) => {
    try {
      return await fn(...args)
    } catch (error) {
      throw new Error(handleError(error, errorMessage))
    }
  }) as T
}
