<script setup lang="ts">
  import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'
  import type { Message } from '@/types'
  import UserMessage from './UserMessage.vue'
  import AIMessage from './AIMessage.vue'

  const { t } = useI18n()

  interface Props {
    messages: Message[]
  }

  const props = defineProps<Props>()

  // 消息列表容器引用
  const messageListRef = ref<HTMLElement | null>(null)

  // 消息列表
  const msgList = computed(() => {
    return props.messages.map(msg => ({
      ...msg,
      type: msg.role as 'user' | 'assistant',
    }))
  })

  // 自动滚动到底部
  const scrollToBottom = async () => {
    await nextTick()
    if (messageListRef.value) {
      messageListRef.value.scrollTop = messageListRef.value.scrollHeight
    }
  }

  // 监听消息列表变化，处理滚动
  watch(
    () => msgList.value.length,
    () => {
      // 自动滚动到底部
      scrollToBottom()
    },
    { immediate: true }
  )
</script>

<template>
  <div ref="messageListRef" class="message-list">
    <div v-if="msgList.length === 0" class="empty-state">
      <div class="empty-icon">
        <svg class="empty-icon-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M21 15a2 2 0 0 1-2 2H9l-3 3-1-3H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
        </svg>
      </div>
      <div class="empty-text">{{ t('message_list.start_conversation') }}</div>
      <div class="empty-hint">{{ t('message_list.send_message_hint') }}</div>
    </div>

    <div v-else class="message-container">
      <template v-for="message in msgList" :key="message.id">
        <!-- 用户消息 -->
        <UserMessage v-if="message.type === 'user'" :message="message" />

        <!-- AI消息 -->
        <AIMessage v-else-if="message.type === 'assistant'" :message="message" />
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
