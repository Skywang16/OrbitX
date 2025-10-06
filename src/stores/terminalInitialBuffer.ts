/*
 * VSCode-like initial data events buffer for terminals.
 * Stores per-pane binary chunks temporarily until the UI is ready to render.
 */

const TTL_MS = 10_000

type Entry = {
  chunks: Uint8Array[]
  timer: number | null
  createdAt: number
}

const buffers = new Map<number, Entry>()

function ensureEntry(paneId: number): Entry {
  let entry = buffers.get(paneId)
  if (!entry) {
    entry = { chunks: [], timer: null, createdAt: Date.now() }
    buffers.set(paneId, entry)
    // Auto-expire like VSCode's _initialDataEvents timeout
    entry.timer = window.setTimeout(() => {
      const e = buffers.get(paneId)
      if (e) {
        buffers.delete(paneId)
      }
    }, TTL_MS)
  }
  return entry
}

export const terminalInitialBuffer = {
  append(paneId: number, bytes: Uint8Array) {
    const entry = ensureEntry(paneId)
    entry.chunks.push(bytes)
  },
  takeAndClear(paneId: number): Uint8Array[] {
    const entry = buffers.get(paneId)
    if (!entry) return []
    if (entry.timer) {
      clearTimeout(entry.timer)
    }
    buffers.delete(paneId)
    return entry.chunks
  },
  clear(paneId: number) {
    const entry = buffers.get(paneId)
    if (entry?.timer) {
      clearTimeout(entry.timer)
    }
    if (entry) buffers.delete(paneId)
  },
  size(paneId: number): number {
    return buffers.get(paneId)?.chunks.length ?? 0
  },
}
