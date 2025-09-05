import Log from '../common/log'
import Context from './context'
import { RetryLanguageModel } from '../llm'
import { parseTask } from '../common/xml'
import { LLMRequest, NativeLLMMessage, NativeLLMStreamChunk, NativeLLMMessagePart } from '../types/llm.types'
import { StreamCallback, Task } from '../types/core.types'
import { getPlanSystemPrompt, getPlanUserPrompt } from '../prompt'

export class Planner {
  private taskId: string
  private context: Context
  private callback?: StreamCallback

  constructor(context: Context, callback?: StreamCallback) {
    this.context = context
    this.taskId = context.taskId
    this.callback = callback || context.config.callback
  }

  async plan(taskPrompt: string | NativeLLMMessagePart, saveHistory: boolean = true): Promise<Task> {
    let taskPromptStr
    let userPrompt: NativeLLMMessagePart
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
    const messages: NativeLLMMessage[] = [
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
      const messages: NativeLLMMessage[] = [
        ...chain.planRequest.messages,
        {
          role: 'assistant',
          content: chain.planResult,
        },
        {
          role: 'user',
          content: taskPrompt,
        },
      ]
      return await this.doPlan(taskPrompt, messages, saveHistory)
    } else {
      return this.plan(taskPrompt, saveHistory)
    }
  }

  async doPlan(taskPrompt: string, messages: NativeLLMMessage[], saveHistory: boolean): Promise<Task> {
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
    try {
      while (true) {
        await this.context.checkAborted(true)
        const { done, value } = await reader.read()
        if (done) {
          break
        }
        let chunk = value as NativeLLMStreamChunk
        if (chunk.type == 'error') {
          Log.error('Plan, LLM Error: ', chunk)
          throw new Error('LLM Error: ' + chunk.error)
        }
        if (chunk.type == 'delta') {
          if (chunk.content) {
            streamText += chunk.content
          }
        }
        if (this.callback) {
          let task = parseTask(this.taskId, streamText, false)
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
    let task = parseTask(this.taskId, streamText, true) as Task
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
