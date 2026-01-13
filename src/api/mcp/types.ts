export interface McpToolInfo {
  name: string
  description?: string
}

export interface McpServerStatus {
  name: string
  source: 'global' | 'workspace'
  status: 'connected' | 'disconnected' | 'error'
  tools: McpToolInfo[]
  error?: string | null
}

export interface McpTestResult {
  success: boolean
  toolsCount: number
  error?: string | null
}
