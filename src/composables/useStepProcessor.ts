import type { UiStep } from '@/api/agent/types'

const cloneStep = (step: UiStep): UiStep => {
  return {
    ...step,
    metadata: step.metadata ? { ...step.metadata } : undefined,
  }
}

/**
 * 合并工具步骤
 */
export const mergeToolSteps = (steps: UiStep[]): UiStep[] => {
  const result: UiStep[] = []

  for (const step of steps) {
    if (step.stepType === 'tool_use') {
      result.push(cloneStep(step))
    } else if (step.stepType === 'tool_result') {
      const lastStep = result[result.length - 1]
      if (lastStep && lastStep.stepType === 'tool_use') {
        lastStep.stepType = 'tool_result'
        lastStep.content = step.content
        lastStep.timestamp = step.timestamp
        lastStep.metadata = {
          ...lastStep.metadata,
          result: step.metadata?.result,
          isError: step.metadata?.isError,
          extInfo: step.metadata?.extInfo,
        }
      } else {
        result.push(cloneStep(step))
      }
    } else {
      result.push(cloneStep(step))
    }
  }

  return result
}

/**
 * 批量处理步骤
 *
 * 只合并工具步骤，流式步骤已由后端合并
 */
export const mergeBatchSteps = (steps: UiStep[]): UiStep[] => {
  return mergeToolSteps(steps)
}

/**
 * 检查是否为流结束标记
 */
export const isStreamEndMarker = (step: { content: string; metadata?: { streamDone?: boolean } }): boolean => {
  return step.content === '' && step.metadata?.streamDone === true
}

/**
 * 步骤处理器
 */
export const useStepProcessor = () => {
  const processSteps = (steps: UiStep[]): UiStep[] => {
    if (!Array.isArray(steps) || steps.length === 0) {
      return []
    }
    const merged = mergeBatchSteps(steps)
    return merged.filter(step => !isStreamEndMarker(step))
  }

  return { processSteps }
}
