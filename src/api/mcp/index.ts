import { invoke } from '@/utils/request'
import type { McpServerConfig } from '@/api/settings'
import type { McpServerStatus, McpTestResult } from './types'

export class McpApi {
  listServers = async (workspace?: string): Promise<McpServerStatus[]> => {
    return await invoke<McpServerStatus[]>('list_mcp_servers', { workspace: workspace ?? null })
  }

  testServer = async (name: string, config: McpServerConfig, workspace?: string): Promise<McpTestResult> => {
    return await invoke<McpTestResult>('test_mcp_server', { name, config, workspace: workspace ?? null })
  }

  reloadServers = async (workspace?: string): Promise<McpServerStatus[]> => {
    return await invoke<McpServerStatus[]>('reload_mcp_servers', { workspace: workspace ?? null })
  }
}

export const mcpApi = new McpApi()
export type * from './types'
