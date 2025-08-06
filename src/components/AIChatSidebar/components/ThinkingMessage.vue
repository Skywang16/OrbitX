<template>
  <div class="thinking-message">
    <div class="spinner"></div>
    <span class="thinking-text">{{ message.content }}</span>
  </div>
</template>

<script setup lang="ts">
  import type { AgentMessageData } from '../types'

  interface Props {
    message: AgentMessageData
  }

  defineProps<Props>()
</script>

<style scoped>
  .thinking-message {
    display: flex;
    align-items: center;
    padding: 8px 12px;
    font-size: 13px;
    color: var(--color-text-secondary);
  }

  .thinking-text {
    margin-left: 8px;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .spinner {
    border: 2px solid rgba(0, 0, 0, 0.1);
    border-left-color: var(--color-text-secondary);
    border-radius: 50%;
    width: 14px;
    height: 14px;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
