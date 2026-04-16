<script setup lang="ts">
  import { useAIChatStore } from '@/components/AIChatSidebar/store'
  import { useWorkspaceStore } from '@/stores/workspace'
  import type { Block, SubagentRecord } from '@/types'
  import { renderMarkdown } from '@/utils/markdown'
  import { computed } from 'vue'

  interface Props {
    subagent: SubagentRecord
  }

  const props = defineProps<Props>()
  const aiChatStore = useAIChatStore()
  const workspaceStore = useWorkspaceStore()

  const stateTone = computed(() => props.subagent.status)
  type PreviewItem = {
    label: string
    detail?: string
  }

  const shorten = (value: string, max = 72) => {
    const text = value.replace(/\s+/g, ' ').trim()
    if (text.length <= max) return text
    return `${text.slice(0, max - 1)}…`
  }

  const summarizeTool = (block: Extract<Block, { type: 'tool' }>): PreviewItem => {
    const params = typeof block.input === 'object' && block.input ? (block.input as Record<string, unknown>) : {}
    const rawDetail = [
      params.path,
      params.file_path,
      params.url,
      params.pattern,
      params.command,
      params.query,
      params.description,
    ].find(value => typeof value === 'string' && value.trim())

    return {
      label: block.name.replace(/[_-]+/g, ' '),
      detail: typeof rawDetail === 'string' ? shorten(rawDetail) : undefined,
    }
  }

  const latestPreview = computed<PreviewItem | null>(() => {
    const messages = workspaceStore.getCachedMessages(props.subagent.childThreadId)

    for (const message of [...messages].reverse()) {
      if (message.role !== 'assistant') continue

      for (const block of [...message.blocks].reverse()) {
        if (block.type === 'tool') return summarizeTool(block)

        if (block.type === 'thinking' || block.type === 'text') {
          const text = shorten(block.content)
          if (text) return { label: text }
        }

        if (block.type === 'error') {
          const text = shorten(block.message)
          if (text) return { label: text }
        }
      }
    }

    const activity = props.subagent.latestActivity?.trim()
    return activity ? { label: shorten(activity) } : null
  })

  const latestPreviewKey = computed(() => {
    if (!latestPreview.value) return 'empty'
    return `${latestPreview.value.label}::${latestPreview.value.detail || ''}`
  })

  const bodyHtml = computed(() => {
    if (props.subagent.status === 'completed') {
      const text = props.subagent.finalSummary?.trim()
      return text ? renderMarkdown(text) : ''
    }

    if (props.subagent.status === 'error' || props.subagent.status === 'cancelled') {
      const text = props.subagent.errorMessage?.trim()
      return text ? renderMarkdown(text) : ''
    }

    return ''
  })

  const openChild = async () => {
    await aiChatStore.switchThread(props.subagent.childThreadId)
  }
</script>

<template>
  <button type="button" class="subagent-card" :class="`tone-${stateTone}`" @click="openChild">
    <div class="card-head">
      <!-- Status icon -->
      <span class="card-status-icon">
        <!-- Running: spinning dashed circle -->
        <svg
          v-if="subagent.status === 'running' || subagent.status === 'pending'"
          class="status-spinner"
          width="14"
          height="14"
          viewBox="0 0 14 14"
          fill="none"
        >
          <circle cx="7" cy="7" r="5.5" stroke="var(--color-primary)" stroke-width="1.5" stroke-dasharray="8 6" fill="none" />
        </svg>
        <!-- Completed: checkmark circle -->
        <svg v-else-if="subagent.status === 'completed'" width="14" height="14" viewBox="0 0 14 14" fill="none">
          <circle cx="7" cy="7" r="6.5" stroke="var(--color-success)" stroke-width="1" fill="color-mix(in srgb, var(--color-success) 15%, transparent)" />
          <path d="M4.5 7L6.5 9L9.5 5" stroke="var(--color-success)" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" fill="none" />
        </svg>
        <!-- Error: X circle -->
        <svg v-else-if="subagent.status === 'error'" width="14" height="14" viewBox="0 0 14 14" fill="none">
          <circle cx="7" cy="7" r="6.5" stroke="var(--color-error)" stroke-width="1" fill="color-mix(in srgb, var(--color-error) 12%, transparent)" />
          <path d="M5 5L9 9M9 5L5 9" stroke="var(--color-error)" stroke-width="1.2" stroke-linecap="round" fill="none" />
        </svg>
        <!-- Cancelled / default: plain circle -->
        <svg v-else width="14" height="14" viewBox="0 0 14 14" fill="none">
          <circle cx="7" cy="7" r="6.5" stroke="var(--text-500)" stroke-width="1" fill="none" />
        </svg>
      </span>
      <span class="card-name">{{ subagent.name }}</span>
      <span class="card-profile">{{ subagent.profile }}</span>
    </div>
    <div class="card-title">{{ subagent.taskTitle }}</div>
    <div
      v-if="(subagent.status === 'running' || subagent.status === 'pending') && latestPreview"
      class="card-latest-shell"
    >
      <Transition name="latest-preview" mode="out-in">
        <div :key="latestPreviewKey" class="card-latest">
          <span class="latest-label">{{ latestPreview.label }}</span>
          <span v-if="latestPreview.detail" class="latest-detail">{{ latestPreview.detail }}</span>
        </div>
      </Transition>
    </div>
    <div v-else-if="bodyHtml" class="card-body" v-html="bodyHtml"></div>
  </button>
</template>

<style scoped>
  .subagent-card {
    display: flex;
    flex-direction: column;
    width: 100%;
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--bg-100);
    border: 1px solid var(--border-200);
    color: inherit;
    text-align: left;
    cursor: pointer;
    box-sizing: border-box;
    overflow: hidden;
    transition:
      border-color 0.15s ease,
      background-color 0.15s ease;
  }

  .subagent-card:hover {
    border-color: var(--border-200);
    background: color-mix(in srgb, var(--bg-200) 45%, transparent);
  }

  .subagent-card.tone-running,
  .subagent-card.tone-pending {
    border-color: var(--border-200);
  }

  .subagent-card.tone-completed {
    border-color: var(--border-200);
  }

  .subagent-card.tone-error {
    border-color: color-mix(in srgb, var(--color-error) 35%, transparent);
  }

  .subagent-card.tone-cancelled {
    border-color: var(--border-200);
    opacity: 0.5;
  }

  .card-head {
    display: flex;
    align-items: center;
    flex-wrap: nowrap;
    gap: 6px;
    margin-bottom: 5px;
  }

  .card-status-icon {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .status-spinner {
    animation: spin 1.5s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .card-name {
    font-size: 12px;
    font-weight: 700;
    color: var(--text-200);
  }

  .card-profile {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-500);
  }

  .card-title {
    font-size: 13px;
    font-weight: 600;
    line-height: 1.45;
    color: var(--text-200);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    min-height: calc(1.45em * 2);
    margin-bottom: 6px;
  }

  .card-body {
    font-size: 11px;
    color: var(--text-400);
    line-height: 1.45;
    min-height: calc(1.45em * 3);
    max-height: calc(1.45em * 3);
    overflow: hidden;
    -webkit-mask-image: linear-gradient(to bottom, black 40%, transparent 100%);
    mask-image: linear-gradient(to bottom, black 40%, transparent 100%);
  }

  .card-body :deep(*) {
    color: inherit;
  }

  .card-body :deep(p) {
    margin: 0;
  }

  .card-body :deep(p + p) {
    margin-top: 2px;
  }

  .card-body :deep(strong) {
    color: var(--text-300);
  }

  .card-latest-shell {
    overflow: hidden;
    min-height: calc(1.4em + 1.35em + 2px);
  }

  .card-latest {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-top: 2px;
  }

  .latest-label {
    font-size: 11px;
    color: var(--text-400);
    line-height: 1.4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .latest-detail {
    font-size: 10px;
    color: var(--text-500);
    line-height: 1.35;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .latest-preview-enter-active,
  .latest-preview-leave-active {
    transition:
      transform 0.22s ease,
      opacity 0.22s ease;
  }

  .latest-preview-enter-from {
    opacity: 0;
    transform: translateY(10px);
  }

  .latest-preview-leave-to {
    opacity: 0;
    transform: translateY(-10px);
  }
</style>
