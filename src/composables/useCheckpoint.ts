/**
 * Checkpoint 管理 composable
 */
import { ref } from 'vue'
import { checkpointApi } from '@/api/checkpoint'
import type { CheckpointSummary } from '@/types/domain/checkpoint'

const checkpointsMap = ref<Map<string, CheckpointSummary[]>>(new Map())
const loadingSessions = ref<Set<string>>(new Set())

const makeKey = (sessionId: number, workspacePath: string) => `${workspacePath}::${sessionId}`

export function useCheckpoint() {
  const loadCheckpoints = async (sessionId: number, workspacePath: string) => {
    if (!workspacePath) return
    const key = makeKey(sessionId, workspacePath)
    if (loadingSessions.value.has(key)) return

    loadingSessions.value.add(key)
    try {
      const list = await checkpointApi.list(sessionId, workspacePath)
      checkpointsMap.value.set(key, list)
    } finally {
      loadingSessions.value.delete(key)
    }
  }

  /**
   * 通过 messageId 查找 checkpoint
   */
  const getCheckpointByMessageId = (
    sessionId: number,
    workspacePath: string,
    messageId: number
  ): CheckpointSummary | null => {
    const list = checkpointsMap.value.get(makeKey(sessionId, workspacePath))
    if (!list) return null
    return list.find(cp => cp.messageId === messageId) ?? null
  }

  /**
   * 获取指定 checkpoint 的子 checkpoint
   */
  const getChildCheckpoint = (
    sessionId: number,
    workspacePath: string,
    checkpointId: number
  ): CheckpointSummary | null => {
    const list = checkpointsMap.value.get(makeKey(sessionId, workspacePath))
    if (!list) return null
    return list.find(cp => cp.parentId === checkpointId) ?? null
  }

  const getCheckpointsBySession = (sessionId: number, workspacePath: string): CheckpointSummary[] => {
    return checkpointsMap.value.get(makeKey(sessionId, workspacePath)) ?? []
  }

  const refreshCheckpoints = async (sessionId: number, workspacePath: string) => {
    const key = makeKey(sessionId, workspacePath)
    checkpointsMap.value.delete(key)
    await loadCheckpoints(sessionId, workspacePath)
  }

  const isLoading = (sessionId: number, workspacePath: string) => {
    return loadingSessions.value.has(makeKey(sessionId, workspacePath))
  }

  return {
    loadCheckpoints,
    getCheckpointByMessageId,
    getChildCheckpoint,
    getCheckpointsBySession,
    refreshCheckpoints,
    isLoading,
  }
}
