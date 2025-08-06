/**
 * Eko框架主入口模块
 * 整合所有模块并提供统一的API接口
 */

import { Eko } from '@eko-ai/eko'

// 导入核心模块
import { getEkoConfig, type EkoConfigOptions } from './core/config'
import { createDefaultCallback, createSilentCallback } from './core/callbacks'

// 导入Agent
import {
  TerminalAgent,
  createTerminalAgent,
  createSafeTerminalAgent,
  createDeveloperTerminalAgent,
} from './agent/terminal-agent'

// 导入工具
import { terminalTools } from './tools/terminal-tools'

// 导入类型
import type { TerminalCallback, TerminalAgentConfig, EkoInstanceConfig, EkoRunOptions, EkoRunResult } from './types'

/**
 * 终端Eko实例类
 * 封装Eko框架，专门为终端模拟器优化
 */
export class TerminalEko {
  private eko: Eko | null = null
  private agent: TerminalAgent
  private callback: TerminalCallback
  private config: EkoInstanceConfig

  constructor(config: EkoInstanceConfig = {}) {
    this.config = {
      debug: false,
      ...config,
    }

    // 创建回调
    this.callback = config.callback || createDefaultCallback()

    // 创建Agent
    this.agent = createTerminalAgent(config.agentConfig)

    if (config.debug) {
      console.log('🚀 TerminalEko 实例已创建')
      console.log('配置:', this.config)
      console.log('Agent状态:', this.agent.getStatus())
    }
  }

  /**
   * 初始化Eko实例
   */
  async initialize(options: EkoConfigOptions = {}): Promise<void> {
    try {
      if (this.config.debug) {
        console.log('🔧 正在初始化Eko实例...')
      }

      // 获取Eko配置
      const ekoConfig = await getEkoConfig({
        debug: this.config.debug,
        ...options,
      })

      // 创建Eko实例
      this.eko = new Eko({
        llms: ekoConfig.llms,
        agents: [this.agent],
        planLlms: ekoConfig.planLlms,
        callback: this.callback,
      })

      if (this.config.debug) {
        console.log('✅ Eko实例初始化完成')
      }
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

      if (this.config.debug) {
        console.log('🎯 开始执行任务:', prompt)
        console.log('选项:', options)
      }

      // 设置终端上下文
      if (options.terminalId) {
        this.agent.setDefaultTerminalId(options.terminalId)
      }

      if (options.workingDirectory) {
        this.agent.setDefaultWorkingDirectory(options.workingDirectory)
      }

      // 执行任务
      const result = await this.eko!.run(prompt)

      const duration = Date.now() - startTime

      if (this.config.debug) {
        console.log('✅ 任务执行完成，耗时:', duration, 'ms')
      }

      return {
        result: result.result,
        duration,
        success: true,
      }
    } catch (error) {
      const duration = Date.now() - startTime
      const errorMessage = error instanceof Error ? error.message : String(error)

      if (this.config.debug) {
        console.error('❌ 任务执行失败:', errorMessage)
      }

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

      if (this.config.debug) {
        console.log('📋 生成工作流:', prompt)
      }

      const workflow = await this.eko!.generate(prompt)

      if (this.config.debug) {
        console.log('✅ 工作流生成完成')
      }

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

      if (this.config.debug) {
        console.log('⚙️ 执行工作流')
        console.log('选项:', options)
      }

      // 设置终端上下文
      if (options.terminalId) {
        this.agent.setDefaultTerminalId(options.terminalId)
      }

      if (options.workingDirectory) {
        this.agent.setDefaultWorkingDirectory(options.workingDirectory)
      }

      // 执行工作流
      const result = await this.eko!.execute(workflow)

      const duration = Date.now() - startTime

      if (this.config.debug) {
        console.log('✅ 工作流执行完成，耗时:', duration, 'ms')
      }

      return {
        result: result.result,
        duration,
        success: true,
      }
    } catch (error) {
      const duration = Date.now() - startTime
      const errorMessage = error instanceof Error ? error.message : String(error)

      if (this.config.debug) {
        console.error('❌ 工作流执行失败:', errorMessage)
      }

      return {
        result: '',
        duration,
        success: false,
        error: errorMessage,
      }
    }
  }

  /**
   * 获取Agent实例
   */
  getAgent(): TerminalAgent {
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
   * 销毁实例
   */
  destroy(): void {
    this.eko = null
    if (this.config.debug) {
      console.log('🗑️ TerminalEko 实例已销毁')
    }
  }
}

/**
 * 创建TerminalEko实例的便捷函数
 */
export async function createTerminalEko(config: EkoInstanceConfig = {}): Promise<TerminalEko> {
  const instance = new TerminalEko(config)
  await instance.initialize()
  return instance
}

/**
 * 创建调试模式的TerminalEko实例
 */
export async function createDebugTerminalEko(config: EkoInstanceConfig = {}): Promise<TerminalEko> {
  return createTerminalEko({
    ...config,
    debug: true,
  })
}

/**
 * 创建静默模式的TerminalEko实例
 */
export async function createSilentTerminalEko(config: EkoInstanceConfig = {}): Promise<TerminalEko> {
  return createTerminalEko({
    ...config,
    callback: createSilentCallback(),
  })
}

// 导出所有类型和工具
export type { TerminalCallback, TerminalAgentConfig, EkoInstanceConfig, EkoRunOptions, EkoRunResult, EkoConfigOptions }

export {
  // 核心类
  TerminalAgent,

  // 工厂函数
  createTerminalAgent,
  createSafeTerminalAgent,
  createDeveloperTerminalAgent,

  // 回调
  createDefaultCallback,
  createSilentCallback,

  // 工具
  terminalTools,

  // 配置
  getEkoConfig,
}
