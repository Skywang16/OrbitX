<script setup lang="ts">
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import type { Block, Message } from '@/types'
  import { formatTime } from '@/utils/dateFormatter'
  import { renderMarkdown } from '@/utils/markdown'
  import ThinkingBlock from './blocks/ThinkingBlock.vue'
  import ToolBlock from './blocks/ToolBlock.vue'
  import AgentSwitchBlock from './blocks/AgentSwitchBlock.vue'
  import SubtaskBlock from './blocks/SubtaskBlock.vue'
  const { t } = useI18n()

  interface Props {
    message: Message
  }

  const props = defineProps<Props>()

  const normalizeBlockType = (block: Block): Block => {
    const type = block.type.trim()
    if (type === block.type) return block
    return { ...block, type } as Block
  }

  const blocks = computed<Block[]>(() => props.message.blocks.map(normalizeBlockType))

  const handleMessageClick = async (event: MouseEvent) => {
    const target = event.target as HTMLElement
    const copyBtn = target.closest('.code-copy-btn')

    if (copyBtn) {
      const wrapper = copyBtn.closest('.code-block-wrapper')
      const codeElement = wrapper?.querySelector('code')

      if (codeElement && codeElement.textContent) {
        try {
          await navigator.clipboard.writeText(codeElement.textContent)

          // 临时切换图标为成功状态
          const originalHTML = copyBtn.innerHTML
          copyBtn.innerHTML = `
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="color: var(--color-success)">
              <polyline points="20 6 9 17 4 12"></polyline>
            </svg>
          `
          setTimeout(() => {
            copyBtn.innerHTML = originalHTML
          }, 2000)
        } catch (err) {
          console.error('Failed to copy code:', err)
        }
      }
    }
  }

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`
    const seconds = (ms / 1000).toFixed(1)
    return `${seconds}s`
  }
</script>

<template>
  <div class="ai-message">
    <div v-if="message.isSummary" class="summary-message">
      <div v-if="message.status === 'streaming'" class="summary-loading">正在压缩…</div>
      <div v-else class="summary-divider" aria-hidden="true"></div>
    </div>

    <template v-else-if="blocks.length > 0">
      <template
        v-for="(block, index) in blocks"
        :key="('id' in block && block.id) || `${message.id}-${block.type}-${index}`"
      >
        <ThinkingBlock v-if="block.type === 'thinking'" :block="block" />

        <ToolBlock v-else-if="block.type === 'tool' && block.name !== 'task'" :block="block" />

        <!-- `task` is orchestration-only; don't render it as a normal tool block -->
        <template v-else-if="block.type === 'tool' && block.name === 'task'"></template>

        <AgentSwitchBlock v-else-if="block.type === 'agent_switch'" :block="block" />

        <SubtaskBlock v-else-if="block.type === 'subtask'" :block="block" />

        <div v-else-if="block.type === 'user_text'" class="ai-message-text step-block" @click="handleMessageClick">
          <div v-html="renderMarkdown(block.content)"></div>
        </div>

        <div v-else-if="block.type === 'user_image'" class="ai-message-text step-block">
          <img :src="block.dataUrl" :alt="block.fileName || 'image'" style="max-width: 100%; border-radius: 8px" />
        </div>

        <div v-else-if="block.type === 'text'" class="ai-message-text step-block" @click="handleMessageClick">
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

  .summary-message {
    margin: 10px 0;
    width: 100%;
  }

  .summary-loading {
    width: 100%;
    padding: 10px 0;
    text-align: center;
    font-size: var(--font-size-sm);
    color: var(--text-400);
  }

  .summary-divider {
    padding: 8px 0;
    border-top: 1px dashed var(--border-200);
    border-bottom: 1px dashed var(--border-200);
    text-align: center;
    font-size: var(--font-size-xs);
    color: var(--text-500);
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
    font-size: var(--font-size-md);
    line-height: 1.6;
    color: var(--text-200);
    overflow: hidden;
    min-width: 0;
    word-wrap: break-word;
    word-break: break-word;
    overflow-wrap: break-word;
  }

  .ai-message-text :deep(p) {
    margin: var(--spacing-sm) 0;
    font-size: 1em;
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
    line-height: 1.5;
    color: var(--text-100);
    letter-spacing: 0.02em;
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
    font-family: var(--font-family-mono);
    font-size: 0.85em;
    padding: 0.2em 0.4em;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 4px;
    color: var(--text-100);
    -webkit-font-smoothing: auto;
  }

  /* --- 代码块新样式 (Card Style) --- */
  .ai-message-text :deep(.code-block-wrapper) {
    margin: var(--spacing-md) 0;
    border-radius: var(--border-radius-lg);
    overflow: hidden;
    background: var(--bg-100); /* 使用主题背景色 */
    border: 1px solid var(--border-300);
  }

  /* 代码块头部 */
  .ai-message-text :deep(.code-block-header) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: var(--bg-300); /* 使用主题变量 */
    border-bottom: 1px solid var(--border-200);
  }

  .ai-message-text :deep(.code-lang) {
    font-size: 12px;
    color: var(--text-400);
    text-transform: lowercase;
    font-family: var(--font-family-mono);
  }

  .ai-message-text :deep(.code-copy-btn) {
    background: transparent;
    border: none;
    color: var(--text-400);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s ease;
  }

  .ai-message-text :deep(.code-copy-btn:hover) {
    color: var(--text-200);
    background: var(--color-hover); /* 使用主题变量 */
  }

  /* 覆盖 pre 样式 */
  .ai-message-text :deep(pre) {
    margin: 0 !important;
    padding: 12px !important;
    background: transparent !important;
    border: none !important;
    border-radius: 0 !important;
    overflow-x: auto;
  }

  .ai-message-text :deep(pre code) {
    background: transparent !important;
    padding: 0 !important;
    border: none !important;
    font-family: var(--font-family-mono);
    font-size: 13px !important;
    line-height: 1.6 !important;
    color: var(--text-200); /* 使用主题变量 */
  }

  /* --- 表格新样式 --- */
  .ai-message-text :deep(.table-wrapper) {
    margin: var(--spacing-md) 0;
    overflow-x: auto;
    border-radius: var(--border-radius);
    border: 1px solid var(--border-300);
  }

  .ai-message-text :deep(table) {
    margin: 0;
    border-collapse: collapse;
    width: 100%;
    border: none;
  }

  .ai-message-text :deep(th) {
    background: var(--bg-400); /* 使用主题变量 */
    color: var(--text-200);
    font-weight: 600;
    padding: 10px 16px;
    border: none;
    border-bottom: 1px solid var(--border-300);
    text-align: left;
    font-size: 0.9em;
  }

  .ai-message-text :deep(td) {
    padding: 10px 16px;
    border: none;
    border-bottom: 1px solid var(--border-200);
    color: var(--text-300);
    font-size: 0.9em;
  }

  .ai-message-text :deep(tr:last-child td) {
    border-bottom: none;
  }

  /* 标题增强 */
  .ai-message-text :deep(h1),
  .ai-message-text :deep(h2) {
    margin-top: 24px;
    margin-bottom: 16px;
    color: var(--text-100);
    letter-spacing: -0.01em;
  }

  .ai-message-text :deep(h3) {
    margin-top: 20px;
    margin-bottom: 12px;
  }

  /* 列表 */
  .ai-message-text :deep(ul),
  .ai-message-text :deep(ol) {
    margin: var(--spacing-sm) 0;
    padding-left: 1.5em;
    line-height: 1.6;
  }

  .ai-message-text :deep(li) {
    margin: var(--spacing-sm) 0;
  }

  .ai-message-text :deep(li > p) {
    margin: var(--spacing-xs) 0;
  }

  .ai-message-text :deep(li::marker) {
    color: var(--text-400);
  }

  /* 引用 */
  .ai-message-text :deep(blockquote) {
    margin: var(--spacing-sm) 0;
    padding: var(--spacing-xs) var(--spacing-md);
    border-left: 3px solid var(--border-300);
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-300);
    border-radius: var(--border-radius-sm);
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

  /* 图片 */
  .ai-message-text :deep(img) {
    max-width: 100%;
    height: auto;
    border-radius: var(--border-radius);
    margin: var(--spacing-sm) 0;
  }

  /* 强调 */
  .ai-message-text :deep(strong) {
    font-weight: 600;
    color: var(--text-100);
  }

  .ai-message-text :deep(em) {
    font-style: italic;
  }

  /* 删除线和下划线 */
  .ai-message-text :deep(del) {
    text-decoration: line-through;
    opacity: 0.7;
  }

  .ai-message-text :deep(u) {
    text-decoration: underline;
  }
</style>
