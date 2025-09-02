import Log from '../common/log'
import Context from './context'
import { RetryLanguageModel } from '../llm'
import { parseTask } from '../common/xml'
import { LLMRequest } from '../types/llm.types'
import { StreamCallback, Task } from '../types/core.types'
import { getPlanSystemPrompt, getPlanUserPrompt } from '../prompt'
import { LanguageModelV2Prompt, LanguageModelV2StreamPart, LanguageModelV2TextPart } from '@ai-sdk/provider'

export class Planner {
  private taskId: string
  private context: Context
  private callback?: StreamCallback

  constructor(context: Context, callback?: StreamCallback) {
    this.context = context
    this.taskId = context.taskId
    this.callback = callback || context.config.callback
  }

  async plan(taskPrompt: string | LanguageModelV2TextPart, saveHistory: boolean = true): Promise<Task> {
    let taskPromptStr
    let userPrompt: LanguageModelV2TextPart
    if (typeof taskPrompt === 'string') {
      taskPromptStr = taskPrompt
      userPrompt = {
        type: 'text',
        text: getPlanUserPrompt(taskPrompt),
      }
    } else {
      userPrompt = taskPrompt
      taskPromptStr = taskPrompt.text || ''
    }
    const messages: LanguageModelV2Prompt = [
      {
        role: 'system',
        content: await getPlanSystemPrompt(this.context),
      },
      {
        role: 'user',
        content: [userPrompt],
      },
    ]
    return await this.doPlan(taskPromptStr, messages, saveHistory)
  }

  async replan(taskPrompt: string, saveHistory: boolean = true): Promise<Task> {
    const chain = this.context.chain
    if (chain.planRequest && chain.planResult) {
      const messages: LanguageModelV2Prompt = [
        ...chain.planRequest.messages,
        {
          role: 'assistant',
          content: [{ type: 'text', text: chain.planResult }],
        },
        {
          role: 'user',
          content: [{ type: 'text', text: taskPrompt }],
        },
      ]
      return await this.doPlan(taskPrompt, messages, saveHistory)
    } else {
      return this.plan(taskPrompt, saveHistory)
    }
  }

  async doPlan(taskPrompt: string, messages: LanguageModelV2Prompt, saveHistory: boolean): Promise<Task> {
    const config = this.context.config
    const rlm = new RetryLanguageModel(config.llms, config.planLlms)
    const request: LLMRequest = {
      maxTokens: 4096,
      temperature: 0.7,
      messages: messages,
      abortSignal: this.context.controller.signal,
    }
    const result = await rlm.callStream(request)
    const reader = result.stream.getReader()
    let streamText = ''
    let thinkingText = ''
    try {
      while (true) {
        await this.context.checkAborted(true)
        const { done, value } = await reader.read()
        if (done) {
          break
        }
        let chunk = value as LanguageModelV2StreamPart
        if (chunk.type == 'error') {
          Log.error('Plan, LLM Error: ', chunk)
          throw new Error('LLM Error: ' + chunk.error)
        }
        if (chunk.type == 'reasoning-delta') {
          thinkingText += chunk.delta || ''
        }
        if (chunk.type == 'text-delta') {
          streamText += chunk.delta || ''
        }
        if (this.callback) {
          let task = parseTask(this.taskId, streamText, false, thinkingText)
          if (task) {
            await this.callback.onMessage({
              taskId: this.taskId,
              agentName: 'Planer',
              type: 'task',
              streamDone: false,
              task: task as Task,
            })
          }
        }
      }
    } finally {
      reader.releaseLock()
      if (Log.isEnableInfo()) {
        Log.info('Planner result: \n' + streamText)
      }
    }
    if (saveHistory) {
      const chain = this.context.chain
      chain.planRequest = request
      chain.planResult = streamText
    }
    let task = parseTask(this.taskId, streamText, true, thinkingText) as Task
    if (this.callback) {
      await this.callback.onMessage({
        taskId: this.taskId,
        agentName: 'Planer',
        type: 'task',
        streamDone: true,
        task: task,
      })
    }
    if (task.taskPrompt) {
      task.taskPrompt += '\n' + taskPrompt.trim()
    } else {
      task.taskPrompt = taskPrompt.trim()
    }
    return task
  }
}
