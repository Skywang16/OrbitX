/**
 * Checkpoint API 封装
 */
import { invoke } from '@tauri-apps/api/core'
import type { CheckpointSummary, RollbackResult, FileDiff } from '@/types/domain/checkpoint'

interface ApiResponse<T> {
  success: boolean
  data?: T
  error?: string
}

export const checkpointApi = {
  /**
   * 获取会话的checkpoint列表
   */
  async list(conversationId: number): Promise<CheckpointSummary[]> {
    const res = await invoke<ApiResponse<CheckpointSummary[]>>('checkpoint_list', {
      conversationId,
    })
    if (!res.success || !res.data) return []
    return res.data
  },

  /**
   * 回滚到指定checkpoint
   */
  async rollback(checkpointId: number, workspacePath: string): Promise<RollbackResult | null> {
    const res = await invoke<ApiResponse<RollbackResult>>('checkpoint_rollback', {
      checkpointId,
      workspacePath,
    })
    if (!res.success || !res.data) return null
    return res.data
  },

  /**
   * 获取两个checkpoint之间的diff
   */
  async diff(fromId: number, toId: number): Promise<FileDiff[]> {
    const res = await invoke<ApiResponse<FileDiff[]>>('checkpoint_diff', {
      fromId,
      toId,
    })
    if (!res.success || !res.data) return []
    return res.data
  },

  /**
   * 获取checkpoint中某个文件的内容
   */
  async getFileContent(checkpointId: number, filePath: string): Promise<string | null> {
    const res = await invoke<ApiResponse<string | null>>('checkpoint_get_file_content', {
      checkpointId,
      filePath,
    })
    if (!res.success) return null
    return res.data ?? null
  },
}
