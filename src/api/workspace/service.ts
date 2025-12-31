import { invoke } from '@/utils/request'

export interface WorkspaceRecord {
  path: string
  displayName?: string | null
  activeSessionId?: number | null
  createdAt: number
  updatedAt: number
  lastAccessedAt: number
}

export interface SessionRecord {
  id: number
  workspacePath: string
  title?: string | null
  messageCount: number
  createdAt: number
  updatedAt: number
}

export interface SessionMessageRecord {
  id: number
  sessionId: number
  role: 'user' | 'assistant'
  content?: string | null
  stepsJson?: string | null
  imagesJson?: string | null
  status?: 'streaming' | 'complete' | 'error' | null
  durationMs?: number | null
  createdAt: number
}

export const workspaceService = {
  getOrCreate: async (path: string): Promise<WorkspaceRecord> => {
    return await invoke<WorkspaceRecord>('workspace_get_or_create', { path })
  },
  listSessions: async (path: string): Promise<SessionRecord[]> => {
    return await invoke<SessionRecord[]>('workspace_list_sessions', { path })
  },
  getMessages: async (sessionId: number): Promise<SessionMessageRecord[]> => {
    return await invoke<SessionMessageRecord[]>('workspace_get_messages', { sessionId })
  },
  getActiveSession: async (path: string): Promise<SessionRecord> => {
    return await invoke<SessionRecord>('workspace_get_active_session', { path })
  },
  createSession: async (path: string, title?: string): Promise<SessionRecord> => {
    return await invoke<SessionRecord>('workspace_create_session', { path, title })
  },
  setActiveSession: async (path: string, sessionId: number): Promise<void> => {
    await invoke('workspace_set_active_session', { path, sessionId })
  },
  listRecent: async (limit?: number): Promise<WorkspaceRecord[]> => {
    return await invoke<WorkspaceRecord[]>('workspace_get_recent', { limit })
  },
}

export default workspaceService
