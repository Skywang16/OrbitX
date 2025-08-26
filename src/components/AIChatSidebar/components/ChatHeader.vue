<script setup lang="ts">
  import { useI18n } from 'vue-i18n'
  import SessionSelect from './SessionSelect.vue'
  import type { Conversation } from '@/types'

  // Props定义
  interface Props {
    sessions: Conversation[]
    currentSessionId: number | null
    isLoading?: boolean
  }

  // Emits定义
  interface Emits {
    (e: 'select-session', sessionId: number): void
    (e: 'create-new-session'): void
    (e: 'delete-session', sessionId: number): void
    (e: 'refresh-sessions'): void
  }

  withDefaults(defineProps<Props>(), {
    isLoading: false,
  })

  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  // 方法
  const handleSelectSession = (sessionId: number) => {
    emit('select-session', sessionId)
  }

  const handleCreateNewSession = () => {
    emit('create-new-session')
  }

  const handleDeleteSession = (sessionId: number) => {
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
      <button class="new-session-btn" @click="handleCreateNewSession" :title="t('chat.new_session')">
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
    flex-direction: row;
    align-items: center;
    border-bottom: 1px solid var(--border-300);
    background-color: var(--bg-300);
    padding: 0.5em 0.8em;
    gap: 0.5em;
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

  .new-session-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2em;
    height: 2em;
    border: none;
    background: none;
    color: var(--text-400);
    cursor: pointer;
    transition: all 0.2s ease;
    padding: 0;
    flex-shrink: 0;
    border-radius: 4px;
  }

  .new-session-btn:hover {
    color: var(--text-200);
    background-color: var(--bg-500);
  }

  .new-session-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
