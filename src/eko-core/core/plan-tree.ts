import Log from '../common/log'
import Context from './context'
import { RetryLanguageModel } from '../llm'
import { LLMRequest, NativeLLMMessage, NativeLLMStreamChunk } from '../types/llm.types'
import { buildTreePlanSystemPrompt, buildTreePlanUserPrompt } from '../prompt/builders/tree-plan-builder'
import { PlannedTaskTree } from '../types/core.types'
import { parseTaskTree } from '../common/xml'

export class TreePlanner {
  private context: Context

  constructor(context: Context) {
    this.context = context
  }

  async planTree(taskPrompt: string, _saveHistory: boolean = false): Promise<PlannedTaskTree> {
    const config = this.context.config
    const rlm = new RetryLanguageModel(config.llms, config.planLlms)

    const messages: NativeLLMMessage[] = [
      { role: 'system', content: await buildTreePlanSystemPrompt(this.context) },
      { role: 'user', content: buildTreePlanUserPrompt(taskPrompt) },
    ]

    const request: LLMRequest = {
      maxTokens: 4096,
      temperature: 0.6,
      messages,
      abortSignal: this.context.controller.signal,
    }

    const result = await rlm.callStream(request)
    const reader = result.stream.getReader()
    let streamText = ''

    try {
      while (true) {
        await this.context.checkAborted(true)
        const { done, value } = await reader.read()
        if (done) break
        const chunk = value as NativeLLMStreamChunk
        if (chunk.type === 'error') {
          Log.error('TreePlanner LLM Error: ', chunk)
          throw new Error('LLM Error: ' + chunk.error)
        }
        if (chunk.type === 'delta' && chunk.content) {
          streamText += chunk.content
        }
      }
    } finally {
      reader.releaseLock()
      if (Log.isEnableInfo()) Log.info('TreePlanner result: \n' + streamText)
    }

    const rootClose = '</root>'
    let xmlText = streamText
    const closeIdx = streamText.lastIndexOf(rootClose)
    if (closeIdx !== -1) {
      xmlText = streamText.substring(0, closeIdx + rootClose.length)
    }

    const tree = parseTaskTree(xmlText)
    if (!tree) {
      throw new Error('Failed to parse planned task tree')
    }

    return tree
  }
}
