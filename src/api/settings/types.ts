export interface PermissionRules {
  allow: string[]
  deny: string[]
  ask: string[]
}

export type McpServerConfig =
  | {
      type: 'stdio'
      command: string
      args?: string[]
      env?: Record<string, string>
      disabled?: boolean
    }
  | {
      type: 'sse'
      url: string
      headers?: Record<string, string>
      disabled?: boolean
    }
  | {
      type: 'streamable_http'
      url: string
      headers?: Record<string, string>
      disabled?: boolean
    }

export interface RulesConfig {
  content: string
  rulesFile?: string | null
  rulesFiles?: string[]
}

export interface AgentConfigPatch {
  maxIterations?: number | null
  maxTokenBudget?: number | null
  thinkingEnabled?: boolean | null
  autoSummaryThreshold?: number | null
}

export interface Settings {
  $schema?: string
  permissions: PermissionRules
  mcpServers: Record<string, McpServerConfig>
  rules: RulesConfig
  agent: AgentConfigPatch
}

export interface EffectiveSettings {
  permissions: PermissionRules
  mcpServers: Record<string, McpServerConfig>
  rulesContent: string
  agent: {
    maxIterations: number
    maxTokenBudget: number
    thinkingEnabled: boolean
    autoSummaryThreshold: number
  }
}
