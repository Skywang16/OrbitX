import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { debounce } from 'lodash-es'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { fileWatcherApi } from '@/api'
import type { FileWatcherEventBatch, FileWatcherStatus } from '@/types/domain/fileWatcher'
import { useTerminalStore } from '@/stores/Terminal'
import { useEditorStore } from '@/stores/Editor'
import { getWorkspacePathFromContext } from '@/tabs/context'

type Subscriber = (batch: FileWatcherEventBatch) => void

export const useFileWatcherStore = defineStore('fileWatcher', () => {
  const terminalStore = useTerminalStore()
  const editorStore = useEditorStore()

  const status = ref<FileWatcherStatus | null>(null)
  const running = computed(() => status.value?.running ?? false)

  const currentPath = computed(() => {
    const active = editorStore.activeTab
    if (!active) return null
    return getWorkspacePathFromContext(active.context, { terminals: terminalStore.terminals })
  })

  const subscribers = new Set<Subscriber>()
  let unlisten: UnlistenFn | null = null
  let watchedPath: string | null = null
  let startInFlight: Promise<void> | null = null

  const notifySubscribers = (batch: FileWatcherEventBatch) => {
    for (const cb of subscribers) cb(batch)
  }

  const subscribe = (cb: Subscriber) => {
    subscribers.add(cb)
    return () => subscribers.delete(cb)
  }

  const stop = async () => {
    if (startInFlight) await startInFlight
    watchedPath = null
    status.value = null
    await fileWatcherApi.stop().catch(() => {})
  }

  const start = async (path: string | null) => {
    if (!path || path === '~') {
      await stop()
      return
    }

    if (watchedPath === path && running.value) return

    if (startInFlight) {
      await startInFlight
      if (watchedPath === path && running.value) return
    }

    startInFlight = (async () => {
      try {
        status.value = await fileWatcherApi.start(path)
        watchedPath = path
      } finally {
        startInFlight = null
      }
    })()

    await startInFlight
  }

  const refreshStatus = async () => {
    status.value = await fileWatcherApi.status()
  }

  const initialize = async () => {
    if (unlisten) return
    unlisten = await fileWatcherApi.onEvent(notifySubscribers)

    const debouncedStart = debounce((path: string | null) => {
      void start(path)
    }, 200)

    watch(
      currentPath,
      next => {
        debouncedStart(next)
      },
      { immediate: true }
    )
  }

  const dispose = async () => {
    if (unlisten) {
      unlisten()
      unlisten = null
    }
    await stop()
    subscribers.clear()
  }

  return {
    status,
    running,
    initialize,
    dispose,
    refreshStatus,
    start,
    stop,
    subscribe,
  }
})
