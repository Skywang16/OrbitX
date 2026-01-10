<template>
  <div v-if="contextUsage" class="context-usage-ring" :title="tooltipText">
    <svg width="24" height="24" viewBox="0 0 24 24" class="ring-svg">
      <!-- Background circle -->
      <circle
        cx="12"
        cy="12"
        r="10"
        stroke="var(--border-300)"
        stroke-width="2"
        fill="none"
        opacity="0.3"
      />
      <!-- Progress circle -->
      <circle
        cx="12"
        cy="12"
        r="10"
        :stroke="ringColor"
        stroke-width="2"
        fill="none"
        stroke-linecap="round"
        :stroke-dasharray="62.8"
        :stroke-dashoffset="62.8 - (percentage / 100) * 62.8"
        transform="rotate(-90 12 12)"
      />
    </svg>
    <div class="percentage-text">{{ Math.round(percentage) }}%</div>
  </div>
</template>

<script setup lang="ts">
  import { computed } from 'vue'
  import type { ContextUsage } from '@/types/domain/aiMessage'

  interface Props {
    contextUsage: ContextUsage | null
  }

  const props = defineProps<Props>()

  const percentage = computed(() => {
    if (!props.contextUsage || props.contextUsage.contextWindow === 0) return 0
    return (props.contextUsage.tokensUsed / props.contextUsage.contextWindow) * 100
  })

  const ringColor = computed(() => {
    const p = percentage.value
    if (p >= 90) return 'var(--color-error)'
    if (p >= 70) return 'var(--color-warning)'
    return 'var(--color-primary)'
  })

  const tooltipText = computed(() => {
    if (!props.contextUsage) return ''
    const { tokensUsed, contextWindow } = props.contextUsage
    return `Context: ${tokensUsed.toLocaleString()} / ${contextWindow.toLocaleString()} tokens (${Math.round(percentage.value)}%)`
  })
</script>

<style scoped>
  .context-usage-ring {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    cursor: help;
  }

  .ring-svg {
    position: absolute;
    top: 0;
    left: 0;
  }

  .percentage-text {
    font-size: 8px;
    font-weight: 600;
    color: var(--text-300);
    user-select: none;
  }
</style>
