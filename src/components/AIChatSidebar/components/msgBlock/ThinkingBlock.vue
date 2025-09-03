<template>
  <div class="thinking-block">
    <div class="thinking-header" @click="toggleExpanded">
      <svg class="thinking-arrow" :class="{ expanded: isExpanded }" width="12" height="12" viewBox="0 0 24 24">
        <path
          d="M9 18l6-6-6-6"
          stroke="currentColor"
          stroke-width="2"
          fill="none"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
      <span class="thinking-text">Think...</span>
    </div>

    <div v-if="isExpanded" class="thinking-content">
      <div class="thinking-text-content">{{ step.content }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref } from 'vue'

  import type { AIOutputStep } from '@/types'

  interface Props {
    step: AIOutputStep
  }

  defineProps<Props>()

  const isExpanded = ref(false)

  const toggleExpanded = () => {
    isExpanded.value = !isExpanded.value
  }
</script>

<style scoped>
  .thinking-block {
    margin-bottom: var(--spacing-xs, 4px);
  }

  .thinking-header {
    display: inline-flex;
    align-items: center;
    cursor: pointer;
    user-select: none;
  }

  .thinking-text {
    font-size: var(--font-size-sm, 14px);
    color: var(--text-400, #666);
  }

  .thinking-arrow {
    color: var(--text-400, #666);
    transition: transform 0.2s ease;
    flex-shrink: 0;
    margin-right: 6px;
  }

  .thinking-arrow.expanded {
    transform: rotate(90deg);
  }

  .thinking-content {
    margin-top: var(--spacing-xs, 4px);
    margin-left: 16px;
    padding: var(--spacing-sm, 8px);
    border-left: 2px solid var(--border-200, #e5e5e5);
  }

  .thinking-text-content {
    font-size: var(--font-size-sm, 14px);
    line-height: 1.5;
    color: var(--text-400, #666);
    white-space: pre-wrap;
    word-wrap: break-word;
  }
</style>
