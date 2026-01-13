import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@/utils/request'
import type { FileWatcherEventBatch, FileWatcherStatus } from '@/types/domain/fileWatcher'

export type FileWatcherConfig = {
  enableFsWatcher: boolean
  enableGitWatcher: boolean
  debounceMs: number
  throttleMs: number
  ignorePatterns: string[]
}

export class FileWatcherApi {
  start = async (path: string, config?: Partial<FileWatcherConfig>): Promise<FileWatcherStatus> => {
    const args: Record<string, unknown> = { path }
    if (config) args.config = config
    return invoke<FileWatcherStatus>('file_watcher_start', args)
  }

  stop = async (): Promise<void> => {
    return invoke<void>('file_watcher_stop')
  }

  status = async (): Promise<FileWatcherStatus> => {
    return invoke<FileWatcherStatus>('file_watcher_status')
  }

  onEvent = async (callback: (batch: FileWatcherEventBatch) => void): Promise<UnlistenFn> => {
    return await listen<FileWatcherEventBatch>('file-watcher:event', event => {
      callback(event.payload)
    })
  }
}

export const fileWatcherApi = new FileWatcherApi()
export default fileWatcherApi
