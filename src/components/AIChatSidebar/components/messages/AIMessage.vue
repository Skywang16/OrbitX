<script setup lang="ts">
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { marked } from 'marked'
  import { markedHighlight } from 'marked-highlight'
  import hljs from 'highlight.js'
  import type { Message } from '@/types'
  import type { UiStep } from '@/api/agent/types'
  import { useAIChatStore } from '../../store'
  import { formatTime } from '@/utils/dateFormatter'
  import ThinkingBlock from './blocks/ThinkingBlock.vue'
  import ToolBlock from './blocks/ToolBlock.vue'
  import { useStepProcessor } from '@/composables/useStepProcessor'
  const { t } = useI18n()
  const { processSteps } = useStepProcessor()

  marked.use(
    markedHighlight({
      langPrefix: 'hljs language-',
      highlight(code, lang) {
        const language = hljs.getLanguage(lang) ? lang : 'plaintext'
        return hljs.highlight(code, { language }).value
      },
    })
  )

  interface Props {
    message: Message
  }

  const props = defineProps<Props>()
  const aiChatStore = useAIChatStore()

  const sortedSteps = computed(() => {
    if (!props.message.steps) {
      return [] as UiStep[]
    }
    return processSteps(props.message.steps as UiStep[])
  })

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`
    const seconds = (ms / 1000).toFixed(1)
    return `${seconds}s`
  }

  const renderMarkdown = (content?: string) => {
    return marked(content || '')
  }
</script>

<template>
  <div class="ai-message">
    <!-- 双轨架构：只基于steps渲染 -->
    <template v-if="message.steps && message.steps.length > 0">
      <template v-for="(step, index) in sortedSteps" :key="step.metadata?.stepId || `fallback-${index}`">
        <!-- 使用 ThinkingBlock 组件 -->
        <ThinkingBlock
          v-if="step.stepType === 'thinking'"
          :step="step"
          :is-streaming="message.status === 'streaming' || !message.status"
        />

        <!-- 使用 ToolBlock 组件 -->
        <ToolBlock v-else-if="step.stepType === 'tool_use' || step.stepType === 'tool_result'" :step="step" />

        <!-- 文本消息直接渲染 -->
        <div v-else-if="step.stepType === 'text'" class="ai-message-text step-block">
          <div v-html="renderMarkdown(step.content)"></div>
        </div>

        <!-- 错误消息 -->
        <div v-else-if="step.stepType === 'error'" class="error-output step-block">
          <div class="error-content">{{ step.content }}</div>
        </div>

        <!-- 未知步骤类型 -->
        <div v-else class="unknown-step step-block">
          <div class="unknown-header">
            <span class="unknown-icon">❓</span>
            <span class="unknown-label">未知步骤类型: {{ step.stepType }}</span>
          </div>
          <div class="unknown-content">{{ step.content }}</div>
        </div>
      </template>
    </template>

    <div class="ai-message-footer">
      <div class="ai-message-time">{{ formatTime(message.createdAt) }}</div>

      <div
        v-if="
          aiChatStore.isLoading &&
          message.role === 'assistant' &&
          aiChatStore.messageList[aiChatStore.messageList.length - 1]?.id === message.id
        "
        class="streaming-indicator"
      >
        <span class="streaming-dot"></span>
        {{ t('message.generating') }}
      </div>

      <div v-else-if="message.duration" class="duration-info">
        {{ t('message.duration_info', { duration: formatDuration(message.duration) }) }}
      </div>
    </div>
  </div>
</template>

<style scoped>
  .ai-message {
    margin-bottom: var(--spacing-md);
    width: 100%;
  }

  .ai-message-text {
    width: 100%;
    padding: var(--spacing-sm) 0;
    font-size: var(--font-size-md);
    line-height: 1.5;
    color: var(--text-200);
    word-wrap: break-word;
    word-break: break-word;
  }

  .ai-message-text :deep(h1),
  .ai-message-text :deep(h2),
  .ai-message-text :deep(h3) {
    margin: 0.8em 0 0.5em 0;
    font-weight: 600;
    color: var(--text-100);
    line-height: 1.3;
  }

  .ai-message-text :deep(h1) {
    font-size: 1.5em;
    border-bottom: 2px solid var(--border-300);
    padding-bottom: 0.3em;
  }

  .ai-message-text :deep(h2) {
    font-size: 1.3em;
    border-bottom: 1px solid var(--border-200);
    padding-bottom: 0.2em;
  }

  .ai-message-text :deep(h3) {
    font-size: 1.15em;
  }

  .ai-message-text :deep(p) {
    margin: 0.5em 0;
  }

  .ai-message-text :deep(code) {
    background: var(--bg-500);
    padding: 0.2em 0.4em;
    border-radius: var(--border-radius-xs);
    font-family: var(--font-family-mono);
    font-size: 0.9em;
    color: var(--syntax-string);
    border: 1px solid var(--border-300);
  }

  .ai-message-text :deep(pre) {
    background: var(--bg-500);
    padding: var(--spacing-md);
    border-radius: var(--border-radius);
    overflow-x: auto;
    margin: 0.8em 0;
    border: 1px solid var(--border-300);
    box-shadow: var(--shadow-sm);
  }

  .ai-message-text :deep(pre code) {
    background: none;
    padding: 0;
    border: none;
    color: var(--text-200);
    font-size: var(--font-size-sm);
    line-height: 1.6;
  }

  .ai-message-text :deep(ul),
  .ai-message-text :deep(ol) {
    margin: 0.8em 0;
    padding-left: 1.8em;
  }

  .ai-message-text :deep(li) {
    margin: 0.3em 0;
    line-height: 1.6;
  }

  .ai-message-text :deep(li::marker) {
    color: var(--color-primary);
  }

  .ai-message-text :deep(a) {
    color: var(--color-primary);
    text-decoration: none;
    border-bottom: 1px solid transparent;
    transition: all 0.2s;
  }

  .ai-message-text :deep(a:hover) {
    border-bottom-color: var(--color-primary);
    filter: brightness(1.2);
  }

  .ai-message-text :deep(blockquote) {
    margin: 0.8em 0;
    padding: var(--spacing-sm) var(--spacing-md);
    border-left: 3px solid var(--color-primary);
    background: var(--bg-400);
    border-radius: var(--border-radius-xs);
    color: var(--text-300);
  }

  .ai-message-text :deep(blockquote p) {
    margin: 0.3em 0;
  }

  .ai-message-text :deep(hr) {
    margin: 1em 0;
    border: none;
    border-top: 2px solid var(--border-300);
  }

  .ai-message-text :deep(table) {
    border-collapse: collapse;
    width: 100%;
    margin: 0.8em 0;
    font-size: var(--font-size-sm);
  }

  .ai-message-text :deep(th),
  .ai-message-text :deep(td) {
    border: 1px solid var(--border-300);
    padding: var(--spacing-sm) var(--spacing-md);
    text-align: left;
  }

  .ai-message-text :deep(th) {
    background: var(--bg-500);
    font-weight: 600;
    color: var(--text-100);
  }

  .ai-message-text :deep(tr:hover) {
    background: var(--bg-400);
  }

  .ai-message-text :deep(img) {
    max-width: 100%;
    height: auto;
    border-radius: var(--border-radius);
    margin: 0.5em 0;
  }

  .ai-message-text :deep(strong) {
    font-weight: 600;
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
</style>
