import { invoke } from '@/utils/request'
import type { CheckpointSummary, RollbackResult, FileDiff } from '@/types/domain/checkpoint'

export const checkpointApi = {
  /**
   * 获取会话的 checkpoint 列表
   */
  async list(sessionId: number, workspacePath: string): Promise<CheckpointSummary[]> {
    if (!workspacePath) {
      console.warn('[checkpointApi] workspacePath is required')
      return []
    }
    return (await invoke<CheckpointSummary[]>('checkpoint_list', { sessionId, workspacePath })) ?? []
  },

  /**
   * 回滚到指定 checkpoint
   *
   * 只需要 checkpointId，后端从 checkpoint 记录获取 session/workspace/message 信息
   */
  async rollback(checkpointId: number): Promise<RollbackResult | null> {
    return (await invoke<RollbackResult>('checkpoint_rollback', { checkpointId })) ?? null
  },

  /**
   * 获取两个 checkpoint 之间的 diff
   */
  async diff(fromId: number | null, toId: number, workspacePath: string): Promise<FileDiff[]> {
    return (await invoke<FileDiff[]>('checkpoint_diff', { fromId, toId, workspacePath })) ?? []
  },

  /**
   * 获取 checkpoint 与当前工作区之间的 diff
   */
  async diffWithWorkspace(checkpointId: number, workspacePath: string): Promise<FileDiff[]> {
    return (await invoke<FileDiff[]>('checkpoint_diff_with_workspace', { checkpointId, workspacePath })) ?? []
  },

  /**
   * 获取 checkpoint 中某个文件的内容
   */
  async getFileContent(checkpointId: number, filePath: string): Promise<string | null> {
    return (await invoke<string | null>('checkpoint_get_file_content', { checkpointId, filePath })) ?? null
  },
}
