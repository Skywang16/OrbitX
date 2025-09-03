<script setup lang="ts">
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { marked } from 'marked'
  import type { Message } from '@/types'
  import ThinkingBlock from './msgBlock/ThinkingBlock.vue'
  import ToolBlock from './msgBlock/ToolBlock.vue'
  import { useAIChatStore } from '../store'

  const { t } = useI18n()

  interface Props {
    message: Message
  }

  const props = defineProps<Props>()
  const aiChatStore = useAIChatStore()

  // 按时间戳排序确保正确的显示顺序
  const sortedSteps = computed(() => {
    if (!props.message.steps) {
      return []
    }

    return props.message.steps
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

        <!-- AI文本回复 -->
        <div v-else-if="step.type === 'text'" class="ai-message-text step-block">
          <div v-html="renderMarkdown(step.content)"></div>
        </div>

        <!-- 文件输出 -->
        <div v-else-if="(step as any).type === 'file'" class="file-output step-block">
          <div class="file-header">
            <span class="file-icon">📁</span>
            <span class="file-label">文件输出</span>
          </div>
          <div class="file-content">{{ (step as any).content }}</div>
        </div>

        <!-- 错误信息 -->
        <div v-else-if="step.type === 'error'" class="error-output step-block">
          <div class="error-header">
            <span class="error-icon">❌</span>
            <span class="error-label">执行错误</span>
          </div>
          <div class="error-content">{{ step.content }}</div>
        </div>

        <!-- 任务事件 -->
        <div v-else-if="step.type === 'task'" class="task-output step-block">
          <div class="task-header">
            <span class="task-icon">📋</span>
            <span class="task-label">任务事件</span>
          </div>
          <div class="task-content">{{ step.content }}</div>
        </div>

        <!-- 未知类型的步骤 -->
        <div v-else class="unknown-step step-block">
          <div class="unknown-header">
            <span class="unknown-icon">❓</span>
            <span class="unknown-label">未知步骤类型: {{ (step as any).type }}</span>
          </div>
          <div class="unknown-content">{{ (step as any).content }}</div>
        </div>
      </template>
    </template>

    <!-- 消息时间和状态 -->
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

  /* 错误消息样式 */
  .error-message {
    padding: var(--spacing-sm);
    border-radius: var(--border-radius);
    background: rgba(239, 68, 68, 0.05);
    border: 1px solid rgba(239, 68, 68, 0.2);
    font-size: var(--font-size-sm);
  }

  .error-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    font-weight: 600;
    margin-bottom: var(--spacing-xs);
    color: var(--color-error);
  }

  .error-icon {
    font-size: var(--font-size-md);
  }

  .error-title {
    font-size: var(--font-size-sm);
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

  /* 文件输出样式 */
  .file-output {
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius);
    background: var(--bg-300);
  }

  .file-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    padding: var(--spacing-sm) var(--spacing-md);
    background: var(--bg-400);
    border-bottom: 1px solid var(--border-300);
    border-radius: var(--border-radius) var(--border-radius) 0 0;
  }

  .file-icon {
    font-size: var(--font-size-sm);
  }

  .file-label {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-400);
  }

  .file-content {
    padding: var(--spacing-md);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    color: var(--text-300);
  }

  /* 错误输出样式 */
  .error-output {
    border: 1px solid var(--color-error);
    border-radius: var(--border-radius);
    background: var(--bg-300);
  }

  .error-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    padding: var(--spacing-sm) var(--spacing-md);
    background: rgba(var(--color-error-rgb), 0.1);
    border-bottom: 1px solid var(--color-error);
    border-radius: var(--border-radius) var(--border-radius) 0 0;
  }

  .error-icon {
    font-size: var(--font-size-sm);
  }

  .error-label {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--color-error);
  }

  .error-content {
    padding: var(--spacing-md);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    color: var(--color-error);
  }

  /* 工作流输出样式 */
  .workflow-output {
    border: 1px solid var(--color-primary);
    border-radius: var(--border-radius);
    background: var(--bg-300);
  }

  .workflow-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    padding: var(--spacing-sm) var(--spacing-md);
    background: rgba(var(--color-primary-rgb), 0.1);
    border-bottom: 1px solid var(--color-primary);
    border-radius: var(--border-radius) var(--border-radius) 0 0;
  }

  .workflow-icon {
    font-size: var(--font-size-sm);
  }

  .workflow-label {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--color-primary);
  }

  .workflow-content {
    padding: var(--spacing-md);
    font-size: var(--font-size-sm);
    color: var(--text-300);
  }

  /* 未知步骤样式 */
  .unknown-step {
    border: 1px solid var(--border-300);
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
    border-bottom: 1px solid var(--border-300);
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
</style>
