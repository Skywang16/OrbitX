/**
 * Agent事件渲染器
 *
 * 负责将Agent系统的事件转换为UI可渲染的消息内容
 */

import type { ChatMessage } from '@/types'
import { AgentEventType } from '@/agent/types'
import type {
  TaskAnalyzingEventData,
  TaskPlanningEventData,
  CommandExecutingEventData,
  CommandCompletedEventData,
  CommandFailedEventData,
  TaskRetryingEventData,
  ResultAnalyzingEventData,
} from '@/agent/types'

/**
 * Agent事件渲染配置
 */
export interface AgentEventRenderConfig {
  showIcons: boolean
  showProgress: boolean
  showDetails: boolean
}

/**
 * 渲染结果
 */
export interface RenderResult {
  content: string
  isComplete: boolean
  showSpinner?: boolean
  progressPercent?: number
}

/**
 * Agent事件渲染器类
 */
export class AgentEventRenderer {
  private config: AgentEventRenderConfig

  constructor(config: Partial<AgentEventRenderConfig> = {}) {
    this.config = {
      showIcons: true,
      showProgress: true,
      showDetails: true,
      ...config,
    }
  }

  /**
   * 渲染Agent事件为消息内容
   */
  renderEvent(eventType: string, eventData: Record<string, unknown>): RenderResult {
    switch (eventType) {
      case AgentEventType.TASK_ANALYZING:
        return this.renderTaskAnalyzing(eventData as TaskAnalyzingEventData)

      case AgentEventType.TASK_PLANNING:
        return this.renderTaskPlanning(eventData as TaskPlanningEventData)

      case AgentEventType.COMMAND_EXECUTING:
        return this.renderCommandExecuting(eventData as CommandExecutingEventData)

      case AgentEventType.COMMAND_COMPLETED:
        return this.renderCommandCompleted(eventData as CommandCompletedEventData)

      case AgentEventType.COMMAND_FAILED:
        return this.renderCommandFailed(eventData as CommandFailedEventData)

      case AgentEventType.TASK_RETRYING:
        return this.renderTaskRetrying(eventData as TaskRetryingEventData)

      case AgentEventType.RESULT_ANALYZING:
        return this.renderResultAnalyzing(eventData as ResultAnalyzingEventData)

      case AgentEventType.TASK_COMPLETED:
        return this.renderTaskCompleted(eventData)

      case AgentEventType.ERROR_OCCURRED:
        return this.renderError(eventData)

      default:
        return {
          content: `未知事件类型: ${eventType}`,
          isComplete: true,
        }
    }
  }

  /**
   * 渲染任务分析事件
   */
  private renderTaskAnalyzing(data: TaskAnalyzingEventData): RenderResult {
    const icon = this.config.showIcons ? '🧠 ' : ''
    return {
      content: `${icon}**分析任务**\n\n用户请求: "${data.userInput}"`,
      isComplete: false,
      showSpinner: true,
      progressPercent: 10,
    }
  }

  /**
   * 渲染任务规划事件
   */
  private renderTaskPlanning(data: TaskPlanningEventData): RenderResult {
    const icon = this.config.showIcons ? '📋 ' : ''
    let content = `${icon}**制定执行计划**\n\n${data.plan}`

    if (data.steps && data.steps.length > 0) {
      content += '\n\n**执行步骤:**\n'
      data.steps.forEach((step, index) => {
        content += `${index + 1}. ${step}\n`
      })
    }

    return {
      content,
      isComplete: false,
      progressPercent: 25,
    }
  }

  /**
   * 渲染命令执行事件
   */
  private renderCommandExecuting(data: CommandExecutingEventData): RenderResult {
    const icon = this.config.showIcons ? '⚡ ' : ''
    let content = `${icon}**执行命令**`

    if (data.explanation) {
      content += `\n\n${data.explanation}`
    }

    content += `\n\n\`\`\`bash\n${data.command}\n\`\`\``

    if (data.attemptCount && data.attemptCount > 1) {
      content += `\n\n*第 ${data.attemptCount} 次尝试*`
    }

    return {
      content,
      isComplete: false,
      showSpinner: true,
      progressPercent: 50,
    }
  }

  /**
   * 渲染命令完成事件
   */
  private renderCommandCompleted(data: CommandCompletedEventData): RenderResult {
    const icon = this.config.showIcons ? '✅ ' : ''
    let content = `${icon}**命令执行完成**\n\n`

    content += `\`\`\`bash\n${data.command}\n\`\`\`\n\n`

    if (data.output) {
      content += `**输出结果:**\n\`\`\`\n${data.output}\n\`\`\`\n\n`
    }

    content += `退出码: ${data.exitCode}`

    if (data.duration > 0) {
      content += ` | 耗时: ${data.duration}ms`
    }

    return {
      content,
      isComplete: false,
      progressPercent: 75,
    }
  }

  /**
   * 渲染命令失败事件
   */
  private renderCommandFailed(data: CommandFailedEventData): RenderResult {
    const icon = this.config.showIcons ? '❌ ' : ''
    let content = `${icon}**命令执行失败**\n\n`

    content += `\`\`\`bash\n${data.command}\n\`\`\`\n\n`
    content += `**错误信息:** ${data.error}\n\n`

    if (data.exitCode !== undefined) {
      content += `退出码: ${data.exitCode}\n\n`
    }

    if (data.stderr) {
      content += `**错误输出:**\n\`\`\`\n${data.stderr}\n\`\`\``
    }

    return {
      content,
      isComplete: false,
      progressPercent: 75,
    }
  }

  /**
   * 渲染任务重试事件
   */
  private renderTaskRetrying(data: TaskRetryingEventData): RenderResult {
    const icon = this.config.showIcons ? '🔄 ' : ''
    let content = `${icon}**继续尝试** (第${data.attemptCount}次)`

    if (data.maxAttempts) {
      content += ` / ${data.maxAttempts}`
    }

    content += `\n\n**原因:** ${data.reason}\n\n`
    content += `**下一步尝试:**\n\`\`\`bash\n${data.nextCommand}\n\`\`\``

    return {
      content,
      isComplete: false,
      showSpinner: true,
      progressPercent: 60,
    }
  }

  /**
   * 渲染结果分析事件
   */
  private renderResultAnalyzing(data: ResultAnalyzingEventData): RenderResult {
    const icon = this.config.showIcons ? '🔍 ' : ''
    let content = `${icon}**分析结果**\n\n`

    content += `命令: \`${data.command}\`\n\n`

    if (data.output) {
      content += `**输出内容:**\n\`\`\`\n${data.output.substring(0, 200)}`
      if (data.output.length > 200) {
        content += '...'
      }
      content += '\n```'
    }

    return {
      content,
      isComplete: false,
      showSpinner: true,
      progressPercent: 80,
    }
  }

  /**
   * 渲染任务完成事件
   */
  private renderTaskCompleted(data: Record<string, unknown>): RenderResult {
    const icon = this.config.showIcons ? '🎉 ' : ''
    let content = `${icon}**任务完成**\n\n`

    if (data.result) {
      content += `${data.result}`
    } else {
      content += '任务已成功完成！'
    }

    return {
      content,
      isComplete: true,
      progressPercent: 100,
    }
  }

  /**
   * 渲染错误事件
   */
  private renderError(data: Record<string, unknown>): RenderResult {
    const icon = this.config.showIcons ? '⚠️ ' : ''
    let content = `${icon}**执行出错**\n\n`

    if (data.error) {
      content += `**错误信息:** ${data.error}\n\n`
    }

    if (data.userInput) {
      content += `**原始请求:** ${data.userInput}`
    }

    return {
      content,
      isComplete: true,
      progressPercent: 0,
    }
  }

  /**
   * 更新渲染配置
   */
  updateConfig(config: Partial<AgentEventRenderConfig>): void {
    this.config = { ...this.config, ...config }
  }
}

/**
 * 默认渲染器实例
 */
export const defaultAgentEventRenderer = new AgentEventRenderer()

/**
 * 便捷函数：渲染Agent事件
 */
export function renderAgentEvent(eventType: string, eventData: Record<string, unknown>): RenderResult {
  return defaultAgentEventRenderer.renderEvent(eventType, eventData)
}
