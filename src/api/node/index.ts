import { invoke } from '@/utils/request'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

// Node version information
export interface NodeVersionInfo {
  version: string
  is_current: boolean
}

// Node version change event payload
export interface NodeVersionChangedPayload {
  paneId: number
  version: string
}

// Node API wrapper class
export class NodeApi {
  // Check if specified path is a Node project
  checkNodeProject = async (path: string): Promise<boolean> => {
    return invoke('node_check_project', { path })
  }

  // Get current system's Node version manager
  getVersionManager = async (): Promise<string> => {
    return invoke('node_get_version_manager')
  }

  // Get all installed Node versions
  listVersions = async (): Promise<NodeVersionInfo[]> => {
    return invoke('node_list_versions')
  }

  // Get the currently active Node version (from system, without needing a terminal)
  getCurrentVersion = async (): Promise<string | null> => {
    return invoke('node_get_current_version')
  }

  // Generate version switch command
  getSwitchCommand = async (manager: string, version: string): Promise<string> => {
    return invoke('node_get_switch_command', { manager, version })
  }

  // Listen to Node version change events
  onVersionChanged = async (callback: (payload: NodeVersionChangedPayload) => void): Promise<UnlistenFn> => {
    return listen<NodeVersionChangedPayload>('node_version_changed', event => callback(event.payload))
  }
}

export const nodeApi = new NodeApi()

export default nodeApi
