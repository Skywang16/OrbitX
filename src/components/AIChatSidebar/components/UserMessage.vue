<script setup lang="ts">
  import type { Message } from '@/types/features/ai/chat'

  interface Props {
    message: Message
  }

  defineProps<Props>()

  import { formatTime } from '@/utils/dateFormatter'
</script>

<template>
  <div class="user-message">
    <div class="user-message-content">
      <div class="user-message-bubble">
        <div class="user-message-text">{{ message.content }}</div>
      </div>
      <div class="user-message-time">{{ formatTime(message.createdAt) }}</div>
    </div>
  </div>
</template>

<style scoped>
  .user-message {
    display: flex;
    justify-content: flex-end;
    margin-bottom: var(--spacing-sm);
  }

  .user-message-content {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    max-width: 80%;
  }

  .user-message-bubble {
    background: var(--color-blue);
    color: white;
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--border-radius-lg);
    border-bottom-right-radius: var(--border-radius-sm);
    position: relative;
    word-wrap: break-word;
    word-break: break-word;
    white-space: pre-wrap;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
  }

  .user-message-bubble::after {
    content: '';
    position: absolute;
    bottom: 0;
    right: -6px;
    width: 0;
    height: 0;
    border: 6px solid transparent;
    border-left-color: var(--color-blue);
    border-bottom: 0;
    border-right: 0;
    margin-bottom: -6px;
  }

  .user-message-text {
    font-size: var(--font-size-md);
    line-height: 1.4;
    margin: 0;
  }

  .user-message-time {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    margin-top: var(--spacing-xs);
    margin-right: var(--spacing-sm);
  }

  /* 响应式设计 */
  @media (max-width: 768px) {
    .user-message-content {
      max-width: 90%;
    }
  }
</style>
