<template>
  <div class="thinking-block">
    <div class="thinking-header" @click="toggleExpanded">
      <div class="thinking-title">Thinking...</div>
      <div class="thinking-timer">{{ formatTime(elapsedTime) }}</div>
      <div class="thinking-toggle">{{ isExpanded ? '▼' : '▶' }}</div>
    </div>

    <div v-if="isExpanded" class="thinking-content">
      <div class="thinking-text">{{ step.content }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, onMounted, onUnmounted, computed } from 'vue'

  import type { AIOutputStep } from '@/types/features/ai/chat'

  interface Props {
    step: AIOutputStep
  }

  const props = defineProps<Props>()

  const isExpanded = ref(false)
  const currentTime = ref(Date.now())
  let timer: number | null = null

  const elapsedTime = computed(() => {
    // 如果有持续时间，直接使用；否则实时计算
    return props.step.metadata?.thinkingDuration || currentTime.value - props.step.timestamp
  })

  const formatTime = (ms: number) => {
    const seconds = Math.floor(ms / 1000)
    return `${seconds}s`
  }

  const toggleExpanded = () => {
    isExpanded.value = !isExpanded.value
  }

  onMounted(() => {
    // 只有在没有固定持续时间时才启动计时器
    if (!props.step.metadata?.thinkingDuration) {
      timer = window.setInterval(() => {
        currentTime.value = Date.now()
      }, 1000)
    }
  })

  onUnmounted(() => {
    if (timer) {
      clearInterval(timer)
    }
  })
</script>

<style scoped>
  .thinking-block {
    margin-bottom: var(--spacing-sm);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius);
    background: var(--bg-400);
    opacity: 0.8;
  }

  .thinking-header {
    display: flex;
    align-items: center;
    padding: var(--spacing-sm) var(--spacing-md);
    cursor: pointer;
    user-select: none;
    gap: var(--spacing-sm);
  }

  .thinking-header:hover {
    background: var(--color-hover);
  }

  .thinking-title {
    flex: 1;
    font-size: var(--font-size-sm);
    color: var(--text-400);
    font-style: italic;
  }

  .thinking-timer {
    font-size: var(--font-size-xs);
    color: var(--text-500);
    font-family: monospace;
  }

  .thinking-toggle {
    font-size: 12px;
    color: var(--text-400);
  }

  .thinking-content {
    border-top: 1px solid var(--border-300);
    padding: var(--spacing-md);
  }

  .thinking-text {
    font-size: var(--font-size-sm);
    line-height: 1.5;
    color: var(--text-400);
    white-space: pre-wrap;
    word-wrap: break-word;
  }
</style>
