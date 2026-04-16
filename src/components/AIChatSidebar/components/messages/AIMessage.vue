<script setup lang="ts">
  import type { Block, Message } from '@/types'
  import { renderMarkdown } from '@/utils/markdown'
  import { useWorkspaceStore } from '@/stores/workspace'
  import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useAIChatStore } from '../../store'
  import AgentSwitchBlock from './blocks/AgentSwitchBlock.vue'
  import ExploredGroup from './blocks/ExploredGroup.vue'
  import ResearchGroup from './blocks/ResearchGroup.vue'
  import SubagentGroup from './blocks/SubagentGroup.vue'
  import ThinkingBlock from './blocks/ThinkingBlock.vue'
  import ToolBlock from './blocks/ToolBlock.vue'
  const { t } = useI18n()
  const aiChatStore = useAIChatStore()
  const workspaceStore = useWorkspaceStore()

  interface Props {
    message: Message
    disableToolExpand?: boolean
  }

  const props = defineProps<Props>()

  const normalizeBlockType = (block: Block): Block => {
    const type = block.type.trim()
    if (type === block.type) return block
    return { ...block, type } as Block
  }

  const blocks = computed<Block[]>(() => props.message.blocks.map(normalizeBlockType))

  type RenderItem =
    | { kind: 'block'; block: Block; key: string }
    | { kind: 'explored-group'; blocks: Extract<Block, { type: 'tool' }>[]; key: string }
    | { kind: 'research-group'; blocks: Extract<Block, { type: 'tool' }>[]; key: string }

  const isExplorationTool = (b: Block): b is Extract<Block, { type: 'tool' }> =>
    b.type === 'tool' &&
    ['grep', 'semantic_search', 'read_file', 'list_files', 'glob', 'read_terminal'].includes(b.name)

  const isResearchTool = (b: Block): b is Extract<Block, { type: 'tool' }> =>
    b.type === 'tool' && ['web_search', 'web_fetch'].includes(b.name)

  const renderItems = computed<RenderItem[]>(() => {
    const items: RenderItem[] = []
    let explore: Extract<Block, { type: 'tool' }>[] = []
    let research: Extract<Block, { type: 'tool' }>[] = []

    const flushExplore = () => {
      if (!explore.length) return
      items.push({ kind: 'explored-group', blocks: explore, key: explore.map(b => b.id).join('-') })
      explore = []
    }
    const flushResearch = () => {
      if (!research.length) return
      items.push({ kind: 'research-group', blocks: research, key: research.map(b => b.id).join('-') })
      research = []
    }

    blocks.value.forEach((block, index) => {
      if (isExplorationTool(block)) {
        flushResearch()
        explore.push(block)
        return
      }
      if (isResearchTool(block)) {
        flushExplore()
        research.push(block)
        return
      }
      flushExplore()
      flushResearch()
      items.push({
        kind: 'block',
        block,
        key: ('id' in block && block.id) || `${props.message.id}-${block.type}-${index}`,
      })
    })
    flushExplore()
    flushResearch()
    return items
  })
  const messageSubagents = computed(() => aiChatStore.getCurrentThreadSubagentsForMessage(props.message.id))

  const STREAMING_HINTS = [
    'Thinking...',
    'Reasoning...',
    'Working on it...',
    'Planning next step...',
    'Figuring it out...',
    'Putting it together...',
    'Drafting a response...',
    'Almost there...',
  ]
  const HINT_DELAY_MS = 3000
  const HINT_ROTATE_MIN_MS = 4000
  const HINT_ROTATE_MAX_MS = 8000

  const hintIndex = ref(0)
  const showHint = ref(false)
  let delayTimer: ReturnType<typeof setTimeout> | null = null
  let rotateTimer: ReturnType<typeof setTimeout> | null = null

  const currentHint = computed(() => STREAMING_HINTS[hintIndex.value % STREAMING_HINTS.length])
  const isStreaming = computed(() => props.message.status === 'streaming')
  const isSubagentThread = computed(() => workspaceStore.selectedThread?.parentThreadId != null)

  // Find the index of the last text block for streaming fade-in
  const lastTextBlockIndex = computed(() => {
    if (!isStreaming.value) return -1
    for (let i = renderItems.value.length - 1; i >= 0; i--) {
      const item = renderItems.value[i]
      if (item.kind === 'block' && item.block.type === 'text') return i
    }
    return -1
  })
  const hasActiveSubagents = computed(() =>
    messageSubagents.value.some(subagent => subagent.status === 'running' || subagent.status === 'pending')
  )
  const shouldShowStreamingHint = computed(() => {
    return (
      isStreaming.value &&
      aiChatStore.isCurrentThreadSending &&
      !aiChatStore.retryStatus &&
      !props.message.isSummary &&
      !isSubagentThread.value &&
      !hasActiveSubagents.value
    )
  })

  const resetHintDelay = () => {
    showHint.value = false
    hintIndex.value = Math.floor(Math.random() * STREAMING_HINTS.length)
    if (delayTimer) clearTimeout(delayTimer)
    if (shouldShowStreamingHint.value) {
      delayTimer = setTimeout(() => {
        showHint.value = true
      }, HINT_DELAY_MS)
    }
  }

  watch(() => blocks.value.length, resetHintDelay)
  watch(shouldShowStreamingHint, enabled => {
    if (enabled) {
      resetHintDelay()
    } else {
      showHint.value = false
      if (delayTimer) clearTimeout(delayTimer)
    }
  })

  const scheduleNextRotation = () => {
    const delay = HINT_ROTATE_MIN_MS + Math.random() * (HINT_ROTATE_MAX_MS - HINT_ROTATE_MIN_MS)
    rotateTimer = setTimeout(() => {
      hintIndex.value = (hintIndex.value + 1) % STREAMING_HINTS.length
      scheduleNextRotation()
    }, delay)
  }

  onMounted(() => {
    resetHintDelay()
    scheduleNextRotation()
  })
  onBeforeUnmount(() => {
    if (delayTimer) clearTimeout(delayTimer)
    if (rotateTimer) clearTimeout(rotateTimer)
  })

  const handleMessageClick = async (event: MouseEvent) => {
    const target = event.target as HTMLElement
    const copyBtn = target.closest('.code-copy-btn')

    if (copyBtn) {
      const wrapper = copyBtn.closest('.code-block-wrapper')
      const codeElement = wrapper?.querySelector('code')

      if (codeElement && codeElement.textContent) {
        try {
          await navigator.clipboard.writeText(codeElement.textContent)

          // Temporarily switch icon to success state
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
</script>

<template>
  <div class="ai-message">
    <div v-if="message.isSummary" class="summary-message">
      <div v-if="message.status === 'streaming'" class="summary-loading">Compressing…</div>
      <div v-else class="summary-divider" aria-hidden="true"></div>
    </div>

    <template v-else-if="blocks.length > 0 || messageSubagents.length > 0">
      <div
        v-for="(item, idx) in renderItems"
        :key="item.key"
        class="block-item"
        :class="{ 'block-fade-in': isStreaming }"
        :style="{ '--i': idx }"
      >
        <ExploredGroup v-if="item.kind === 'explored-group'" :blocks="item.blocks" />
        <ResearchGroup v-else-if="item.kind === 'research-group'" :blocks="item.blocks" />
        <ThinkingBlock
          v-else-if="item.block.type === 'thinking'"
          :block="item.block"
          :disable-expand="disableToolExpand"
        />

        <ToolBlock v-else-if="item.block.type === 'tool'" :block="item.block" :disable-expand="disableToolExpand" />

        <AgentSwitchBlock v-else-if="item.block.type === 'agent_switch'" :block="item.block" />

        <div v-else-if="item.block.type === 'user_text'" class="ai-message-text step-block" @click="handleMessageClick">
          <div v-html="renderMarkdown(item.block.content)"></div>
        </div>

        <div v-else-if="item.block.type === 'user_image'" class="ai-message-text step-block">
          <img
            :src="item.block.dataUrl"
            :alt="item.block.fileName || 'image'"
            style="max-width: 100%; border-radius: 8px"
          />
        </div>

        <div
          v-else-if="item.block.type === 'text'"
          class="ai-message-text step-block"
          :class="{ 'text-streaming': idx === lastTextBlockIndex }"
          @click="handleMessageClick"
        >
          <div v-html="renderMarkdown(item.block.content)"></div>
        </div>

        <div v-else-if="item.block.type === 'error'" class="error-inline step-block">
          <span class="error-message">{{ item.block.message }}</span>
        </div>

        <div v-else class="unknown-step step-block">
          <div class="unknown-header">
            <span class="unknown-icon">❓</span>
            <span class="unknown-label">Unknown block</span>
          </div>
        </div>
      </div>

      <SubagentGroup v-if="messageSubagents.length > 0" :subagents="messageSubagents" />
    </template>

    <div v-if="shouldShowStreamingHint && showHint" class="streaming-hint">
      <span class="streaming-hint-text">{{ currentHint }}</span>
    </div>

    <div v-if="isStreaming && aiChatStore.retryStatus" class="empty-message-status">
      <div class="retry-status">
        <span class="retry-label">
          {{ t('message.reconnecting', 'Reconnecting...') }} {{ aiChatStore.retryStatus.attempt }}/{{
            aiChatStore.retryStatus.maxAttempts
          }}
        </span>
        <span class="retry-error">{{ aiChatStore.retryStatus.errorMessage }}</span>
      </div>
    </div>

    <div v-if="blocks.length === 0 && !message.isSummary && !isStreaming" class="empty-message-status">
      {{ t('message.empty_response', 'No response from model') }}
    </div>
  </div>
</template>

<style scoped>
  .ai-message {
    padding: var(--spacing-md) 0;
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
    text-align: center;
    font-size: var(--font-size-xs);
    color: var(--text-500);
  }

  .block-item + .block-item {
    margin-top: 6px;
  }

  .block-fade-in {
    animation: block-enter 0.35s cubic-bezier(0.4, 0, 0.2, 1) both;
    animation-delay: calc(var(--i, 0) * 40ms);
  }

  @keyframes block-enter {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .step-block {
    margin-bottom: var(--spacing-sm);
  }

  .step-block:last-of-type {
    margin-bottom: 0;
  }

  .error-inline {
    font-size: var(--font-size-sm);
    line-height: 1.6;
  }

  .error-message {
    color: color-mix(in srgb, var(--color-error) 60%, var(--text-400));
  }

  .streaming-hint {
    padding: 4px 0;
    font-size: 13px;
    line-height: 1.6;
  }

  .streaming-hint-text {
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
    animation: hint-scan 2s linear infinite;
  }

  @keyframes hint-scan {
    0% {
      background-position: 100% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }

  .empty-message-status {
    padding: 6px 0;
    font-size: var(--font-size-sm);
    color: var(--text-400);
    line-height: 1.4;
  }

  .retry-status {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .retry-label {
    color: var(--text-300);
    font-weight: 500;
  }

  .retry-error {
    color: color-mix(in srgb, var(--color-error) 60%, var(--text-400));
    font-size: var(--font-size-xs);
    word-break: break-word;
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

  /* Streaming fade-in: each new paragraph/block animates in as it appears */
  .text-streaming :deep(p:last-child),
  .text-streaming :deep(li:last-child),
  .text-streaming :deep(h1:last-child),
  .text-streaming :deep(h2:last-child),
  .text-streaming :deep(h3:last-child),
  .text-streaming :deep(pre:last-child),
  .text-streaming :deep(blockquote:last-child) {
    animation: stream-in 0.35s ease both;
  }

  @keyframes stream-in {
    from {
      opacity: 0.2;
    }
    to {
      opacity: 1;
    }
  }

  /* Blinking cursor at the end of streaming text */
  .text-streaming > div::after {
    content: '▋';
    display: inline;
    font-size: 0.85em;
    color: var(--color-primary);
    opacity: 0.8;
    animation: blink-cursor 0.9s step-end infinite;
    margin-left: 1px;
    vertical-align: baseline;
  }

  @keyframes blink-cursor {
    0%,
    100% {
      opacity: 0.8;
    }
    50% {
      opacity: 0;
    }
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

  /* Headings */
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

  /* Code */
  .ai-message-text :deep(code) {
    font-family: var(--font-family-mono);
    font-size: 0.85em;
    padding: 0.2em 0.4em;
    background: var(--bg-300);
    border-radius: var(--border-radius-sm);
    color: var(--text-100);
    -webkit-font-smoothing: auto;
  }

  /* --- New code block style (Card Style) --- */
  .ai-message-text :deep(.code-block-wrapper) {
    margin: var(--spacing-md) 0;
    border-radius: var(--border-radius-lg);
    overflow: hidden;
    background: var(--bg-100); /* Use theme background color */
    border: 1px solid var(--border-300);
  }

  /* Code block header */
  .ai-message-text :deep(.code-block-header) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: var(--bg-300); /* Use theme variables */
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
    border-radius: var(--border-radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s ease;
  }

  .ai-message-text :deep(.code-copy-btn:hover) {
    color: var(--text-200);
    background: var(--color-hover); /* Use theme variables */
  }

  /* Override pre styles */
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
    color: var(--text-200); /* Use theme variables */
  }

  /* --- New table style --- */
  .ai-message-text :deep(.table-wrapper) {
    margin: var(--spacing-md) 0;
    overflow-x: auto;
    border-radius: var(--border-radius-md);
    border: 1px solid var(--border-300);
  }

  .ai-message-text :deep(table) {
    margin: 0;
    border-collapse: collapse;
    width: 100%;
    border: none;
  }

  .ai-message-text :deep(th) {
    background: var(--bg-400); /* Use theme variables */
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

  /* Heading enhancement */
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

  /* Lists */
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

  /* Blockquote */
  .ai-message-text :deep(blockquote) {
    margin: var(--spacing-sm) 0;
    padding: var(--spacing-xs) var(--spacing-md);
    border-left: 3px solid var(--border-300);
    background: var(--bg-200);
    color: var(--text-300);
    border-radius: var(--border-radius-sm);
  }

  .ai-message-text :deep(blockquote p) {
    margin: var(--spacing-xs) 0;
  }

  /* Links */
  .ai-message-text :deep(a) {
    color: var(--color-primary);
    text-decoration: none;
  }

  .ai-message-text :deep(a:hover) {
    text-decoration: underline;
  }

  /* Horizontal rule */
  .ai-message-text :deep(hr) {
    margin: var(--spacing-md) 0;
    border: none;
    border-top: 1px solid var(--border-200);
  }

  /* Images */
  .ai-message-text :deep(img) {
    max-width: 100%;
    height: auto;
    border-radius: var(--border-radius-md);
    margin: var(--spacing-sm) 0;
  }

  /* Emphasis */
  .ai-message-text :deep(strong) {
    font-weight: 600;
    color: var(--text-100);
  }

  .ai-message-text :deep(em) {
    font-style: italic;
  }

  /* Strikethrough and underline */
  .ai-message-text :deep(del) {
    text-decoration: line-through;
    opacity: 0.7;
  }

  .ai-message-text :deep(u) {
    text-decoration: underline;
  }
</style>
