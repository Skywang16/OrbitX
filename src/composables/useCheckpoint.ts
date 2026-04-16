/**
 * Checkpoint management composable
 */
import { checkpointApi } from '@/api/checkpoint'
import type { CheckpointSummary } from '@/types/domain/checkpoint'
import { ref } from 'vue'

const checkpointsMap = ref<Map<string, CheckpointSummary[]>>(new Map())
const loadingThreads = ref<Set<string>>(new Set())

const makeKey = (threadId: number, workspacePath: string) => `${workspacePath}::${threadId}`

export const useCheckpoint = () => {
  const loadCheckpoints = async (threadId: number, workspacePath: string) => {
    if (!workspacePath) return
    const key = makeKey(threadId, workspacePath)
    if (loadingThreads.value.has(key)) return

    loadingThreads.value.add(key)
    try {
      const list = await checkpointApi.list(threadId, workspacePath)
      checkpointsMap.value.set(key, list)
    } finally {
      loadingThreads.value.delete(key)
    }
  }

  /**
   * Find checkpoint by messageId
   */
  const getCheckpointByMessageId = (
    threadId: number,
    workspacePath: string,
    messageId: number
  ): CheckpointSummary | null => {
    const list = checkpointsMap.value.get(makeKey(threadId, workspacePath))
    if (!list) return null
    return list.find(cp => cp.messageId === messageId) ?? null
  }

  /**
   * Get child checkpoint of specified checkpoint
   */
  const getChildCheckpoint = (
    threadId: number,
    workspacePath: string,
    checkpointId: number
  ): CheckpointSummary | null => {
    const list = checkpointsMap.value.get(makeKey(threadId, workspacePath))
    if (!list) return null
    return list.find(cp => cp.parentId === checkpointId) ?? null
  }

  const getCheckpointsByThread = (threadId: number, workspacePath: string): CheckpointSummary[] => {
    return checkpointsMap.value.get(makeKey(threadId, workspacePath)) ?? []
  }

  const refreshCheckpoints = async (threadId: number, workspacePath: string) => {
    const key = makeKey(threadId, workspacePath)
    checkpointsMap.value.delete(key)
    await loadCheckpoints(threadId, workspacePath)
  }

  const isLoading = (threadId: number, workspacePath: string) => {
    return loadingThreads.value.has(makeKey(threadId, workspacePath))
  }

  return {
    loadCheckpoints,
    getCheckpointByMessageId,
    getChildCheckpoint,
    getCheckpointsByThread,
    refreshCheckpoints,
    isLoading,
  }
}
