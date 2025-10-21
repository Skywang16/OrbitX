/**
 * 类型守卫函数
 */

import type { UiStep } from '../../api/agent/types'

// ===== AI相关类型守卫 =====

export const isToolStep = (step: UiStep): boolean => {
  return step.stepType === 'tool_use' || step.stepType === 'tool_result'
}
