<script setup lang="ts">
  import { nextTick, ref, watch, onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import type { Message } from '@/types'
  import UserMessage from './UserMessage.vue'
  import AIMessage from './AIMessage.vue'
  import { useAISettingsStore } from '@/components/settings/components/AI/store'

  const { t } = useI18n()
  const aiSettingsStore = useAISettingsStore()

  interface Props {
    messages: Message[]
    isLoading?: boolean
    chatMode?: string
  }

  const props = defineProps<Props>()

  // 消息列表容器引用
  const messageListRef = ref<HTMLElement | null>(null)

  // 自动滚动到底部
  const scrollToBottom = async () => {
    await nextTick()
    if (messageListRef.value) {
      messageListRef.value.scrollTop = messageListRef.value.scrollHeight
    }
  }

  // 监听消息列表变化，处理滚动
  watch(
    () => props.messages.length,
    () => {
      scrollToBottom()
    },
    { immediate: true }
  )

  // 初始化AI设置
  onMounted(async () => {
    if (!aiSettingsStore.isInitialized) {
      await aiSettingsStore.loadSettings()
    }
  })
</script>

<template>
  <div ref="messageListRef" class="message-list">
    <div v-if="messages.length === 0" class="empty-state">
      <!-- 没有配置模型时的提醒 -->
      <div v-if="!aiSettingsStore.hasModels && aiSettingsStore.isInitialized" class="no-model-state">
        <div class="empty-text">{{ t('message_list.no_model_configured') }}</div>
        <div class="empty-hint">{{ t('message_list.configure_model_hint') }}</div>
      </div>

      <div v-else class="normal-empty-state">
        <div class="empty-icon">
          <svg class="empty-icon-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path
              d="M21 15a2 2 0 0 1-2 2H10l-3 5c-.3 .4-.8 .1-.8-.4v-4.6H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v10z"
            />
          </svg>
        </div>
        <div class="empty-text">{{ t('message_list.start_conversation') }}</div>
        <div class="empty-hint">{{ t('message_list.send_message_hint') }}</div>
      </div>
    </div>

    <div v-else class="message-container">
      <template v-for="message in messages" :key="message.id">
        <UserMessage v-if="message.role === 'user'" :message="message" />
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

  /* 自定义滚动条样式 */
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
    align-items: center;
    justify-content: center;
    text-align: center;
    color: var(--text-400);
    gap: var(--spacing-lg);
  }

  .no-model-state,
  .normal-empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--spacing-lg);
  }

  .empty-icon {
    margin-bottom: var(--spacing-md);
  }

  .empty-icon-svg {
    width: 64px;
    height: 64px;
    color: var(--color-primary);
    opacity: 0.6;
  }

  .empty-text {
    font-size: var(--font-size-lg);
    font-weight: 500;
    color: var(--text-200);
  }

  .empty-hint {
    font-size: var(--font-size-sm);
    opacity: 0.7;
  }

  .message-container {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
  }
</style>
