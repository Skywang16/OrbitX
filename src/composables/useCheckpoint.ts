/**
 * Checkpoint 管理 composable
 */
import { ref, computed } from 'vue'
import { checkpointApi } from '@/api/checkpoint'
import type { CheckpointSummary } from '@/types/domain/checkpoint'

const checkpointsMap = ref<Map<number, CheckpointSummary[]>>(new Map())
const loadingConversations = ref<Set<number>>(new Set())

export function useCheckpoint() {
  const loadCheckpoints = async (conversationId: number) => {
    if (loadingConversations.value.has(conversationId)) return

    loadingConversations.value.add(conversationId)
    try {
      const list = await checkpointApi.list(conversationId)
      checkpointsMap.value.set(conversationId, list)
    } finally {
      loadingConversations.value.delete(conversationId)
    }
  }

  const getCheckpointByMessage = (conversationId: number, userMessage: string): CheckpointSummary | null => {
    const list = checkpointsMap.value.get(conversationId)
    if (!list) return null
    // 匹配用户消息内容找到对应的checkpoint
    return list.find(cp => cp.userMessage === userMessage) ?? null
  }

  const getCheckpointsByConversation = (conversationId: number): CheckpointSummary[] => {
    return checkpointsMap.value.get(conversationId) ?? []
  }

  const refreshCheckpoints = async (conversationId: number) => {
    checkpointsMap.value.delete(conversationId)
    await loadCheckpoints(conversationId)
  }

  const isLoading = (conversationId: number) => {
    return loadingConversations.value.has(conversationId)
  }

  return {
    loadCheckpoints,
    getCheckpointByMessage,
    getCheckpointsByConversation,
    refreshCheckpoints,
    isLoading,
  }
}
