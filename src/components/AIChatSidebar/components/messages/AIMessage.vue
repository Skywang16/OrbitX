<script setup lang="ts">
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import type { Message } from '@/types'
  import { formatTime } from '@/utils/dateFormatter'
  import { renderMarkdown } from '@/utils/markdown'
  import ThinkingBlock from './blocks/ThinkingBlock.vue'
  import ToolBlock from './blocks/ToolBlock.vue'
  const { t } = useI18n()

  interface Props {
    message: Message
  }

  const props = defineProps<Props>()

  const blocks = computed(() => props.message.blocks)

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`
    const seconds = (ms / 1000).toFixed(1)
    return `${seconds}s`
  }
</script>

<template>
  <div class="ai-message">
    <template v-if="blocks.length > 0">
      <template
        v-for="(block, index) in blocks"
        :key="('id' in block && block.id) || `${message.id}-${block.type}-${index}`"
      >
        <ThinkingBlock v-if="block.type === 'thinking'" :block="block" />

        <ToolBlock v-else-if="block.type === 'tool'" :block="block" />

        <div v-else-if="block.type === 'text'" class="ai-message-text step-block">
          <div v-html="renderMarkdown(block.content)"></div>
        </div>

        <div v-else-if="block.type === 'error'" class="error-output step-block">
          <div class="error-content">{{ block.message }}</div>
        </div>

        <div v-else class="unknown-step step-block">
          <div class="unknown-header">
            <span class="unknown-icon">❓</span>
            <span class="unknown-label">未知块类型: {{ block.type }}</span>
          </div>
        </div>
      </template>
    </template>

    <div class="ai-message-footer">
      <div class="ai-message-time">{{ formatTime(message.createdAt) }}</div>

      <div v-if="message.status === 'streaming'" class="streaming-indicator">
        <span class="streaming-dot"></span>
        {{ t('message.generating') }}
      </div>

      <div v-else-if="message.durationMs" class="duration-info">
        {{ t('message.duration_info', { duration: formatDuration(message.durationMs) }) }}
      </div>
    </div>
  </div>
</template>

<style scoped>
  .ai-message {
    margin-bottom: var(--spacing-md);
    width: 100%;
    min-width: 0;
    overflow: hidden;
  }

  .step-block {
    margin-bottom: var(--spacing-sm);
  }

  .step-block:last-of-type {
    margin-bottom: 0;
  }

  .error-output {
    border: 1px solid var(--color-error);
    border-radius: var(--border-radius);
    background: var(--bg-300);
  }

  .error-content {
    padding: var(--spacing-md);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    color: var(--color-error);
  }

  .unknown-step {
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius);
    background: var(--bg-300);
    opacity: 0.7;
  }

  .unknown-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    padding: var(--spacing-sm) var(--spacing-md);
    background: var(--bg-400);
    border-bottom: 1px solid var(--border-200);
    border-radius: var(--border-radius) var(--border-radius) 0 0;
  }

  .unknown-icon {
    font-size: var(--font-size-sm);
  }

  .unknown-label {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-400);
  }

  .unknown-content {
    padding: var(--spacing-md);
    font-size: var(--font-size-sm);
    color: var(--text-400);
  }

  .ai-message-footer {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    margin-top: var(--spacing-sm);
    padding-top: var(--spacing-xs);
    border-top: 1px solid var(--border-200);
  }

  .ai-message-time {
    font-size: var(--font-size-xs);
    color: var(--text-400);
  }

  .streaming-indicator {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    font-size: var(--font-size-xs);
    color: var(--text-400);
  }

  .streaming-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-primary);
    animation: pulse 1.5s infinite;
  }

  .duration-info {
    font-size: var(--font-size-xs);
    color: var(--text-500);
    font-family: monospace;
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

  .ai-message-text {
    font-size: 0.9em;
    line-height: 1.6;
    color: var(--text-200);
    overflow: hidden;
    min-width: 0;
  }

  .ai-message-text :deep(p) {
    margin: var(--spacing-sm) 0;
    font-size: 0.95em;
  }

  .ai-message-text :deep(p:first-child) {
    margin-top: 0;
  }

  .ai-message-text :deep(p:last-child) {
    margin-bottom: 0;
  }

  /* 标题 */
  .ai-message-text :deep(h1),
  .ai-message-text :deep(h2),
  .ai-message-text :deep(h3),
  .ai-message-text :deep(h4),
  .ai-message-text :deep(h5),
  .ai-message-text :deep(h6) {
    margin: var(--spacing-md) 0 var(--spacing-sm) 0;
    font-weight: 600;
    line-height: 1.4;
    color: var(--text-100);
  }

  .ai-message-text :deep(h1) {
    font-size: 1.3em;
    border-bottom: 1px solid var(--border-200);
    padding-bottom: var(--spacing-xs);
  }

  .ai-message-text :deep(h2) {
    font-size: 1.15em;
  }

  .ai-message-text :deep(h3) {
    font-size: 1.05em;
  }

  .ai-message-text :deep(h4) {
    font-size: 0.95em;
  }

  /* 代码 */
  .ai-message-text :deep(code) {
    font-family: var(--font-mono);
    font-size: 0.85em;
    padding: 0.15em 0.4em;
    background: var(--bg-400);
    border-radius: 3px;
    color: var(--text-200);
  }

  .ai-message-text :deep(pre) {
    margin: var(--spacing-sm) 0;
    overflow-x: auto;
    max-width: 100%;
  }

  .ai-message-text :deep(pre code) {
    padding: 0;
    background: transparent;
    font-size: 0.9em;
  }

  /* 列表 */
  .ai-message-text :deep(ul),
  .ai-message-text :deep(ol) {
    margin: var(--spacing-sm) 0;
    padding-left: 1.5em;
  }

  .ai-message-text :deep(li) {
    margin: var(--spacing-xs) 0;
  }

  .ai-message-text :deep(li > p) {
    margin: var(--spacing-xs) 0;
  }

  /* 引用 */
  .ai-message-text :deep(blockquote) {
    margin: var(--spacing-sm) 0;
    padding: var(--spacing-xs) var(--spacing-md);
    border-left: 3px solid var(--color-primary);
    background: var(--bg-400);
    color: var(--text-300);
  }

  .ai-message-text :deep(blockquote p) {
    margin: var(--spacing-xs) 0;
  }

  /* 链接 */
  .ai-message-text :deep(a) {
    color: var(--color-primary);
    text-decoration: none;
  }

  .ai-message-text :deep(a:hover) {
    text-decoration: underline;
  }

  /* 分隔线 */
  .ai-message-text :deep(hr) {
    margin: var(--spacing-md) 0;
    border: none;
    border-top: 1px solid var(--border-200);
  }

  /* 表格 */
  .ai-message-text :deep(table) {
    margin: var(--spacing-sm) 0;
    border-collapse: collapse;
    width: 100%;
    max-width: 100%;
    display: block;
    overflow-x: auto;
  }

  .ai-message-text :deep(th),
  .ai-message-text :deep(td) {
    padding: var(--spacing-xs) var(--spacing-sm);
    border: 1px solid var(--border-200);
    text-align: left;
  }

  .ai-message-text :deep(th) {
    background: var(--bg-400);
    font-weight: 600;
  }

  .ai-message-text :deep(tr:nth-child(even)) {
    background: var(--bg-300);
  }

  /* 图片 */
  .ai-message-text :deep(img) {
    max-width: 100%;
    height: auto;
    border-radius: var(--border-radius);
  }

  /* 强调 */
  .ai-message-text :deep(strong) {
    font-weight: 600;
    color: var(--text-100);
  }

  .ai-message-text :deep(em) {
    font-style: italic;
  }
</style>
