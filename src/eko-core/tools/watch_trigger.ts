import Log from '../common/log'
import { LLMRequest, JSONSchema7, NativeLLMToolCall } from '../types'
import { toImage } from '../common/utils'
import { RetryLanguageModel } from '../llm'
import { AgentContext } from '../core/context'
import type { Agent } from '../agent'
import { extractAgentXmlNode } from '../common/xml'
import { Tool, ToolResult } from '../types/tools.types'

export const TOOL_NAME = 'watch_trigger'

type ImageSource = {
  image: Uint8Array | URL | string
  imageType: 'image/jpeg' | 'image/png'
}

const watch_system_prompt = `You are a tool for detecting system changes. Given a task description, compare two images to determine whether the changes described in the task have occurred.
If the changes have occurred, return an json with \`changed\` set to true and \`changeInfo\` containing a description of the changes. If no changes have occurred, return an object with \`changed\` set to false.

## Example
User: Monitor file changes in directory
### No changes detected
Output:
{
  "changed": false
}
### Change detected
Output:
{
  "changed": true,
  "changeInfo": "New file created in the directory. The file name is: 'backup.sh'"
}`

export default class WatchTriggerTool implements Tool {
  readonly name: string = TOOL_NAME
  readonly description: string
  readonly parameters: JSONSchema7

  constructor() {
    this.description = `When executing the \`watch\` node, please use it to monitor DOM element changes, it will block the listener until the element changes or times out.`
    this.parameters = {
      type: 'object',
      properties: {
        nodeId: {
          type: 'number',
          description: 'watch node ID.',
        },
        watch_area: {
          type: 'array',
          description: 'Element changes in monitoring area, eg: [x, y, width, height].',
          items: {
            type: 'number',
          },
        },
        watch_index: {
          type: 'array',
          description: 'The index of elements to be monitoring multiple elements simultaneously.',
          items: {
            type: 'number',
          },
        },
        frequency: {
          type: 'number',
          description: 'Check frequency, how many seconds between each check, default 1 seconds.',
          default: 1,
          minimum: 0.5,
          maximum: 30,
        },
        timeout: {
          type: 'number',
          description: 'Timeout in minute, default 5 minutes.',
          default: 5,
          minimum: 1,
          maximum: 30,
        },
      },
      required: ['nodeId'],
    }
  }

  async execute(
    args: Record<string, unknown>,
    agentContext: AgentContext,
    _toolCall?: NativeLLMToolCall
  ): Promise<ToolResult> {
    let nodeId = args.nodeId as number
    let agentXml = agentContext.context.task?.xml || ''
    let node = extractAgentXmlNode(agentXml, `[id="${nodeId}"]`)
    if (node == null) {
      throw new Error('Node ID does not exist: ' + nodeId)
    }
    if (node.tagName !== 'watch') {
      throw new Error('Node ID is not a watch node: ' + nodeId)
    }
    let task_description = node.getElementsByTagName('description')[0]?.textContent || ''
    if (!task_description) {
      return {
        content: [
          {
            type: 'text',
            text: 'The watch node does not have a description, skip.',
          },
        ],
      }
    }
    await this.init_eko_observer(agentContext)
    const image1 = await this.get_screenshot(agentContext)
    const start = new Date().getTime()
    const timeout = ((args.timeout as number) || 5) * 60000
    const frequency = Math.max(500, ((args.frequency as number) || 1) * 1000)
    let rlm = new RetryLanguageModel(agentContext.context.config.llms, agentContext.agent.Llms)
    while (new Date().getTime() - start < timeout) {
      await agentContext.context.checkAborted()
      await new Promise(resolve => setTimeout(resolve, frequency))
      let changed = await this.has_eko_changed(agentContext)
      if (changed == 'false') {
        continue
      }
      await this.init_eko_observer(agentContext)
      const image2 = await this.get_screenshot(agentContext)
      const changeResult = await this.is_dom_change(agentContext, rlm, image1, image2, task_description)
      if (changeResult.changed) {
        return {
          content: [
            {
              type: 'text',
              text: changeResult.changeInfo || 'System change detected.',
            },
          ],
        }
      }
    }
    return {
      content: [
        {
          type: 'text',
          text: 'Timeout reached, no system changes detected.',
        },
      ],
    }
  }

  private async get_screenshot(agentContext: AgentContext): Promise<ImageSource> {
    type AgentWithScreenshot = Agent & {
      screenshot?: (ctx: AgentContext) => Promise<{ imageBase64: string; imageType: 'image/jpeg' | 'image/png' }>
    }
    const agent = agentContext.agent as unknown as AgentWithScreenshot
    if (!agent.screenshot) throw new Error('Agent does not support screenshot')
    const imageResult = await agent.screenshot(agentContext)
    const image = toImage(imageResult.imageBase64)
    return {
      image: image,
      imageType: imageResult.imageType,
    }
  }

  private async init_eko_observer(agentContext: AgentContext): Promise<void> {
    try {
      type AgentWithExec = Agent & {
        execute_script?: (ctx: AgentContext, fn: () => void, args: unknown[]) => Promise<unknown>
      }
      const agent = agentContext.agent as unknown as AgentWithExec
      if (!agent.execute_script) throw new Error('Agent does not support execute_script')
      await agent.execute_script(agentContext, () => {
        const _window = window as unknown as {
          has_eko_changed?: boolean
          eko_observer?: MutationObserver
        }
        _window.has_eko_changed = false
        _window.eko_observer && _window.eko_observer.disconnect()
        const eko_observer = new MutationObserver(function () {
          _window.has_eko_changed = true
        })
        eko_observer.observe(document.body, {
          childList: true,
          subtree: true,
          attributes: true,
          attributeOldValue: true,
          characterData: true,
          characterDataOldValue: true,
        })
        _window.eko_observer = eko_observer
      }, [])
    } catch (error) {
      console.error('Error initializing Eko observer:', error)
    }
  }

  private async has_eko_changed(agentContext: AgentContext): Promise<'true' | 'false' | 'undefined'> {
    try {
      type AgentWithExec = Agent & {
        execute_script?: (ctx: AgentContext, fn: () => string, args: unknown[]) => Promise<string>
      }
      const agent = agentContext.agent as unknown as AgentWithExec
      if (!agent.execute_script) throw new Error('Agent does not support execute_script')
      const result = await agent.execute_script(agentContext, () => {
        return (window as unknown as { has_eko_changed?: boolean }).has_eko_changed + ''
      }, [])
      return result as 'true' | 'false' | 'undefined'
    } catch (e) {
      console.error('Error checking Eko change:', e)
      return 'undefined'
    }
  }

  private async is_dom_change(
    agentContext: AgentContext,
    rlm: RetryLanguageModel,
    image1: ImageSource,
    image2: ImageSource,
    task_description: string
  ): Promise<{
    changed: boolean
    changeInfo?: string
  }> {
    try {
      let request: LLMRequest = {
        messages: [
          {
            role: 'system',
            content: watch_system_prompt,
          },
          {
            role: 'user',
            content: [
              {
                type: 'file',
                data: typeof image1.image === 'string' ? image1.image : '',
                mimeType: image1.imageType,
              },
              {
                type: 'file',
                data: typeof image2.image === 'string' ? image2.image : '',
                mimeType: image2.imageType,
              },
              {
                type: 'text',
                text: task_description,
              },
            ],
          },
        ],
        abortSignal: agentContext.context.controller.signal,
      }
      const result = await rlm.call(request)
      let resultText = result.content || '{}'
      resultText = resultText.substring(resultText.indexOf('{'), resultText.lastIndexOf('}') + 1)
      return JSON.parse(resultText)
    } catch (error) {
      Log.error('Error in is_dom_change:', error instanceof Error ? error : String(error))
    }
    return {
      changed: false,
    }
  }
}

export { WatchTriggerTool }
