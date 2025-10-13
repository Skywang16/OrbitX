import { invoke } from '@/utils/request'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

// Node 版本信息
export interface NodeVersionInfo {
  version: string
  is_current: boolean
}

// Node 版本变化事件载荷
export interface NodeVersionChangedPayload {
  paneId: number
  version: string
}

// Node API 封装类
export class NodeApi {
  // 检查指定路径是否为 Node 项目
  checkNodeProject = async (path: string): Promise<boolean> => {
    return invoke('node_check_project', { path })
  }

  // 获取当前系统的 Node 版本管理器
  getVersionManager = async (): Promise<string> => {
    return invoke('node_get_version_manager')
  }

  // 获取所有已安装的 Node 版本
  listVersions = async (): Promise<NodeVersionInfo[]> => {
    return invoke('node_list_versions')
  }

  // 生成版本切换命令
  getSwitchCommand = async (manager: string, version: string): Promise<string> => {
    return invoke('node_get_switch_command', { manager, version })
  }

  // 监听 Node 版本变化事件
  onVersionChanged = async (callback: (payload: NodeVersionChangedPayload) => void): Promise<UnlistenFn> => {
    return listen<NodeVersionChangedPayload>('node_version_changed', event => callback(event.payload))
  }
}

export const nodeApi = new NodeApi()

export default nodeApi
