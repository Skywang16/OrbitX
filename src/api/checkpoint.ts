import type { CheckpointSummary, FileDiff, RollbackResult } from '@/types/domain/checkpoint'
import { invoke } from '@/utils/request'

export const checkpointApi = {
  /**
   * Get checkpoint list for thread
   */
  list: async (threadId: number, workspacePath: string): Promise<CheckpointSummary[]> => {
    if (!workspacePath) {
      console.warn('[checkpointApi] workspacePath is required')
      return []
    }
    return (await invoke<CheckpointSummary[]>('checkpoint_list', { threadId, workspacePath })) ?? []
  },

  /**
   * Rollback to specified checkpoint
   *
   * Only checkpointId needed, backend retrieves thread/workspace/message information from checkpoint record
   */
  rollback: async (checkpointId: number): Promise<RollbackResult | null> => {
    return (await invoke<RollbackResult>('checkpoint_rollback', { checkpointId })) ?? null
  },

  /**
   * Get diff between two checkpoints
   */
  diff: async (fromId: number | null, toId: number, workspacePath: string): Promise<FileDiff[]> => {
    return (await invoke<FileDiff[]>('checkpoint_diff', { fromId, toId, workspacePath })) ?? []
  },

  /**
   * Get diff between checkpoint and current workspace
   */
  diffWithWorkspace: async (checkpointId: number, workspacePath: string): Promise<FileDiff[]> => {
    return (await invoke<FileDiff[]>('checkpoint_diff_with_workspace', { checkpointId, workspacePath })) ?? []
  },

  /**
   * Get content of a file in checkpoint
   */
  getFileContent: async (checkpointId: number, filePath: string): Promise<string | null> => {
    return (await invoke<string | null>('checkpoint_get_file_content', { checkpointId, filePath })) ?? null
  },
}
