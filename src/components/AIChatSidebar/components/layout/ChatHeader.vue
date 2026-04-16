<script setup lang="ts">
  /**
   * ChatHeader - Header of the chat sidebar
   * Note: This component is only used in AIChatSidebar, the main interface uses MainChatArea
   */
  import type { ThreadRecord } from '@/api/workspace'
  import { useI18n } from 'vue-i18n'
  import SessionSelect from './SessionSelect.vue'

  interface Props {
    threads: ThreadRecord[]
    currentThreadId?: number | null
    parentThreadId?: number | null
    selectedLabel?: string | null
    isLoading?: boolean
  }

  interface Emits {
    (e: 'select-thread', threadId: number): void
    (e: 'create-new-thread'): void
    (e: 'refresh-threads'): void
    (e: 'go-back'): void
  }

  withDefaults(defineProps<Props>(), {
    isLoading: false,
  })

  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const handleSelectThread = (threadId: number) => {
    emit('select-thread', threadId)
  }

  const handleCreateNewThread = () => {
    emit('create-new-thread')
  }

  const handleRefreshThreads = () => {
    emit('refresh-threads')
  }
</script>

<template>
  <div class="chat-header">
    <button v-if="parentThreadId" class="back-btn" @click="emit('go-back')">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
        <path d="M19 12H5M12 19l-7-7 7-7" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </button>
    <div class="header-content">
      <SessionSelect
        :threads="threads"
        :current-thread-id="currentThreadId || null"
        :selected-label="selectedLabel || null"
        :loading="isLoading"
        @select-thread="handleSelectThread"
        @create-new-thread="handleCreateNewThread"
        @refresh-threads="handleRefreshThreads"
      />
    </div>

    <div class="header-actions">
      <button class="icon-btn" :title="t('chat.new_session')" @click="handleCreateNewThread">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="12" y1="5" x2="12" y2="19" />
          <line x1="5" y1="12" x2="19" y2="12" />
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
  .chat-header {
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--border-200);
    background-color: var(--bg-200);
    padding: 6px 12px 0 12px;
    gap: 8px;
    height: 40px;
    position: relative;
  }

  .back-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: none;
    color: var(--text-300);
    cursor: pointer;
    padding: 2px;
    border-radius: 6px;
    flex-shrink: 0;
  }

  .back-btn:hover {
    color: var(--text-100);
    background: var(--bg-300);
  }

  .chat-header::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    bottom: -24px;
    height: 24px;
    pointer-events: none;
    z-index: -1;
    background: linear-gradient(to bottom, var(--bg-200) 0%, transparent 100%);
  }

  .header-content {
    flex: 1;
    display: flex;
    align-items: center;
    min-width: 0;
    overflow: hidden;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-sm);
    color: var(--text-300);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .icon-btn:hover {
    background: var(--color-hover);
    color: var(--color-primary);
  }

  .icon-btn:active {
    transform: scale(0.95);
  }

  .icon-btn svg {
    width: 16px;
    height: 16px;
  }
</style>
