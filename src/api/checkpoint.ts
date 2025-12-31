import { invoke } from '@/utils/request'
import type { CheckpointSummary, RollbackResult, FileDiff } from '@/types/domain/checkpoint'

export const checkpointApi = {
  /**
   * 获取会话的checkpoint列表
   */
  async list(params: { workspacePath?: string; sessionId?: number }): Promise<CheckpointSummary[]> {
    const res = await invoke<CheckpointSummary[]>('checkpoint_list', {
      workspacePath: params.workspacePath,
      sessionId: params.sessionId,
    })
    return res ?? []
  },

  /**
   * 回滚到指定checkpoint
   */
  async rollback(checkpointId: number): Promise<RollbackResult | null> {
    const res = await invoke<RollbackResult>('checkpoint_rollback', {
      checkpointId,
    })
    return res ?? null
  },

  /**
   * 获取两个checkpoint之间的diff
   */
  async diff(fromId: number | null, toId: number, workspacePath: string): Promise<FileDiff[]> {
    const res = await invoke<FileDiff[]>('checkpoint_diff', {
      fromId,
      toId,
      workspacePath,
    })
    return res ?? []
  },

  /**
   * 获取checkpoint中某个文件的内容
   */
  async getFileContent(checkpointId: number, filePath: string): Promise<string | null> {
    const res = await invoke<string | null>('checkpoint_get_file_content', {
      checkpointId,
      filePath,
    })
    return res ?? null
  },

  /**
   * 获取 checkpoint 与当前工作区之间的 diff
   */
  async diffWithWorkspace(checkpointId: number, workspacePath: string): Promise<FileDiff[]> {
    const res = await invoke<FileDiff[]>('checkpoint_diff_with_workspace', {
      checkpointId,
      workspacePath,
    })
    return res ?? []
  },
}
