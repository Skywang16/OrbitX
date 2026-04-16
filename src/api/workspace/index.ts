import type { Message, SubagentRecord } from '@/types'
import { invoke } from '@/utils/request'

export interface WorkspaceRecord {
  path: string
  displayName?: string | null
  activeThreadId?: number | null
  selectedRunActionId?: string | null
  createdAt: number
  updatedAt: number
  lastAccessedAt: number
}

export interface ThreadRecord {
  id: number
  workspacePath: string
  parentThreadId?: number | null
  title: string
  messageCount: number
  status: 'idle' | 'running' | 'completed' | 'error' | 'cancelled'
  agentType: string
  createdAt: number
  updatedAt: number
}

export interface ExecutionNodeRecord {
  id: number
  backingThreadId?: number | null
  role: 'root' | 'branch'
  profile: string
  title: string
  status: 'queued' | 'running' | 'completed' | 'error' | 'cancelled' | 'idle'
  startedAt?: number | null
  finishedAt?: number | null
  children: ExecutionNodeRecord[]
}

export interface ThreadViewRecord {
  thread: ThreadRecord
  timeline: ThreadTimelineItemRecord[]
  executionTree: ExecutionNodeRecord[]
}

export interface ThreadTimelineItemRecord {
  id: string
  messageId: number
  title: string
  createdAt: number
  status?: 'queued' | 'running' | 'completed' | 'error' | 'cancelled' | 'idle' | null
}

export interface RunActionRecord {
  id: string
  workspacePath: string
  name: string
  command: string
  sortOrder: number
}

export class WorkspaceApi {
  getOrCreate = async (path: string): Promise<WorkspaceRecord> => {
    return invoke<WorkspaceRecord>('workspace_get_or_create', { path })
  }

  deleteWorkspace = async (path: string): Promise<void> => {
    await invoke('workspace_remove_recent', { path })
  }

  listThreadViews = async (path: string): Promise<ThreadViewRecord[]> => {
    return invoke<ThreadViewRecord[]>('workspace_list_thread_views', { path })
  }

  createThread = async (path: string, title?: string): Promise<ThreadRecord> => {
    return invoke<ThreadRecord>('workspace_create_thread', { path, title })
  }

  deleteThread = async (threadId: number): Promise<void> => {
    await invoke('workspace_delete_thread', { threadId })
  }

  getActiveThread = async (path: string): Promise<ThreadRecord> => {
    return invoke<ThreadRecord>('workspace_get_active_thread', { path })
  }

  getThread = async (threadId: number): Promise<ThreadRecord> => {
    return invoke<ThreadRecord>('workspace_get_thread', { threadId })
  }

  setActiveThread = async (path: string, threadId: number): Promise<void> => {
    await invoke('workspace_set_active_thread', { path, threadId })
  }

  clearActiveThread = async (path: string): Promise<void> => {
    await invoke('workspace_clear_active_thread', { path })
  }

  getThreadMessages = async (threadId: number, limit?: number, beforeId?: number): Promise<Message[]> => {
    return invoke<Message[]>('workspace_get_thread_messages', { threadId, limit, beforeId })
  }

  listSubagents = async (threadId: number): Promise<SubagentRecord[]> => {
    return invoke<SubagentRecord[]>('workspace_list_subagents', { threadId })
  }

  listRecent = async (limit?: number): Promise<WorkspaceRecord[]> => {
    return invoke<WorkspaceRecord[]>('workspace_get_recent', { limit })
  }

  addRecentWorkspace = async (path: string): Promise<void> => {
    await invoke('workspace_add_recent', { path })
  }

  maintainWorkspaces = async (): Promise<[number, number]> => {
    return invoke<[number, number]>('workspace_maintain')
  }

  getProjectRules = async (): Promise<string | null> => {
    return invoke<string | null>('workspace_get_project_rules')
  }

  setProjectRules = async (rules: string | null): Promise<void> => {
    await invoke<void>('workspace_set_project_rules', { rules })
  }

  listAvailableRulesFiles = async (cwd: string): Promise<string[]> => {
    return invoke<string[]>('workspace_list_rules_files', { cwd })
  }

  listRunActions = async (path: string): Promise<RunActionRecord[]> => {
    return invoke<RunActionRecord[]>('workspace_list_run_actions', { path })
  }

  createRunAction = async (path: string, name: string, command: string): Promise<RunActionRecord> => {
    return invoke<RunActionRecord>('workspace_create_run_action', { path, name, command })
  }

  updateRunAction = async (id: string, name: string, command: string): Promise<void> => {
    await invoke('workspace_update_run_action', { id, name, command })
  }

  deleteRunAction = async (id: string): Promise<void> => {
    await invoke('workspace_delete_run_action', { id })
  }

  setSelectedRunAction = async (path: string, actionId: string | null): Promise<void> => {
    await invoke('workspace_set_selected_run_action', { path, actionId })
  }

  getPreferences = async (keys: string[]): Promise<Record<string, string>> => {
    return invoke<Record<string, string>>('preferences_get_batch', { keys })
  }

  setPreference = async (key: string, value: string | null): Promise<void> => {
    await invoke('preferences_set', { key, value })
  }
}

export const workspaceApi = new WorkspaceApi()

export default workspaceApi
