import { invoke } from '@/utils/request'
import type { WorkspaceIndex, BuildWorkspaceIndexParams } from './types'

export interface CkSearchParams {
  query: string
  maxResults?: number
  minScore?: number
  directory?: string
  languageFilter?: string
  mode?: 'semantic' | 'hybrid' | 'regex' | 'lexical'
  fullSection?: boolean
  reindex?: boolean
}

// Raw shape from Rust (snake_case)
interface CkSearchResultItemRaw {
  file_path: string
  content: string
  start_line: number
  end_line: number
  language: string
  chunk_type: string
  score: number
}

// Frontend-consumed shape (camelCase)
export interface VectorSearchResult {
  filePath: string
  content: string
  startLine: number
  endLine: number
  language: string
  chunkType: string
  score: number
}

export class CkApi {
  async search(params: CkSearchParams): Promise<VectorSearchResult[]> {
    const raw = await invoke<CkSearchResultItemRaw[]>('code_search', { params })
    return raw.map(r => ({
      filePath: r.file_path,
      content: r.content,
      startLine: r.start_line,
      endLine: r.end_line,
      language: r.language,
      chunkType: r.chunk_type,
      score: r.score,
    }))
  }

  async checkCurrentWorkspace(): Promise<WorkspaceIndex | null> {
    return await invoke<WorkspaceIndex | null>('workspace_check_current_index')
  }

  async buildWorkspaceIndex(params: BuildWorkspaceIndexParams): Promise<WorkspaceIndex> {
    return await invoke<WorkspaceIndex>('workspace_build_index', {
      workspacePath: params.path,
      name: params.name,
    })
  }

  async getAllWorkspaces(): Promise<WorkspaceIndex[]> {
    return await invoke<WorkspaceIndex[]>('workspace_get_all_indexes')
  }

  async deleteWorkspace(id: number): Promise<void> {
    return await invoke<void>('workspace_delete_index', { id })
  }

  async refreshWorkspace(id: number): Promise<WorkspaceIndex> {
    return await invoke<WorkspaceIndex>('workspace_refresh_index', { id })
  }
}

export const ckApi = new CkApi()
export const workspaceIndexApi = ckApi

// 导出类型
export type * from './types'
