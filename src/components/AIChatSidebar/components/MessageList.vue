<script setup lang="ts">
  import { computed, nextTick, ref, watch } from 'vue'
  import type { Message } from '@/types/features/ai/chat'
  import UserMessage from './UserMessage.vue'
  import AIMessage from './AIMessage.vue'

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

  // 监听消息变化，自动滚动到底部
  watch(
    () => msgList.value.length,
    () => {
      scrollToBottom()
    },
    { immediate: true }
  )
</script>

<template>
  <div ref="messageListRef" class="message-list">
    <div v-if="msgList.length === 0" class="empty-state">
      <div class="empty-text">开始对话吧</div>
      <div class="empty-hint">发送消息开始与AI助手对话</div>
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
    gap: var(--spacing-md);
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
