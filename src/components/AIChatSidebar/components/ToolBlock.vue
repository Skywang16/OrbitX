<template>
  <div class="tool-block" @click="toggleExpanded">
    <div class="tool-header">
      <div class="tool-info">
        <span class="tool-name">{{ step.metadata?.toolName || '工具调用' }}</span>
        <span class="tool-command" v-if="step.metadata?.toolCommand">{{ step.metadata.toolCommand }}</span>
      </div>
      <div
        class="status-dot"
        :class="{
          running: step.metadata?.status === 'running',
          error: step.metadata?.status === 'error',
        }"
      ></div>
    </div>

    <div v-if="isExpanded && step.metadata?.toolResult" class="tool-result">
      <div class="tool-result-content">{{ formatResult(step.metadata.toolResult) }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref } from 'vue'
  import type { AIOutputStep } from '@/types/features/ai/chat'

  defineProps<{
    step: AIOutputStep
  }>()

  const isExpanded = ref(false)

  const toggleExpanded = () => {
    isExpanded.value = !isExpanded.value
  }

  const formatResult = (result: unknown) => {
    if (typeof result === 'string') return result
    if (typeof result === 'object') return JSON.stringify(result, null, 2)
    return String(result)
  }
</script>

<style scoped>
  .tool-block {
    padding: 8px 12px;
    background: var(--bg-500);
    border: 1px solid var(--border-300);
    border-radius: 6px;
    font-size: 13px;
    cursor: pointer;
    transition: background-color 0.2s ease;
    max-width: 100%;
  }

  .tool-block:hover {
    background: var(--bg-600);
  }

  .tool-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .tool-info {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0; /* 允许flex子元素收缩 */
  }

  .tool-name {
    color: var(--text-200);
    font-weight: 500;
    white-space: nowrap;
  }

  .tool-command {
    color: var(--text-400);
    font-family: monospace;
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-success);
  }

  .status-dot.running {
    background: var(--color-info);
    animation: pulse 1.5s infinite;
  }

  .status-dot.error {
    background: var(--color-error);
  }

  .tool-result {
    margin-top: 8px;
    padding: 8px;
    background: var(--bg-400);
    border-radius: 4px;
  }

  .tool-result-content {
    font-family: monospace;
    color: var(--text-300);
    white-space: pre-wrap;
    word-wrap: break-word;
    font-size: 12px;
    line-height: 1.4;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.5;
    }
  }
</style>
