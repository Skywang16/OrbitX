/**
 * Eko框架主入口模块
 * 整合所有模块并提供统一的API接口
 */

import { Eko } from '@eko-ai/eko'

// 导入核心模块
import { getEkoConfig, type EkoConfigOptions } from './core/config'
import { createCallback, createSidebarCallback } from './core/callbacks'

// 导入Agent
import { TerminalAgent, createTerminalAgent } from './agent/terminal-agent'
import { CodeAgent, createCodeAgent } from './agent/code-agent'

// 导入工具
import { getAllTools } from './tools'

// 导入类型
import type { TerminalCallback, TerminalAgentConfig, EkoInstanceConfig, EkoRunOptions, EkoRunResult } from './types'

/**
 * OrbitX Eko实例类
 * 封装Eko框架，支持智能Agent选择
 */
export class OrbitXEko {
  private eko: Eko | null = null
  private terminalAgent: TerminalAgent
  private codeAgent: CodeAgent
  private callback: TerminalCallback
  private config: EkoInstanceConfig
  private mode: 'chat' | 'agent' = 'chat'

  constructor(config: EkoInstanceConfig = {}) {
    this.config = { ...config }

    // 创建回调
    this.callback = config.callback || createCallback()

    // 创建两个专门的Agent
    this.terminalAgent = createTerminalAgent(config.agentConfig)
    this.codeAgent = createCodeAgent(config.codeAgentConfig)
  }

  /**
   * 初始化Eko实例
   */
  async initialize(options: EkoConfigOptions = {}): Promise<void> {
    try {
      // 获取Eko配置
      const ekoConfig = await getEkoConfig({
        ...options,
      })

      // 创建Eko实例，传入多个Agent让Eko智能选择
      this.eko = new Eko({
        llms: ekoConfig.llms,
        agents: [this.terminalAgent, this.codeAgent], // Eko会智能选择合适的Agent
        planLlms: ekoConfig.planLlms,
        callback: this.callback,
      })

      // 初始化模式（默认chat）
      this.terminalAgent.setMode(this.mode)
      this.codeAgent.setMode(this.mode)

      // 初始化完成，无需输出额外日志
    } catch (error) {
      console.error('❌ Eko实例初始化失败:', error)
      throw error
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
      }

      // 设置终端上下文
      if (options.terminalId) {
        this.terminalAgent.setDefaultTerminalId(options.terminalId)
      }

      if (options.workingDirectory) {
        this.terminalAgent.setDefaultWorkingDirectory(options.workingDirectory)
        this.codeAgent.updateConfig({ defaultWorkingDirectory: options.workingDirectory })
      }

      // 构建用户请求prompt
      const enhancedPrompt = `🎯 **用户请求**
${prompt}`

      // 执行任务
      const result = await this.eko!.run(enhancedPrompt)

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
   * 生成工作流（不执行）
   */
  async generate(prompt: string): Promise<any> {
    try {
      if (!this.eko) {
        await this.initialize()
      }

      const workflow = await this.eko!.generate(prompt)

      return workflow
    } catch (error) {
      console.error('❌ 工作流生成失败:', error)
      throw error
    }
  }

  /**
   * 执行已生成的工作流
   */
  async execute(workflow: any, options: EkoRunOptions = {}): Promise<EkoRunResult> {
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
        this.codeAgent.updateConfig({ defaultWorkingDirectory: options.workingDirectory })
      }

      // 执行工作流
      const result = await this.eko!.execute(workflow)

      const duration = Date.now() - startTime

      return {
        result: result.result,
        duration,
        success: true,
      }
    } catch (error) {
      const duration = Date.now() - startTime
      const errorMessage = error instanceof Error ? error.message : String(error)
      console.error('❌ 工作流执行失败:', errorMessage)

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
   * 获取代码Agent实例
   */
  getCodeAgent(): CodeAgent {
    return this.codeAgent
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

    if (updates.codeAgentConfig) {
      this.codeAgent.updateConfig(updates.codeAgentConfig)
    }
  }

  /**
   * 设置工作模式（chat/agent）并同步到Agent
   */
  setMode(mode: 'chat' | 'agent'): void {
    this.mode = mode
    this.terminalAgent.setMode(mode)
    this.codeAgent.setMode(mode)
  }

  /**
   * 获取Agent专属终端ID
   */
  getAgentTerminalId(): number | null {
    return this.terminalAgent.getAgentTerminalId()
  }

  /**
   * 清理资源
   */
  async cleanup(): Promise<void> {
    try {
      await this.terminalAgent.cleanupAgentTerminal()
    } catch (error) {
      console.error('清理OrbitXEko资源失败:', error)
    }
  }

  /**
   * 销毁实例
   */
  destroy(): void {
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

// 导出所有类型和工具
export type { TerminalCallback, TerminalAgentConfig, EkoInstanceConfig, EkoRunOptions, EkoRunResult, EkoConfigOptions }

export {
  // 核心类
  TerminalAgent,
  CodeAgent,

  // 工厂函数
  createOrbitXEko,
  createTerminalAgent,
  createCodeAgent,

  // 回调
  createCallback,
  createSidebarCallback,

  // 工具
  getAllTools,

  // 配置
  getEkoConfig,
}
