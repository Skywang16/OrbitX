/**
 * 工作区快照提示词组件
 */

import { ComponentConfig, ComponentContext, PromptComponent } from './types'
import { resolveTemplate } from '../template-engine'

/**
 * 从任务描述中提取文件快照信息
 */
export function extractFileSnapshot(taskPrompt: string): string | null {
  if (!taskPrompt || typeof taskPrompt !== 'string') return null

  // 常见的工作区/目录快照模式
  const patterns = [
    /## 当前工作区[\s\S]*?(?=##|$)/,
    /Current working directory:[\s\S]*?(?=\n\n|$)/,
    /Found \d+ files and \d+ directories[\s\S]*?(?=\n\n|$)/,
    /Workspace snapshot[\s\S]*?(?=\n\n|$)/i,
  ]

  for (const pattern of patterns) {
    const match = taskPrompt.match(pattern)
    if (match) {
      return match[0].trim()
    }
  }

  // 退化检测：包含文件数量提示时，回溯到空行或文本结尾
  if (taskPrompt.includes('Found') && taskPrompt.includes('files') && taskPrompt.includes('directories')) {
    const lines = taskPrompt.split('\n')
    const startIdx = lines.findIndex(line => line.includes('Found') && line.includes('files'))
    if (startIdx !== -1) {
      // 取从 Found 开始到下一个空行或结束的内容
      let endIdx = lines.length
      for (let i = startIdx + 1; i < lines.length; i++) {
        if (lines[i].trim() === '') {
          endIdx = i
          break
        }
      }
      return lines.slice(startIdx, endIdx).join('\n').trim()
    }
  }

  return null
}

/**
 * 工作区快照组件
 */
export const workspaceSnapshotComponent: ComponentConfig = {
  id: PromptComponent.WORKSPACE_SNAPSHOT,
  name: 'Workspace Snapshot',
  description: 'Extract workspace snapshot from user task prompt',
  required: false,
  template: `## 当前工作区\n{snapshot}`,
  fn: async (context: ComponentContext) => {
    const taskPrompt = (context.task?.taskPrompt || context.context?.chain.taskPrompt || '').trim()
    if (!taskPrompt) return undefined

    const snapshot = extractFileSnapshot(taskPrompt)
    if (!snapshot) return undefined

    const template = workspaceSnapshotComponent.template!
    return resolveTemplate(template, { snapshot })
  },
}
