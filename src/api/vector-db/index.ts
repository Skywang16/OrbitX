import { channel } from '@/api/channel'
import type { ChannelCallbacks, ChannelSubscription } from '@/api/channel'
import { invoke } from '@/utils/request'

export interface VectorIndexStatus {
  isReady: boolean
  path: string
  size?: string
  sizeBytes?: number
  totalFiles: number
  totalChunks: number
  model: string
  dim: number
}

export interface VectorBuildProgress {
  phase: 'pending' | 'collecting_files' | 'chunking' | 'embedding' | 'writing' | 'completed' | 'cancelled' | 'failed'
  root: string
  totalFiles: number
  filesDone: number
  filesFailed: number
  currentFile?: string
  currentFileChunksTotal: number
  currentFileChunksDone: number
  isDone: boolean
  error?: string
}

type RawVectorBuildProgress = {
  phase: VectorBuildProgress['phase']
  root: string
  total_files: number
  files_done: number
  files_failed: number
  current_file?: string
  current_file_chunks_total: number
  current_file_chunks_done: number
  is_done: boolean
  error?: string
}

const formatBytes = (bytes: number): string => {
  if (!Number.isFinite(bytes) || bytes <= 0) return ''
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let n = bytes
  let i = 0
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024
    i += 1
  }
  const digits = i === 0 ? 0 : n >= 100 ? 0 : n >= 10 ? 1 : 2
  return `${n.toFixed(digits)} ${units[i]}`
}

const mapBuildProgress = (raw: RawVectorBuildProgress): VectorBuildProgress => ({
  phase: raw.phase,
  root: raw.root,
  totalFiles: raw.total_files,
  filesDone: raw.files_done,
  filesFailed: raw.files_failed,
  currentFile: raw.current_file,
  currentFileChunksTotal: raw.current_file_chunks_total,
  currentFileChunksDone: raw.current_file_chunks_done,
  isDone: raw.is_done,
  error: raw.error,
})

export class VectorDbApi {
  getIndexStatus = async (params: { path: string }): Promise<VectorIndexStatus> => {
    const raw = await invoke<{
      total_files: number
      total_chunks: number
      embedding_model: string
      vector_dimension: number
      size_bytes: number
    }>('get_index_status', { path: params.path })
    return {
      isReady: raw.total_chunks > 0,
      path: params.path,
      sizeBytes: raw.size_bytes,
      size: formatBytes(raw.size_bytes),
      totalFiles: raw.total_files,
      totalChunks: raw.total_chunks,
      model: raw.embedding_model,
      dim: raw.vector_dimension,
    }
  }

  deleteWorkspaceIndex = async (path: string): Promise<void> => invoke('delete_workspace_index', { path })

  startBuildIndex = async (params: { root: string }): Promise<void> =>
    invoke('vector_build_index_start', { path: params.root })

  getBuildStatus = async (params: { root: string }): Promise<VectorBuildProgress | null> => {
    const raw = await invoke<RawVectorBuildProgress | null>('vector_build_index_status', { path: params.root })
    return raw ? mapBuildProgress(raw) : null
  }

  subscribeBuildProgress = (
    params: { root: string },
    callbacks: ChannelCallbacks<VectorBuildProgress>
  ): ChannelSubscription => {
    return channel.subscribe<RawVectorBuildProgress>(
      'vector_build_index_subscribe',
      { path: params.root },
      {
        onMessage: msg => callbacks.onMessage(mapBuildProgress(msg)),
        onError: callbacks.onError,
      },
      { cancelCommand: 'vector_build_index_cancel' }
    )
  }

  cancelBuild = async (params: { root: string }): Promise<void> =>
    invoke('vector_build_index_cancel', { path: params.root })
}

export const vectorDbApi = new VectorDbApi()
