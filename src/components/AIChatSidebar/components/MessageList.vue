<script setup lang="ts">
  import { ref, nextTick } from 'vue'
  import type { ChatMessage } from '@/types'
  import ChatMessageItem from './ChatMessageItem.vue'

  // Props定义
  interface Props {
    messages: ChatMessage[]
    hasMessages?: boolean
    isStreaming?: boolean
    emptyStateTitle?: string
    emptyStateDescription?: string
  }

  const props = withDefaults(defineProps<Props>(), {
    hasMessages: false,
    isStreaming: false,
    emptyStateTitle: '开始与AI对话',
    emptyStateDescription: '请先配置AI模型',
  })

  // 响应式引用
  const messagesContainer = ref<HTMLElement>()

  // 方法
  /**
   * 滚动到底部
   */
  const scrollToBottom = async () => {
    await nextTick()
    if (messagesContainer.value) {
      // 使用 smooth 滚动，但在流式过程中使用 auto 以提高性能
      const behavior = props.isStreaming ? 'auto' : 'smooth'
      messagesContainer.value.scrollTo({
        top: messagesContainer.value.scrollHeight,
        behavior,
      })
    }
  }

  // 暴露方法给父组件
  defineExpose({
    scrollToBottom,
    messagesContainer,
  })
</script>

<template>
  <div ref="messagesContainer" class="messages-container">
    <!-- 空状态 -->
    <div v-if="!hasMessages" class="empty-state">
      <div class="empty-icon">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
        </svg>
      </div>
      <div class="empty-title">{{ emptyStateTitle }}</div>
      <div class="empty-description">{{ emptyStateDescription }}</div>
    </div>

    <!-- 消息列表 -->
    <div v-else class="messages-list">
      <ChatMessageItem
        v-for="message in messages"
        :key="message.id"
        :message="message"
        :is-streaming="isStreaming && message.messageType === 'assistant' && message === messages[messages.length - 1]"
      />
    </div>
  </div>
</template>

<style scoped>
  .messages-container {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    background-color: var(--color-ai-sidebar-background);
  }

  .messages-container::-webkit-scrollbar {
    width: 4px;
  }

  .messages-container::-webkit-scrollbar-thumb {
    background: var(--color-border);
    border-radius: 2px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    text-align: center;
    color: var(--color-text-secondary);
    padding: 40px 20px;
  }

  .empty-icon {
    margin-bottom: 16px;
    opacity: 0.6;
  }

  .empty-title {
    font-size: 18px;
    font-weight: 600;
    margin-bottom: 12px;
    color: var(--color-text);
  }

  .empty-description {
    font-size: 14px;
    opacity: 0.7;
    line-height: 1.5;
  }

  .messages-list {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding-bottom: 16px;
  }
</style>
