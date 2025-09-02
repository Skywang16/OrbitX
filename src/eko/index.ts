import { Eko } from '@/eko-core'
import { getEkoConfig, getEkoLLMsConfig, type EkoConfigOptions } from './core/config'
import { createSidebarCallback } from './core/callbacks'
import { TerminalAgent, createTerminalAgent } from './agent/terminal-agent'
import { allTools } from './tools'
import type { TerminalCallback, TerminalAgentConfig, EkoInstanceConfig, EkoRunOptions, EkoRunResult } from './types'

export class OrbitXEko {
  private eko: Eko | null = null
  private agent: TerminalAgent
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

    // 创建单一Agent实例，通过模式控制行为
    this.agent = createTerminalAgent('chat', config.agentConfig)
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

      // 更新Agent模式
      this.agent.setMode(this.mode)

      // 创建Eko实例
      this.eko = new Eko({
        llms: ekoConfig.llms,
        agent: this.agent,
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

      // 更新Agent模式
      this.agent.setMode(this.mode)

      // 重新创建Eko实例
      this.eko = new Eko({
        llms: newLLMsConfig,
        agent: this.agent,
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

      // 设置终端上下文并自动继承工作目录
      if (options.terminalId) {
        this.agent.setDefaultTerminalId(options.terminalId)

        // Agent自动继承终端工作目录
        await this.agent.getWorkingDirectoryFromTerminal(options.terminalId)
      }

      // 生成唯一的taskId
      const taskId = `task_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`
      this.currentTaskId = taskId

      // 执行任务，直接使用用户原始prompt
      const result = await this.eko!.run(prompt, taskId)

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

      // 设置终端上下文并自动继承工作目录
      if (options.terminalId) {
        this.agent.setDefaultTerminalId(options.terminalId)

        // Agent自动继承终端工作目录
        await this.agent.getWorkingDirectoryFromTerminal(options.terminalId)
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
    return this.agent
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
      this.agent.updateConfig(updates.agentConfig)
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

    // 重新初始化Eko实例
    if (this.eko) {
      await this.initialize()
    }
  }

  /**
   * 获取Agent专属终端ID
   */
  getAgentTerminalId(): number | null {
    return this.agent.getAgentTerminalId()
  }

  /**
   * 清理资源
   */
  async cleanup(): Promise<void> {
    try {
      await this.agent.cleanupAgentTerminal()
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

  // 工厂函数
  createOrbitXEko,
  createTerminalEko,
  createTerminalAgent,

  // 回调
  createSidebarCallback,

  // 工具
  allTools,

  // 配置
  getEkoConfig,
}
