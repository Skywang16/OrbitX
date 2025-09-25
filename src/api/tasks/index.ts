import { invoke } from '@/utils/request'

// ============================================================================
// 双轨制任务系统 API - 按照 task-system-architecture-final.md 设计
// ============================================================================

export interface UITask {
  ui_id?: number
  conversation_id: number
  task_id: string
  name: string
  status: 'init' | 'active' | 'paused' | 'completed' | 'error'
  parent_ui_id?: number
  render_json?: string
  created_at?: string
  updated_at?: string
}

export interface EkoContext {
  id?: number
  task_id: string
  conversation_id: number
  kind: 'state' | 'event' | 'snapshot'
  name?: string
  node_id?: string
  status?: 'init' | 'running' | 'paused' | 'aborted' | 'done' | 'error'
  payload_json: string
  created_at?: string
}

export class TaskAPI {
  // ---- 原始上下文轨 API ----

  /** 更新或插入任务状态到上下文轨 */
  static async ekoCtxUpsertState(
    taskId: string,
    context: string,
    conversationId?: number,
    nodeId?: string,
    status?: string
  ): Promise<number> {
    return await invoke<number>('eko_ctx_upsert_state', {
      taskId,
      context,
      conversationId,
      nodeId,
      status,
    })
  }

  /** 追加事件到上下文轨 */
  static async ekoCtxAppendEvent(taskId: string, event: string, nodeId?: string): Promise<number> {
    return await invoke<number>('eko_ctx_append_event', { taskId, event, nodeId })
  }

  /** 保存快照到上下文轨 */
  static async ekoCtxSnapshotSave(taskId: string, snapshot: string, name?: string): Promise<number> {
    return await invoke<number>('eko_ctx_snapshot_save', { taskId, name, snapshot })
  }

  /** 获取任务的最新状态 */
  static async ekoCtxGetState(taskId: string): Promise<EkoContext | null> {
    return await invoke<EkoContext | null>('eko_ctx_get_state', { taskId })
  }

  /** 重建任务执行上下文（用于恢复/重跑） */
  static async ekoCtxRebuild(taskId: string, fromSnapshotName?: string): Promise<string> {
    return await invoke<string>('eko_ctx_rebuild', { taskId, fromSnapshotName })
  }

  /** 构建Prompt（统一入口） */
  static async ekoCtxBuildPrompt(
    taskId: string,
    userInput: string,
    paneId?: string,
    tagContext?: string
  ): Promise<string> {
    return await invoke<string>('eko_ctx_build_prompt', { taskId, userInput, paneId, tagContext })
  }

  // ---- UI 轨 API ----

  /** 创建或更新UI任务 */
  static async uiTaskUpsert(record: UITask): Promise<number> {
    return await invoke<number>('ui_task_upsert', { record })
  }

  /** 批量创建或更新UI任务 */
  static async uiTaskBulkUpsert(records: UITask[]): Promise<number[]> {
    return await invoke<number[]>('ui_task_bulk_upsert', { records })
  }

  /** 获取会话的UI任务列表 */
  static async uiTaskList(conversationId: number, filters?: string): Promise<UITask[]> {
    return await invoke<UITask[]>('ui_task_list', { conversationId, filters })
  }

  /** 删除UI任务 */
  static async uiTaskDelete(uiId: number): Promise<void> {
    await invoke<void>('ui_task_delete', { uiId })
  }
}

export default TaskAPI
