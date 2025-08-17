<script setup lang="ts">
  import { computed } from 'vue'
  import { marked } from 'marked'
  import type { Message } from '@/types/features/ai/chat'
  import ThinkingBlock from './ThinkingBlock.vue'
  import ToolBlock from './ToolBlock.vue'

  interface Props {
    message: Message
  }

  const props = defineProps<Props>()

  // 按时间戳排序步骤，确保瀑布式显示顺序
  const sortedSteps = computed(() => {
    if (!props.message.steps) {
      console.log('🔍 [AIMessage] 消息没有steps:', { messageId: props.message.id, role: props.message.role })
      return []
    }

    const sorted = [...props.message.steps].sort((a, b) => a.timestamp - b.timestamp)
    console.log('🔍 [AIMessage] 渲染steps:', {
      messageId: props.message.id,
      totalSteps: props.message.steps.length,
      toolSteps: props.message.steps.filter(s => s.type === 'tool_use').length,
      thinkingSteps: props.message.steps.filter(s => s.type === 'thinking').length,
      textSteps: props.message.steps.filter(s => s.type === 'text').length,
      steps: props.message.steps,
    })

    return sorted
  })

  import { formatTime } from '@/utils/dateFormatter'

  // 格式化持续时间
  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`
    const seconds = (ms / 1000).toFixed(1)
    return `${seconds}s`
  }

  // 渲染markdown
  const renderMarkdown = (content: string) => {
    return marked(content)
  }
</script>

<template>
  <div class="ai-message">
    <!-- 瀑布式渲染所有步骤 -->
    <template v-if="message.steps && message.steps.length > 0">
      <!-- 按时间戳排序确保正确的显示顺序 -->
      <template v-for="(step, index) in sortedSteps" :key="`${step.timestamp}-${index}`">
        <!-- 思考块：可折叠，带计时器 -->
        <ThinkingBlock v-if="step.type === 'thinking'" :step="step" class="step-block" />

        <!-- 工具调用块 -->
        <ToolBlock v-else-if="step.type === 'tool_use'" :step="step" class="step-block" />

        <!-- 错误块 -->
        <div v-else-if="step.type === 'error'" class="error-block step-block">
          <div class="error-header">{{ step.metadata?.errorType || '错误' }}</div>
          <div class="error-content">{{ step.content }}</div>
          <div v-if="step.metadata?.errorDetails" class="error-details">
            {{ step.metadata.errorDetails }}
          </div>
        </div>

        <!-- AI文本回复 -->
        <div v-else-if="step.type === 'text'" class="ai-message-text step-block">
          <div v-html="renderMarkdown(step.content)"></div>
        </div>
      </template>
    </template>

    <!-- 回退渲染：无步骤数据时，直接渲染消息内容（用于从数据库加载的历史AI消息） -->
    <div v-else-if="message.content" class="ai-message-text">
      <div v-html="renderMarkdown(message.content)"></div>
    </div>

    <!-- 消息时间和状态 -->
    <div class="ai-message-footer">
      <div class="ai-message-time">{{ formatTime(message.createdAt) }}</div>
      <div v-if="message.status === 'streaming'" class="streaming-indicator">
        <span class="streaming-dot"></span>
        正在生成...
      </div>
      <div v-else-if="message.duration" class="duration-info">耗时 {{ formatDuration(message.duration) }}</div>
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

  /* Markdown样式 */
  .ai-message-text :deep(h1),
  .ai-message-text :deep(h2),
  .ai-message-text :deep(h3) {
    margin: 0.5em 0;
    font-weight: 600;
  }

  .ai-message-text :deep(p) {
    margin: 0.5em 0;
  }

  .ai-message-text :deep(code) {
    background: var(--bg-500);
    padding: 0.2em 0.4em;
    border-radius: 3px;
    font-family: monospace;
    font-size: 0.9em;
  }

  .ai-message-text :deep(pre) {
    background: var(--bg-500);
    padding: var(--spacing-sm);
    border-radius: var(--border-radius);
    overflow-x: auto;
    margin: 0.5em 0;
  }

  .ai-message-text :deep(pre code) {
    background: none;
    padding: 0;
  }

  .ai-message-text :deep(ul),
  .ai-message-text :deep(ol) {
    margin: 0.5em 0;
    padding-left: 1.5em;
  }

  .ai-message-text :deep(strong) {
    font-weight: 600;
  }

  /* 步骤块通用样式 */
  .step-block {
    margin-bottom: var(--spacing-sm);
  }

  .step-block:last-of-type {
    margin-bottom: 0;
  }

  /* 错误块 */
  .error-block {
    padding: var(--spacing-sm);
    border: 1px solid var(--color-error);
    border-left: 3px solid var(--color-error);
    border-radius: var(--border-radius);
    background: var(--color-error);
    opacity: 0.1;
    font-size: var(--font-size-sm);
  }

  .error-header {
    font-weight: 600;
    margin-bottom: var(--spacing-xs);
    color: var(--color-error);
  }

  .error-content,
  .error-details {
    color: var(--color-error);
    margin-bottom: var(--spacing-xs);
  }

  .error-details {
    font-size: var(--font-size-xs);
    opacity: 0.8;
  }

  /* 消息底部信息 */
  .ai-message-footer {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    margin-top: var(--spacing-sm);
    padding-top: var(--spacing-xs);
    border-top: 1px solid var(--border-300);
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
