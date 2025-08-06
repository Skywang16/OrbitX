<script setup lang="ts">
  import SessionSelect from './SessionSelect.vue'
  import type { ChatSession } from '@/types'
  import type { ChatMode } from '../types'

  // Props定义
  interface Props {
    sessions: ChatSession[]
    currentSessionId: string | null
    isLoading?: boolean
  }

  // Emits定义
  interface Emits {
    (e: 'select-session', sessionId: string): void
    (e: 'create-new-session'): void
    (e: 'delete-session', sessionId: string): void
    (e: 'refresh-sessions'): void
  }

  const props = withDefaults(defineProps<Props>(), {
    isLoading: false,
  })

  const emit = defineEmits<Emits>()

  // 方法
  const handleSelectSession = (sessionId: string) => {
    emit('select-session', sessionId)
  }

  const handleCreateNewSession = () => {
    emit('create-new-session')
  }

  const handleDeleteSession = (sessionId: string) => {
    emit('delete-session', sessionId)
  }

  const handleRefreshSessions = () => {
    emit('refresh-sessions')
  }
</script>

<template>
  <div class="chat-header">
    <div class="header-content">
      <!-- 会话选择下拉菜单 -->
      <SessionSelect
        :sessions="sessions"
        :current-session-id="currentSessionId"
        :loading="isLoading"
        @select-session="handleSelectSession"
        @create-new-session="handleCreateNewSession"
        @delete-session="handleDeleteSession"
        @refresh-sessions="handleRefreshSessions"
      />
    </div>

    <div class="header-actions">
      <!-- 新建会话按钮 -->
      <button class="new-session-btn" :disabled="isLoading" @click="handleCreateNewSession" title="新建会话">
        <svg width="1em" height="1em" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
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
    flex-direction: column;
    border-bottom: 1px solid var(--color-border);
    background-color: var(--color-ai-sidebar-background);
    gap: 0.5em;
  }

  .header-content {
    flex: 1;
    display: flex;
    align-items: center;
    min-width: 0;
    overflow: hidden;
    padding: 0 0.8em;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
    padding-right: 0.8em;
  }

  .new-session-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.4em;
    height: 1.4em;
    border: none;
    background: none;
    color: var(--color-text-secondary);
    cursor: pointer;
    transition: color 0.2s ease;
    padding: 0;
    flex-shrink: 0;
  }

  .new-session-btn:hover {
    color: var(--color-text);
  }

  .new-session-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
