<script setup lang="ts">
  import type { Message } from '@/types'

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
    background: var(--color-primary-alpha);
    color: var(--text-100);
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--border-radius-lg);
    word-wrap: break-word;
    word-break: break-word;
    white-space: pre-wrap;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
  }

  .user-message-text {
    font-size: var(--font-size-md);
    line-height: 1.4;
    margin: 0;
  }

  .user-message-time {
    font-size: var(--font-size-xs);
    color: var(--text-400);
    margin-top: var(--spacing-xs);
    margin-right: var(--spacing-sm);
  }
</style>
