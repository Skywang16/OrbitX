import { Eko } from '@/eko-core'
import { getEkoConfig, getEkoLLMsConfig, type EkoConfigOptions } from './core/config'
import { createSidebarCallback } from './core/callbacks'
import { TerminalAgent, createTerminalAgent, createTerminalChatAgent } from './agent/terminal-agent'
import { CodeAgent, createCodeAgent } from './agent/code-agent'
import { allTools } from './tools'
import type { TerminalCallback, TerminalAgentConfig, EkoInstanceConfig, EkoRunOptions, EkoRunResult } from './types'

export class OrbitXEko {
  private eko: Eko | null = null
  private terminalChatAgent: TerminalAgent
  private terminalAgent: TerminalAgent
  private callback: TerminalCallback
  private config: EkoInstanceConfig
  private mode: 'chat' | 'agent' = 'chat'
  private currentTaskId: string | null = null
  private isRunning: boolean = false
  private selectedModelId: string | null = null

  constructor(config: EkoInstanceConfig = {}) {
    this.config = { ...config }

    // 初始化选中的模型ID
    this.selectedModelId = config.selectedModelId || null

    // 创建回调
    this.callback = config.callback || createSidebarCallback()

    // 创建Chat模式的Agent（只读）
    this.terminalChatAgent = createTerminalChatAgent(config.agentConfig)

    // 创建Agent模式的Agent（全权限）
    this.terminalAgent = createTerminalAgent('agent', config.agentConfig)
  }

  /**
   * 初始化Eko实例
   */
  async initialize(options: EkoConfigOptions = {}): Promise<void> {
    try {
      // 获取Eko配置，传递选中的模型ID
      const ekoConfig = await getEkoConfig({
        ...options,
        selectedModelId: this.selectedModelId,
      })

      // 根据模式选择对应的Agent（单Agent模式）
      const agent =
        this.mode === 'chat'
          ? this.terminalChatAgent // Chat模式：只读Agent
          : this.terminalAgent // Agent模式：全权限Agent

      // 创建Eko实例（单Agent配置）
      this.eko = new Eko({
        llms: ekoConfig.llms,
        agent: agent, // 单Agent配置
        planLlms: ekoConfig.planLlms,
        callback: this.callback,
      })

      // 初始化完成，无需输出额外日志
    } catch (error) {
      console.error('❌ Eko实例初始化失败:', error)
      throw error
    }
  }

  /**
   * 设置选中的模型ID并更新LLM配置
   */
  async setSelectedModelId(modelId: string | null): Promise<void> {
    if (this.selectedModelId !== modelId) {
      this.selectedModelId = modelId
      await this.updateLLMConfig()
    }
  }

  /**
   * 获取当前选中的模型ID
   */
  getSelectedModelId(): string | null {
    return this.selectedModelId
  }

  /**
   * 更新LLM配置（重新创建Eko实例以使用最新的AI模型配置）
   */
  private async updateLLMConfig(): Promise<void> {
    try {
      // 重新获取最新的LLM配置，传递选中的模型ID
      const newLLMsConfig = await getEkoLLMsConfig(this.selectedModelId)

      // 重新创建Eko实例（简单可靠，单Agent配置）
      const agent =
        this.mode === 'chat' ? this.terminalChatAgent : this.terminalAgent

      this.eko = new Eko({
        llms: newLLMsConfig,
        agent: agent,
        planLlms: ['default'],
        callback: this.callback,
      })
    } catch (error) {
      console.error('❌ Failed to update LLM configuration:', error)
      // 不抛出错误，避免影响正常运行
    }
  }

  /**
   * 运行AI任务
   */
  async run(prompt: string, options: EkoRunOptions = {}): Promise<EkoRunResult> {
    const startTime = Date.now()

    try {
      if (!this.eko) {
        await this.initialize()
      } else {
        // 每次运行时都更新LLM配置，确保使用最新的AI模型配置
        await this.updateLLMConfig()
      }

      // 设置运行状态
      this.isRunning = true

      // 设置终端上下文
      if (options.terminalId) {
        this.terminalAgent.setDefaultTerminalId(options.terminalId)
      }

      if (options.workingDirectory) {
        this.terminalAgent.setDefaultWorkingDirectory(options.workingDirectory)
      }

      // Build user request prompt
      const enhancedPrompt = `🎯 **User Request**
${prompt}`

      // 生成唯一的taskId
      const taskId = `task_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`
      this.currentTaskId = taskId

      // 执行任务，使用eko的原生run方法（内部会生成taskId）
      const result = await this.eko!.run(enhancedPrompt, taskId)

      const duration = Date.now() - startTime

      return {
        result: result.result,
        duration,
        success: true,
      }
    } catch (error) {
      const duration = Date.now() - startTime
      const errorMessage = error instanceof Error ? error.message : String(error)

      return {
        result: '',
        duration,
        success: false,
        error: errorMessage,
      }
    } finally {
      this.isRunning = false
      this.currentTaskId = null
    }
  }

  /**
   * 生成任务（不执行）
   */
  async generate(prompt: string): Promise<any> {
    try {
      if (!this.eko) {
        await this.initialize()
      }

      const task = await this.eko!.generate(prompt)

      return task
    } catch (error) {
      console.error('❌ 任务生成失败:', error)
      throw error
    }
  }

  /**
   * 执行已生成的任务
   */
  async execute(task: any, options: EkoRunOptions = {}): Promise<EkoRunResult> {
    const startTime = Date.now()

    try {
      if (!this.eko) {
        await this.initialize()
      }

      // 设置终端上下文
      if (options.terminalId) {
        this.terminalAgent.setDefaultTerminalId(options.terminalId)
      }

      if (options.workingDirectory) {
        this.terminalAgent.setDefaultWorkingDirectory(options.workingDirectory)
      }

      // 执行任务
      const result = await this.eko!.execute(task.taskId)

      const duration = Date.now() - startTime

      return {
        result: result.result,
        duration,
        success: true,
      }
    } catch (error) {
      const duration = Date.now() - startTime
      const errorMessage = error instanceof Error ? error.message : String(error)
      console.error('❌ 任务执行失败:', errorMessage)

      return {
        result: '',
        duration,
        success: false,
        error: errorMessage,
      }
    }
  }

  /**
   * 获取终端Agent实例
   */
  getTerminalAgent(): TerminalAgent {
    return this.terminalAgent
  }



  /**
   * 获取Eko实例
   */
  getEko(): Eko | null {
    return this.eko
  }

  /**
   * 获取配置
   */
  getConfig(): EkoInstanceConfig {
    return { ...this.config }
  }

  /**
   * 更新配置
   */
  updateConfig(updates: Partial<EkoInstanceConfig>): void {
    this.config = { ...this.config, ...updates }

    if (updates.callback) {
      this.callback = updates.callback
    }

    if (updates.agentConfig) {
      this.terminalAgent.updateConfig(updates.agentConfig)
    }
  }



  /**
   * 设置工作模式（chat/agent）并重新初始化Eko实例
   */
  async setMode(mode: 'chat' | 'agent'): Promise<void> {
    if (this.mode === mode) {
      return // 模式未改变，无需重新初始化
    }

    this.mode = mode

    // 重新初始化Eko实例以使用对应模式的Agent
    if (this.eko) {
      await this.initialize()
    }
  }

  /**
   * 获取Agent专属终端ID
   */
  getAgentTerminalId(): number | null {
    // 根据当前模式返回对应Agent的终端ID
    if (this.mode === 'agent') {
      return this.terminalAgent.getAgentTerminalId()
    } else {
      return this.terminalChatAgent.getAgentTerminalId()
    }
  }

  /**
   * 清理资源
   */
  async cleanup(): Promise<void> {
    try {
      // 根据当前模式清理对应Agent的终端资源
      if (this.mode === 'agent') {
        await this.terminalAgent.cleanupAgentTerminal()
      } else {
        await this.terminalChatAgent.cleanupAgentTerminal()
      }
    } catch (error) {
      // 清理失败不影响程序运行
    }
  }

  /**
   * 中断当前正在运行的任务
   */
  abort(): boolean {
    if (this.eko && this.currentTaskId && this.isRunning) {
      const success = this.eko.abortTask(this.currentTaskId)
      if (success) {
        this.isRunning = false
        this.currentTaskId = null
      }
      return success
    }
    return false
  }

  /**
   * 检查是否有任务正在运行
   */
  isTaskRunning(): boolean {
    return this.isRunning
  }

  /**
   * 获取当前任务ID
   */
  getCurrentTaskId(): string | null {
    return this.currentTaskId
  }

  /**
   * 销毁实例
   */
  destroy(): void {
    // 中断任何正在运行的任务
    this.abort()
    this.eko = null
    // 保持静默销毁，避免冗余日志
  }
}

/**
 * 创建OrbitXEko实例
 */
const createOrbitXEko = async (config: EkoInstanceConfig = {}): Promise<OrbitXEko> => {
  const instance = new OrbitXEko(config)
  await instance.initialize()
  return instance
}

/**
 * 创建终端Eko实例（createOrbitXEko的别名）
 */
const createTerminalEko = createOrbitXEko

// 导出所有类型和工具
export type { TerminalCallback, TerminalAgentConfig, EkoInstanceConfig, EkoRunOptions, EkoRunResult, EkoConfigOptions }

// 类型别名
export type TerminalEko = OrbitXEko

export {
  // 核心类
  TerminalAgent,
  CodeAgent,

  // 工厂函数
  createOrbitXEko,
  createTerminalEko,
  createTerminalAgent,
  createCodeAgent,

  // 回调
  createSidebarCallback,

  // 工具
  allTools,

  // 配置
  getEkoConfig,
}
