<script setup lang="ts">
  import { nextTick, ref, watch, onMounted, computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import type { Message } from '@/types'
  import UserMessage from './UserMessage.vue'
  import AIMessage from './AIMessage.vue'
  import { useAISettingsStore } from '@/components/settings/components/AI/store'
  import { useCheckpoint } from '@/composables/useCheckpoint'
  import { useAIChatStore } from '../../store'

  const { t } = useI18n()
  const aiSettingsStore = useAISettingsStore()
  const aiChatStore = useAIChatStore()
  const { loadCheckpoints, getCheckpointByMessageId } = useCheckpoint()

  interface Props {
    messages: Message[]
    isLoading?: boolean
    chatMode?: string
    sessionId?: number | null
    workspacePath?: string
  }

  const props = defineProps<Props>()

  // 获取最近 3 条历史会话（排除当前会话）
  const recentSessions = computed(() => {
    return aiChatStore.sessions.filter(s => s.id !== props.sessionId).slice(0, 3)
  })

  // 格式化时间
  const formatTime = (date: Date) => {
    const now = new Date()
    const diff = now.getTime() - date.getTime()
    const hours = Math.floor(diff / (1000 * 60 * 60))
    const days = Math.floor(hours / 24)

    if (hours < 1) return 'Now'
    if (hours < 24) return `${hours}h`
    if (days < 7) return `${days}d`
    return date.toLocaleDateString()
  }

  const handleSelectSession = (sessionId: number) => {
    aiChatStore.switchSession(sessionId)
  }

  const messageListRef = ref<HTMLElement | null>(null)

  const scrollToBottom = async () => {
    await nextTick()
    if (messageListRef.value) {
      messageListRef.value.scrollTop = messageListRef.value.scrollHeight
    }
  }

  const previousLength = ref(props.messages.length)

  // 获取消息对应的 checkpoint（使用 message.id 查找）
  const getCheckpoint = (message: Message) => {
    if (!props.sessionId || !props.workspacePath || message.role !== 'user') return null
    return getCheckpointByMessageId(props.sessionId, props.workspacePath, message.id)
  }

  watch(
    () => props.messages.length,
    newLength => {
      if (newLength > previousLength.value) {
        scrollToBottom()
      }
      previousLength.value = newLength
    }
  )

  // 当会话ID变化时加载checkpoints
  watch(
    () => [props.sessionId, props.workspacePath] as const,
    async ([newId, workspacePath]) => {
      if (newId && newId > 0 && workspacePath) {
        await loadCheckpoints(newId, workspacePath)
      }
    },
    { immediate: true }
  )

  // 当消息列表变化时刷新checkpoints
  watch(
    () => props.messages.length,
    async () => {
      if (props.sessionId && props.sessionId > 0 && props.workspacePath) {
        await loadCheckpoints(props.sessionId, props.workspacePath)
      }
    }
  )

  onMounted(async () => {
    if (!aiSettingsStore.isInitialized) {
      await aiSettingsStore.loadSettings()
    }
  })
</script>

<template>
  <div ref="messageListRef" class="message-list">
    <div v-if="messages.length === 0" class="empty-state">
      <!-- 没有配置模型 -->
      <div v-if="!aiSettingsStore.hasModels && aiSettingsStore.isInitialized" class="no-model-state">
        <div class="empty-text">{{ t('message_list.no_model_configured') }}</div>
        <div class="empty-hint">{{ t('message_list.configure_model_hint') }}</div>
      </div>

      <!-- 有历史会话时显示最近记录 -->
      <div v-else-if="recentSessions.length > 0" class="recent-sessions-wrapper">
        <div class="recent-sessions">
          <div
            v-for="session in recentSessions"
            :key="session.id"
            class="recent-item"
            @click="handleSelectSession(session.id)"
          >
            <span class="recent-title">{{ session.title }}</span>
            <span class="recent-time">{{ formatTime(session.updatedAt) }}</span>
          </div>
        </div>
      </div>

      <!-- 没有历史会话：什么都不显示 -->
      <div v-else></div>
    </div>

    <div v-else class="message-container">
      <template v-for="message in messages" :key="message.id">
        <UserMessage
          v-if="message.role === 'user'"
          :message="message"
          :checkpoint="getCheckpoint(message)"
          :workspace-path="workspacePath"
        />
        <AIMessage v-else-if="message.role === 'assistant'" :message="message" />
      </template>
    </div>
  </div>
</template>

<style scoped>
  .message-list {
    flex: 1;
    overflow-y: auto;
    padding: var(--spacing-md);
    display: flex;
    flex-direction: column;
    scrollbar-gutter: stable;
  }

  .message-list::-webkit-scrollbar {
    width: 8px;
  }

  .message-list::-webkit-scrollbar-track {
    background: var(--bg-200);
    border-radius: 4px;
  }

  .message-list::-webkit-scrollbar-thumb {
    background: var(--border-300);
    border-radius: 4px;
    transition: background-color 0.2s ease;
  }

  .message-list::-webkit-scrollbar-thumb:hover {
    background: var(--border-400);
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    color: var(--text-400);
  }

  .no-model-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--spacing-lg);
  }

  .empty-text {
    font-size: var(--font-size-lg);
    font-weight: 500;
    color: var(--text-200);
  }

  .empty-hint {
    font-size: var(--font-size-sm);
    color: var(--text-200);
  }

  /* 历史会话列表 */
  .recent-sessions-wrapper {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--spacing-md);
  }

  .recent-sessions {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
    width: 100%;
    max-width: 320px;
  }

  .recent-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--spacing-md);
    background: var(--bg-300);
    border-radius: var(--border-radius-md);
    cursor: pointer;
    transition: background-color 0.15s ease;
    user-select: none;
  }

  .recent-item:hover {
    background: var(--bg-400);
  }

  .recent-title {
    font-size: 13px;
    color: var(--text-300);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    min-width: 0;
  }

  .recent-time {
    font-size: 12px;
    color: var(--text-500);
    flex-shrink: 0;
    margin-left: 16px;
  }

  .message-container {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
  }
</style>
