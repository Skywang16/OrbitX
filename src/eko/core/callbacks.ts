/**
 * Eko回调系统实现
 * 提供流式回调和人机交互回调功能
 */

import type { AgentContext, StreamCallbackMessage } from '@eko-ai/eko'
import type { TerminalCallback, TerminalStreamCallback, TerminalHumanCallback } from '../types'

/**
 * 默认的流式回调实现
 * 在控制台打印所有回调消息
 */
export class DefaultStreamCallback implements TerminalStreamCallback {
  async onMessage(message: StreamCallbackMessage): Promise<void> {
    // 根据消息类型进行不同的处理
    switch (message.type) {
      case 'workflow':
        if (message.streamDone) {
          console.log('🔄 [Workflow] 工作流生成完成:')
          console.log(message.workflow?.xml || '无工作流内容')
        } else {
          console.log('🔄 [Workflow] 工作流生成中...')
        }
        break

      case 'text':
        if (message.streamDone) {
          console.log('💬 [Text] AI响应:', message.text)
        } else {
          // 在浏览器环境中使用console.log而不是process.stdout.write
          console.log('💬 [Text] 流式输出:', message.text || '')
        }
        break

      case 'thinking':
        if (message.streamDone) {
          console.log('🤔 [Thinking] 思考完成:', message.text)
        } else {
          console.log('🤔 [Thinking] 思考中...', message.text)
        }
        break

      case 'tool_streaming':
        console.log('🔧 [Tool Streaming]', message.toolName)
        break

      case 'tool_use':
        console.log(`🛠️ [Tool Use] 使用工具: ${message.toolName}`)
        console.log('参数:', JSON.stringify(message.params, null, 2))
        break

      case 'tool_running':
        console.log(`⚙️ [Tool Running] 工具运行中: ${message.toolName}`)
        break

      case 'tool_result':
        console.log(`✅ [Tool Result] 工具执行完成: ${message.toolName}`)
        if (message.toolResult) {
          console.log('结果:', message.toolResult)
        }
        break

      case 'file':
        console.log('📁 [File] 文件操作')
        break

      case 'error':
        console.error('❌ [Error] 错误:', message.error)
        break

      case 'finish':
        console.log('🎉 [Finish] 执行完成')
        break

      case 'agent_result':
        console.log('🤖 [Agent Result] Agent执行结果:')
        console.log('- Agent名称:', message.agentName)
        console.log('- 任务ID:', message.taskId)
        if (message.error) {
          console.error('- 错误:', message.error)
        }
        if (message.result) {
          console.log('- 结果:', message.result)
        }
        if (message.agentNode) {
          console.log('- 节点信息:', message.agentNode)
        }
        break

      default:
        console.log('📝 [Unknown] 未知消息类型:', message.type, message)
    }
  }
}

/**
 * 默认的人机交互回调实现
 * 在控制台提示用户输入
 */
export class DefaultHumanCallback implements TerminalHumanCallback {
  async onHumanConfirm(context: AgentContext, prompt: string): Promise<boolean> {
    console.log('❓ [Human Confirm] 需要用户确认:')
    console.log(prompt)
    console.log('上下文:', context)

    // 在实际应用中，这里应该显示UI对话框让用户确认
    // 现在先默认返回true（确认）
    console.log('⚠️ 自动确认（开发模式）')
    return true
  }

  async onHumanInput(context: AgentContext, prompt: string): Promise<string> {
    console.log('✏️ [Human Input] 需要用户输入:')
    console.log(prompt)
    console.log('上下文:', context)

    // 在实际应用中，这里应该显示输入框让用户输入
    // 现在先返回默认值
    const defaultInput = '用户输入（开发模式默认值）'
    console.log('⚠️ 使用默认输入:', defaultInput)
    return defaultInput
  }

  async onHumanSelect(context: AgentContext, prompt: string, options: string[], multiple?: boolean): Promise<string[]> {
    console.log('📋 [Human Select] 需要用户选择:')
    console.log(prompt)
    console.log('选项:', options)
    console.log('多选:', multiple)
    console.log('上下文:', context)

    // 在实际应用中，这里应该显示选择框让用户选择
    // 现在先返回第一个选项
    const defaultSelection = [options[0]]
    console.log('⚠️ 使用默认选择:', defaultSelection)
    return defaultSelection
  }

  async onHumanHelp(context: AgentContext, helpType: string, prompt: string): Promise<boolean> {
    console.log('🆘 [Human Help] 需要用户帮助:')
    console.log('帮助类型:', helpType)
    console.log('提示:', prompt)
    console.log('上下文:', context)

    // 在实际应用中，这里应该显示帮助信息或UI
    // 返回true表示帮助请求已被处理
    console.log('⚠️ 自动处理帮助请求（开发模式）')
    return true
  }

  // 终端专用回调方法
  async onCommandConfirm(context: AgentContext, command: string): Promise<boolean> {
    console.log('⚠️ [Command Confirm] 需要确认命令执行:')
    console.log('命令:', command)
    console.log('上下文:', context)

    // 检查是否是危险命令
    const dangerousCommands = ['rm -rf', 'sudo rm', 'format', 'del /f', 'shutdown']
    const isDangerous = dangerousCommands.some(dangerous => command.toLowerCase().includes(dangerous))

    if (isDangerous) {
      console.log('🚨 检测到危险命令，建议拒绝执行')
      return false
    }

    console.log('✅ 命令看起来安全，自动确认执行')
    return true
  }

  async onFileSelect(context: AgentContext, prompt: string, directory?: string): Promise<string> {
    console.log('📂 [File Select] 需要选择文件:')
    console.log(prompt)
    console.log('目录:', directory)
    console.log('上下文:', context)

    // 在实际应用中，这里应该显示文件选择对话框
    const defaultFile = directory ? `${directory}/example.txt` : './example.txt'
    console.log('⚠️ 使用默认文件:', defaultFile)
    return defaultFile
  }

  async onPathInput(context: AgentContext, prompt: string, defaultPath?: string): Promise<string> {
    console.log('📍 [Path Input] 需要输入路径:')
    console.log(prompt)
    console.log('默认路径:', defaultPath)
    console.log('上下文:', context)

    // 在实际应用中，这里应该显示路径输入框
    const finalPath = defaultPath || './default-path'
    console.log('⚠️ 使用路径:', finalPath)
    return finalPath
  }
}

/**
 * 组合的默认回调实现
 */
export class DefaultTerminalCallback implements TerminalCallback {
  private streamCallback: DefaultStreamCallback
  private humanCallback: DefaultHumanCallback

  constructor() {
    this.streamCallback = new DefaultStreamCallback()
    this.humanCallback = new DefaultHumanCallback()
  }

  // 流式回调方法
  async onMessage(message: StreamCallbackMessage): Promise<void> {
    return this.streamCallback.onMessage(message)
  }

  // 人机交互回调方法
  async onHumanConfirm(context: AgentContext, prompt: string): Promise<boolean> {
    return this.humanCallback.onHumanConfirm(context, prompt)
  }

  async onHumanInput(context: AgentContext, prompt: string): Promise<string> {
    return this.humanCallback.onHumanInput(context, prompt)
  }

  async onHumanSelect(context: AgentContext, prompt: string, options: string[], multiple?: boolean): Promise<string[]> {
    return this.humanCallback.onHumanSelect(context, prompt, options, multiple)
  }

  async onHumanHelp(context: AgentContext, helpType: string, prompt: string): Promise<boolean> {
    return this.humanCallback.onHumanHelp(context, helpType, prompt)
  }

  // 终端专用回调方法
  async onCommandConfirm(context: AgentContext, command: string): Promise<boolean> {
    return this.humanCallback.onCommandConfirm!(context, command)
  }

  async onFileSelect(context: AgentContext, prompt: string, directory?: string): Promise<string> {
    return this.humanCallback.onFileSelect!(context, prompt, directory)
  }

  async onPathInput(context: AgentContext, prompt: string, defaultPath?: string): Promise<string> {
    return this.humanCallback.onPathInput!(context, prompt, defaultPath)
  }
}

/**
 * 创建默认回调实例
 */
export const createDefaultCallback = (): TerminalCallback => {
  return new DefaultTerminalCallback()
}

/**
 * 创建静默回调（不输出任何信息）
 */
export const createSilentCallback = (): TerminalCallback => {
  return {
    onMessage: async () => {},
    onHumanConfirm: async () => true,
    onHumanInput: async () => '',
    onHumanSelect: async (_, __, options) => [options[0]],
    onHumanHelp: async () => true,
    onCommandConfirm: async () => true,
    onFileSelect: async () => './default-file',
    onPathInput: async (_, __, defaultPath) => defaultPath || './default-path',
  }
}
