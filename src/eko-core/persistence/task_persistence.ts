import { invoke } from '@/utils/request'
import type { StreamCallbackMessage } from '@/eko-core/types'

// Minimal per-task UI event persistence with buffered flush
type UIEvent = StreamCallbackMessage & { _ts: number }

class TaskPersistenceService {
  private uiBuffers: Map<string, UIEvent[]> = new Map()
  private uiLoaded: Set<string> = new Set()
  private timers: Map<string, number> = new Map()
  private flushIntervalMs = 800

  private getBuffer(taskId: string): UIEvent[] {
    if (!this.uiBuffers.has(taskId)) {
      this.uiBuffers.set(taskId, [])
    }
    return this.uiBuffers.get(taskId) as UIEvent[]
  }

  // Append UI event and schedule flush
  async appendUiEvent(taskId: string, event: StreamCallbackMessage): Promise<void> {
    // On first append, try to preload existing file to avoid overwrite
    if (!this.uiLoaded.has(taskId)) {
      try {
        const existing = await invoke<unknown>('task_read_ui_messages', { task_id: taskId })
        if (Array.isArray(existing)) {
          this.uiBuffers.set(taskId, existing as UIEvent[])
        } else if (existing && typeof existing === 'object' && Array.isArray((existing as any).events)) {
          this.uiBuffers.set(taskId, ((existing as any).events || []) as UIEvent[])
        }
      } catch {
        // ignore
      } finally {
        this.uiLoaded.add(taskId)
      }
    }

    const buf = this.getBuffer(taskId)
    buf.push({ ...(event as StreamCallbackMessage), _ts: Date.now() })
    this.scheduleFlush(taskId)
  }

  private scheduleFlush(taskId: string) {
    const old = this.timers.get(taskId)
    if (old) {
      clearTimeout(old)
    }
    const timer = setTimeout(() => {
      this.flushUi(taskId).catch(() => {})
    }, this.flushIntervalMs) as unknown as number
    this.timers.set(taskId, timer)
  }

  async flushUi(taskId: string): Promise<void> {
    const buf = this.getBuffer(taskId)
    try {
      await invoke('task_save_ui_messages', { task_id: taskId, messages: buf })
    } catch (e) {
      // swallow to avoid breaking UX
      // console.warn('flushUi failed', e)
    }
  }

  async saveApiMessages(taskId: string, messages: unknown): Promise<void> {
    await invoke('task_save_api_messages', { task_id: taskId, messages })
  }

  async readApiMessages<T = unknown>(taskId: string): Promise<T> {
    return await invoke<T>('task_read_api_messages', { task_id: taskId })
  }

  async updateMetadataPartial(taskId: string, patch: Record<string, unknown>): Promise<void> {
    try {
      const prev = await invoke<Record<string, unknown>>('task_read_metadata', { task_id: taskId })
      const next = { ...(prev || {}), ...patch }
      await invoke('task_save_metadata', { task_id: taskId, metadata: next })
    } catch {
      await invoke('task_save_metadata', { task_id: taskId, metadata: patch })
    }
  }

  async saveCheckpoint(taskId: string, checkpoint: unknown, name?: string): Promise<string> {
    return await invoke<string>('task_checkpoint_save', { task_id: taskId, checkpoint, name })
  }

  async listCheckpoints(taskId: string): Promise<string[]> {
    return await invoke<string[]>('task_checkpoint_list', { task_id: taskId })
  }

  async purgeAll(): Promise<void> {
    await invoke('task_purge_all')
  }
}

export const taskPersistence = new TaskPersistenceService()
export default taskPersistence
