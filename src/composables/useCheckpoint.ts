/**
 * Checkpoint 管理 composable
 */
import { ref } from 'vue'
import { checkpointApi } from '@/api/checkpoint'
import type { CheckpointSummary } from '@/types/domain/checkpoint'

const checkpointsMap = ref<Map<number, CheckpointSummary[]>>(new Map())
const loadingSessions = ref<Set<number>>(new Set())

export function useCheckpoint() {
  const loadCheckpoints = async (sessionId: number) => {
    if (loadingSessions.value.has(sessionId)) return

    loadingSessions.value.add(sessionId)
    try {
      const list = await checkpointApi.list({ sessionId })
      checkpointsMap.value.set(sessionId, list)
    } finally {
      loadingSessions.value.delete(sessionId)
    }
  }

  const getCheckpointByMessage = (sessionId: number, userMessage: string): CheckpointSummary | null => {
    const list = checkpointsMap.value.get(sessionId)
    if (!list) return null
    // 匹配用户消息内容找到对应的checkpoint
    return list.find(cp => cp.userMessage === userMessage) ?? null
  }

  const getChildCheckpoint = (sessionId: number, checkpointId: number): CheckpointSummary | null => {
    const list = checkpointsMap.value.get(sessionId)
    if (!list) return null
    return list.find(cp => cp.parentId === checkpointId) ?? null
  }

  const getCheckpointsBySession = (sessionId: number): CheckpointSummary[] => {
    return checkpointsMap.value.get(sessionId) ?? []
  }

  const refreshCheckpoints = async (sessionId: number) => {
    checkpointsMap.value.delete(sessionId)
    await loadCheckpoints(sessionId)
  }

  const isLoading = (sessionId: number) => {
    return loadingSessions.value.has(sessionId)
  }

  return {
    loadCheckpoints,
    getCheckpointByMessage,
    getChildCheckpoint,
    getCheckpointsBySession,
    refreshCheckpoints,
    isLoading,
  }
}
