/**
 * 统一的错误处理工具
 */

import { AIError, AIErrorType } from '@/types'
import { createMessage } from '@/ui'

/**
 * 统一的错误处理函数
 * @param error 错误对象
 * @param defaultMessage 默认错误消息
 * @returns 用户友好的错误消息
 */
export const handleError = (error: unknown, defaultMessage = '操作失败'): string => {
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

/**
 * 获取AI错误的用户友好消息
 */
const getAIErrorMessage = (error: AIError): string => {
  switch (error.type) {
    case AIErrorType.AUTHENTICATION_ERROR:
      return '认证失败，请检查API密钥'
    case AIErrorType.NETWORK_ERROR:
      return '网络连接失败，请检查网络设置'
    case AIErrorType.RATE_LIMIT_ERROR:
      return '请求过于频繁，请稍后再试'
    case AIErrorType.TIMEOUT_ERROR:
      return '请求超时，请稍后再试'
    case AIErrorType.MODEL_ERROR:
      return '模型配置错误，请检查模型设置'
    default:
      return error.message || '未知错误'
  }
}

/**
 * 处理错误并显示消息
 */
export const handleErrorWithMessage = (error: unknown, defaultMessage = '操作失败') => {
  const errorMessage = handleError(error, defaultMessage)
  createMessage.error(errorMessage)
  return errorMessage
}

/**
 * API调用装饰器，统一处理错误
 */
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
