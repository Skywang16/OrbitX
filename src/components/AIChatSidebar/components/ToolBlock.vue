<template>
  <div class="tool-block">
    <div class="tool-header" @click="toggleExpanded">
      <div
        class="tool-icon"
        :class="{
          running: step.metadata?.status === 'running',
          completed: step.metadata?.status === 'completed',
          error: step.metadata?.status === 'error',
        }"
      >
        <span v-if="step.metadata?.status === 'running'">⚙️</span>
        <span v-else-if="step.metadata?.status === 'error'">❌</span>
        <span v-else>✅</span>
      </div>
      <div class="tool-info">
        <div class="tool-name">{{ step.metadata?.toolName || '工具调用' }}</div>
        <div class="tool-command" v-if="step.metadata?.toolCommand">{{ step.metadata.toolCommand }}</div>
        <div class="tool-status">
          <span v-if="step.metadata?.status === 'running'">正在执行...</span>
          <span v-else-if="step.metadata?.status === 'error'">执行失败</span>
          <span v-else>执行完成</span>
        </div>
      </div>
      <div class="tool-toggle">{{ isExpanded ? '▼' : '▶' }}</div>
    </div>

    <div v-if="isExpanded && step.metadata?.toolResult" class="tool-result">
      <div class="tool-result-label">执行结果：</div>
      <div class="tool-result-content">{{ formatResult(step.metadata.toolResult) }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref } from 'vue'

  import type { AIOutputStep } from '@/types/features/ai/chat'

  interface Props {
    step: AIOutputStep
  }

  defineProps<Props>()

  const isExpanded = ref(false)

  const toggleExpanded = () => {
    isExpanded.value = !isExpanded.value
  }

  const formatResult = (result: any) => {
    if (typeof result === 'string') return result
    if (typeof result === 'object') return JSON.stringify(result, null, 2)
    return String(result)
  }
</script>

<style scoped>
  .tool-block {
    padding: var(--spacing-sm);
    border: 1px solid var(--color-border);
    border-radius: var(--border-radius);
    background: var(--color-background-secondary);
    font-size: var(--font-size-sm);
    border-left: 3px solid #3b82f6;
  }

  .tool-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    cursor: pointer;
    user-select: none;
  }

  .tool-header:hover {
    background: var(--color-background-hover);
    margin: calc(var(--spacing-sm) * -1);
    padding: var(--spacing-sm);
    border-radius: var(--border-radius);
  }

  .tool-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    font-size: 14px;
  }

  .tool-icon.running {
    background: #3b82f6;
    color: white;
    animation: pulse 1.5s infinite;
  }

  .tool-icon.completed {
    background: #10b981;
    color: white;
  }

  .tool-icon.error {
    background: #ef4444;
    color: white;
  }

  .tool-info {
    flex: 1;
  }

  .tool-name {
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 2px;
  }

  .tool-command {
    font-family: monospace;
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    background: var(--color-background);
    padding: 2px 6px;
    border-radius: 3px;
    margin-bottom: 2px;
  }

  .tool-status {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
  }

  .tool-toggle {
    font-size: 12px;
    color: var(--text-secondary);
    margin-left: auto;
  }

  .tool-result {
    margin-top: var(--spacing-sm);
    padding: var(--spacing-xs);
    background: var(--color-background);
    border-radius: var(--border-radius);
  }

  .tool-result-label {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    margin-bottom: var(--spacing-xs);
  }

  .tool-result-content {
    font-family: monospace;
    color: var(--text-primary);
    white-space: pre-wrap;
    word-wrap: break-word;
    font-size: var(--font-size-sm);
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
