import type { IMcpClient } from '../types/mcp.types'
import type { Tool, ToolSchema, ToolResult, ToolExecuter } from '../types/tools.types'
import { ToolWrapper } from './wrapper'
import { McpTool } from './index'

export interface ToolProvider {
  isAvailable(context: ToolContext): Promise<boolean>
  getTools(context: ToolContext): Promise<Tool[]>
}

export interface ToolContext {
  taskId: string
  nodeId?: string
  agentName: string
  environment: 'browser' | 'windows' | 'mac' | 'linux'
  iteration: number
  abortSignal: AbortSignal
}

/**
 * 统一的工具注册表（核心层）
 * 负责整合静态工具、动态工具提供者与 MCP 客户端工具
 */
export class ToolRegistry {
  private staticTools: Map<string, Tool> = new Map()
  private dynamicToolProviders: ToolProvider[] = []
  private mcpClients: Map<string, IMcpClient> = new Map()

  registerStaticTool(tool: Tool): void {
    this.staticTools.set(tool.name, tool)
  }

  registerStaticTools(tools: Tool[]): void {
    tools.forEach(t => this.registerStaticTool(t))
  }

  registerDynamicToolProvider(provider: ToolProvider): void {
    this.dynamicToolProviders.push(provider)
  }

  registerMcpClient(name: string, client: IMcpClient): void {
    this.mcpClients.set(name, client)
  }

  async getAvailableTools(context: ToolContext): Promise<Tool[]> {
    const tools: Tool[] = []

    // 静态工具
    tools.push(...this.getStaticTools(context))

    // 动态工具
    const dynamic = await this.getDynamicTools(context)
    tools.push(...dynamic)

    // MCP 工具
    const mcp = await this.getMcpTools(context)
    tools.push(...mcp)

    return this.resolveToolConflicts(tools, context)
  }

  private getStaticTools(context: ToolContext): Tool[] {
    return Array.from(this.staticTools.values()).filter(tool => this.isToolAvailable(tool, context))
  }

  private async getDynamicTools(context: ToolContext): Promise<Tool[]> {
    const tools: Tool[] = []
    for (const provider of this.dynamicToolProviders) {
      if (await provider.isAvailable(context)) {
        const providerTools = await provider.getTools(context)
        tools.push(...providerTools)
      }
    }
    return tools
  }

  private async getMcpTools(context: ToolContext): Promise<Tool[]> {
    const tools: Tool[] = []
    for (const [name, client] of this.mcpClients) {
      if (await this.shouldUseMcpClient(name, client, context)) {
        const mcpTools = await this.loadMcpTools(client, context)
        tools.push(...mcpTools)
      }
    }
    return tools
  }

  // 简化版可用性判断与冲突解决策略（后续可替换为更精细逻辑）

  private isToolAvailable(_tool: Tool, _context: ToolContext): boolean {
    return true
  }
  private getToolPriority(_tool: Tool): number {
    // 暂定全部同优先级；后续可根据来源区分
    return 1
  }

  private resolveToolConflicts(tools: Tool[], _context: ToolContext): Tool[] {
    const toolMap = new Map<string, { tool: Tool; priority: number }>()
    for (const tool of tools) {
      const priority = this.getToolPriority(tool)
      const existing = toolMap.get(tool.name)
      if (!existing || priority > existing.priority) {
        toolMap.set(tool.name, { tool, priority })
      }
    }
    return Array.from(toolMap.values()).map(item => item.tool)
  }

  private async shouldUseMcpClient(_name: string, client: IMcpClient, _context: ToolContext): Promise<boolean> {
    try {
      return client.isConnected() || true
    } catch {
      return false
    }
  }

  private async loadMcpTools(client: IMcpClient, context: ToolContext): Promise<Tool[]> {
    try {
      if (!client.isConnected()) {
        await client.connect(context.abortSignal)
      }
      const list = await client.listTools(
        {
          taskId: context.taskId,
          nodeId: context.nodeId,
          environment: context.environment,
          agent_name: context.agentName,
          params: {},
          prompt: '',
        },
        context.abortSignal
      )

      const tools: Tool[] = []
      for (const item of list) {
        const schema: ToolSchema = {
          name: item.name,
          description: item.description,
          inputSchema: item.inputSchema,
        }
        const executer: ToolExecuter = {
          execute: async (args, agentContext, toolCall): Promise<ToolResult> => {
            // mark used to satisfy noUnusedParameters
            void toolCall
            return await client.callTool(
              {
                name: item.name,
                arguments: args,
                extInfo: {
                  taskId: agentContext.context.taskId,
                  nodeId: agentContext.context.currentNodeId || agentContext.context.taskId,
                  environment: context.environment,
                  agent_name: agentContext.agent.Name,
                },
              },
              agentContext.context.controller.signal
            )
          },
        }
        const wrapper = new ToolWrapper(schema, executer)
        tools.push(new McpTool(wrapper))
      }
      return tools
    } catch {
      return []
    }
  }
}
