/**
 * Agent注册表
 *
 * 负责管理和提供不同类型的Agent实例
 */

import type { IAgent } from './BaseAgent'
import { NewToolAgent } from './NewToolAgent'

export class AgentRegistry {
  private agents: Map<string, IAgent> = new Map()

  constructor() {
    // 在构造时可以进行默认的Agent注册
    this.registerDefaultAgents()
  }

  /**
   * 注册一个Agent实例
   * @param type - Agent的类型，例如 "Tool", "Chat", "WebSearch"
   * @param agent - 遵循IAgent接口的Agent实例
   */
  register(type: string, agent: IAgent): void {
    if (this.agents.has(type)) {
      console.warn(`Agent类型 "${type}" 已被注册，将会被覆盖。`)
    }
    this.agents.set(type, agent)
  }

  /**
   * 根据类型获取一个Agent实例
   * @param type - Agent的类型
   * @returns 返回对应的Agent实例，如果找不到则返回undefined
   */
  getAgent(type: string): IAgent | undefined {
    return this.agents.get(type)
  }

  /**
   * 注册所有内置的、默认的Agent
   */
  private registerDefaultAgents(): void {
    // 注册混合工具Agent
    this.register('Tool', new NewToolAgent())
    this.register('HybridTool', new NewToolAgent())
  }
}

// 创建并导出一个默认的AgentRegistry实例，供整个应用使用
export const agentRegistry = new AgentRegistry()
