<template>
  <div class="thinking-block">
    <div class="thinking-line" :class="{ clickable: hasContent, running: block.isStreaming }" @click="toggleExpanded">
      <span class="text" :class="{ running: block.isStreaming }">
        <span class="thinking-content">{{ displayText }}</span>
      </span>
      <svg
        v-if="hasContent"
        class="chevron"
        :class="{ expanded: isExpanded }"
        width="10"
        height="10"
        viewBox="0 0 10 10"
      >
        <path
          d="M3.5 2.5L6 5L3.5 7.5"
          stroke="currentColor"
          stroke-width="1"
          stroke-linecap="round"
          stroke-linejoin="round"
          fill="none"
        />
      </svg>
    </div>

    <transition name="expand">
      <div v-if="isExpanded && hasContent" class="thinking-result">
        <div class="result-wrapper">
          <pre class="result-text">{{ block.content }}</pre>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
  import type { Block } from '@/types'
  import { computed, onMounted, onUnmounted, ref, watch } from 'vue'

  interface Props {
    block: Extract<Block, { type: 'thinking' }>
    disableExpand?: boolean
  }

  const props = defineProps<Props>()

  const isExpanded = ref(false)
  const hasContent = computed(() => !props.disableExpand && Boolean(props.block.content))

  const displayText = computed(() => {
    return 'Thinking'
  })
  const elapsedSeconds = ref(0)
  const startTime = ref<number | null>(null)
  let timer: ReturnType<typeof setInterval> | null = null

  const startTimer = () => {
    if (timer) return
    startTime.value = Date.now()
    timer = setInterval(() => {
      if (startTime.value) {
        elapsedSeconds.value = Math.floor((Date.now() - startTime.value) / 1000)
      }
    }, 1000)
  }

  const stopTimer = () => {
    if (timer) {
      clearInterval(timer)
      timer = null
    }
  }

  watch(
    () => props.block.isStreaming,
    isStreaming => {
      if (isStreaming) {
        startTimer()
      } else {
        stopTimer()
      }
    },
    { immediate: true }
  )

  onMounted(() => {
    if (props.block.isStreaming) {
      startTimer()
    }
  })

  onUnmounted(() => {
    stopTimer()
  })

  const toggleExpanded = () => {
    if (hasContent.value) {
      isExpanded.value = !isExpanded.value
    }
  }
</script>

<style scoped>
  .thinking-block {
    margin: 6px 0;
    font-size: 14px;
    line-height: 1.8;
  }

  .thinking-line {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--text-400);
    transition: all 0.15s ease;
  }

  .thinking-line.clickable {
    cursor: pointer;
  }

  .thinking-line.clickable:hover {
    color: var(--text-300);
  }

  .thinking-line.clickable:hover .chevron {
    opacity: 1;
  }

  .thinking-line.running .text,
  .text.running {
    background: linear-gradient(
      90deg,
      var(--text-500) 0%,
      var(--text-500) 25%,
      var(--text-200) 50%,
      var(--text-500) 75%,
      var(--text-500) 100%
    );
    background-size: 300% 100%;
    background-clip: text;
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    animation: scan 2s linear infinite;
  }

  @keyframes scan {
    0% {
      background-position: 100% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }

  .text {
    font-size: 14px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .thinking-content {
    color: var(--text-500);
    font-weight: 500;
  }

  .chevron {
    flex-shrink: 0;
    color: var(--text-500);
    transition: transform 0.2s ease;
    opacity: 0.5;
  }

  .chevron.expanded {
    transform: rotate(90deg);
  }

  .thinking-result {
    margin-top: 2px;
    max-height: 300px;
    overflow: hidden;
  }

  .result-wrapper {
    max-height: 300px;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 0;
    scrollbar-width: none;
  }

  .result-wrapper::-webkit-scrollbar {
    display: none;
  }

  .result-text {
    margin: 0;
    padding: 0;
    font-family: var(--font-family-mono);
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-400);
    white-space: pre-wrap;
    word-wrap: break-word;
    overflow-wrap: break-word;
    background: transparent;
  }

  .expand-enter-active,
  .expand-leave-active {
    transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
    overflow: hidden;
  }

  .expand-enter-from,
  .expand-leave-to {
    max-height: 0;
    opacity: 0;
    margin-top: 0;
  }

  .expand-enter-to,
  .expand-leave-from {
    max-height: 300px;
    opacity: 1;
    margin-top: 4px;
  }
</style>
