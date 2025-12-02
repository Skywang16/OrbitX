import { invoke } from '@/utils/request'

export interface VectorIndexStatus {
  isReady: boolean
  path: string
  size?: string
  totalFiles: number
  totalChunks: number
  model: string
  dim: number
}

export interface VectorBuildProgress {
  currentFile?: string
  filesCompleted: number
  totalFiles: number
  currentFileChunks?: number
  totalChunks: number
  isComplete: boolean
  error?: string
}

export class VectorDbApi {
  getIndexStatus = async (params: { path?: string } = {}): Promise<VectorIndexStatus> => {
    const raw = await invoke<{ total_files: number; total_chunks: number; embedding_model: string; vector_dimension: number }>('get_index_status')
    return {
      isReady: raw.total_chunks > 0,
      path: params.path || '.',
      size: '',
      totalFiles: raw.total_files,
      totalChunks: raw.total_chunks,
      model: raw.embedding_model,
      dim: raw.vector_dimension,
    }
  }

  // Start background build (rebuild)
  rebuildIndex = async (params: { root: string }): Promise<void> => {
    await invoke('vector_build_index', { path: params.root })
  }

  indexFiles = async (paths: string[]): Promise<void> => {
    await invoke('index_files', { paths })
  }

  updateFileIndex = async (path: string): Promise<void> => {
    await invoke('update_file_index', { path })
  }

  removeFileIndex = async (path: string): Promise<void> => {
    await invoke('remove_file_index', { path })
  }

  getBuildProgress = async (params: { path: string }): Promise<VectorBuildProgress> => {
    const p = await invoke<{ current_file?: string; files_completed: number; total_files: number; current_file_chunks?: number; total_chunks: number; is_complete: boolean; error?: string }>('vector_get_build_progress', { path: params.path })
    return {
      currentFile: p.current_file,
      filesCompleted: p.files_completed,
      totalFiles: p.total_files,
      currentFileChunks: p.current_file_chunks,
      totalChunks: p.total_chunks,
      isComplete: p.is_complete,
      error: p.error,
    }
  }

  cancelBuild = async (params: { path: string }): Promise<void> => {
    await invoke('vector_cancel_build', { path: params.path })
  }
}

export const vectorDbApi = new VectorDbApi()
export type { VectorIndexStatus, VectorBuildProgress }
