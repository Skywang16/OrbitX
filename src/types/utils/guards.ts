/**
 * 类型守卫函数
 */

import type { PersistedStep, PersistedToolStep } from '../../api/ai/types'

// ===== AI相关类型守卫 =====

export function isToolStep(step: PersistedStep): step is PersistedToolStep {
  return step.type === 'tool_use' || step.type === 'tool_result'
}

// ===== 通用类型守卫 =====

export function isDefined<T>(value: T | undefined | null): value is T {
  return value !== undefined && value !== null
}

export function isString(value: unknown): value is string {
  return typeof value === 'string'
}

export function isNumber(value: unknown): value is number {
  return typeof value === 'number' && !isNaN(value)
}

export function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
