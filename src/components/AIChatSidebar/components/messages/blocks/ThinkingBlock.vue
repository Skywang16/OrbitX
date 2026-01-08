<template>
  <div class="thinking-block">
    <div class="thinking-line" :class="{ running: block.isStreaming }">
      <span class="text clickable" @click="toggleExpanded">
        <span class="thinking-prefix">Thought</span>
      </span>
      <svg
        class="chevron"
        :class="{ expanded: isExpanded }"
        width="10"
        height="10"
        viewBox="0 0 10 10"
        @click="toggleExpanded"
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
      <div v-if="isExpanded" class="thinking-result">
        <div class="result-wrapper">
          <pre class="result-text-plain">{{ block.content }}</pre>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
  import { ref, watch } from 'vue'
  import type { Block } from '@/types'

  interface Props {
    block: Extract<Block, { type: 'thinking' }>
  }

  const props = defineProps<Props>()

  const isExpanded = ref(props.block.isStreaming)

  watch(
    () => props.block.isStreaming,
    isStreaming => {
      isExpanded.value = isStreaming
    }
  )

  const toggleExpanded = () => {
    isExpanded.value = !isExpanded.value
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
    padding: 4px 0;
    color: var(--text-400);
    transition: all 0.15s ease;
    font-size: 14px;
  }

  .thinking-line.running .text {
    opacity: 0.6;
  }

  .text {
    font-size: 14px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .text.clickable {
    cursor: pointer;
  }

  .text.clickable:hover {
    color: var(--text-300);
  }

  .thinking-prefix {
    color: var(--text-400);
    font-weight: 400;
  }

  .chevron {
    flex-shrink: 0;
    color: var(--text-500);
    transition: transform 0.2s ease;
    opacity: 0.5;
    cursor: pointer;
  }

  .chevron:hover,
  .text.clickable:hover ~ .chevron {
    opacity: 1;
  }

  .chevron.expanded {
    transform: rotate(90deg);
  }

  .thinking-result {
    margin-top: 8px;
    margin-left: 0;
    position: relative;
    max-height: 300px;
    overflow: hidden;
  }

  .result-wrapper {
    max-height: 300px;
    overflow-y: auto;
    overflow-x: auto;
    padding: 0;
    scrollbar-gutter: stable;
  }

  /* 和 MessageList 一致的滚动条样式 */
  .result-wrapper::-webkit-scrollbar {
    width: 8px;
  }

  .result-wrapper::-webkit-scrollbar-track {
    background: var(--bg-200);
    border-radius: 4px;
  }

  .result-wrapper::-webkit-scrollbar-thumb {
    background: var(--border-300);
    border-radius: 4px;
    transition: background-color 0.2s ease;
  }

  .result-wrapper::-webkit-scrollbar-thumb:hover {
    background: var(--border-400);
  }

  .result-text-plain {
    margin: 0;
    padding: 0;
    font-family: 'SF Mono', 'Monaco', 'Menlo', 'Consolas', monospace;
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-400);
    white-space: pre-wrap;
    word-wrap: break-word;
    overflow-wrap: break-word;
    background: transparent;
  }

  /* 展开动画 */
  .expand-enter-active,
  .expand-leave-active {
    transition: all 0.2s ease;
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
    margin-top: 8px;
  }
</style>
