<template>
  <div class="thinking-block">
    <div class="thinking-header" @click="toggleExpanded">
      <div class="thinking-title">Thinking...</div>
      <div class="thinking-timer">{{ formatTime(elapsedTime) }}</div>
    </div>

    <div v-if="isExpanded" class="thinking-content">
      <div class="thinking-text">{{ step.content }}</div>
    </div>

    <!-- 正在思考时：显示若隐若现的内容预览 -->
    <div v-else-if="!isThinkingComplete && step.content" class="thinking-preview">
      <div class="thinking-text-preview">{{ step.content }}</div>
      <div class="thinking-gradient-top"></div>
      <div class="thinking-gradient-bottom"></div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, onMounted, onUnmounted, computed } from 'vue'

  import type { AIOutputStep } from '@/types'

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

  // 判断思考是否已完成
  const isThinkingComplete = computed(() => {
    return Boolean(props.step.metadata?.thinkingDuration)
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
    border-radius: var(--border-radius);
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

  .thinking-preview {
    position: relative;
    height: 60px;
    overflow: hidden;
    padding: var(--spacing-md);
    padding-top: 0;
  }

  .thinking-text-preview {
    font-size: var(--font-size-sm);
    line-height: 1.5;
    color: var(--text-400);
    white-space: pre-wrap;
    word-wrap: break-word;
    opacity: 0.7;
  }

  .thinking-gradient-top {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 20px;
    background: linear-gradient(to bottom, var(--bg-400), transparent);
    pointer-events: none;
  }

  .thinking-gradient-bottom {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 20px;
    background: linear-gradient(to top, var(--bg-400), transparent);
    pointer-events: none;
  }
</style>
