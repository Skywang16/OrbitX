<script setup lang="ts">
  import type { ChatSession } from '@/types'

  // Props定义
  interface Props {
    sessions: ChatSession[]
    currentSessionId: string | null
    visible?: boolean
  }

  // Emits定义
  interface Emits {
    (e: 'close'): void
    (e: 'select-session', sessionId: string): void
    (e: 'delete-session', sessionId: string): void
  }

  const props = withDefaults(defineProps<Props>(), {
    visible: false,
  })

  const emit = defineEmits<Emits>()

  // 方法
  const handleClose = () => {
    emit('close')
  }

  const handleSelectSession = (sessionId: string) => {
    emit('select-session', sessionId)
  }

  const handleDeleteSession = (sessionId: string, event: Event) => {
    event.stopPropagation()
    emit('delete-session', sessionId)
  }

  // 格式化时间显示
  const formatSessionTime = (timestamp: Date) => {
    const now = new Date()
    const diff = now.getTime() - new Date(timestamp).getTime()
    const minutes = Math.floor(diff / (1000 * 60))
    const hours = Math.floor(diff / (1000 * 60 * 60))
    const days = Math.floor(diff / (1000 * 60 * 60 * 24))

    if (minutes < 1) return '刚刚'
    if (minutes < 60) return `${minutes}分钟前`
    if (hours < 24) return `${hours}小时前`
    if (days < 7) return `${days}天前`

    return new Date(timestamp).toLocaleDateString('zh-CN', {
      month: 'short',
      day: 'numeric',
    })
  }
</script>

<template>
  <div v-if="visible" class="session-list">
    <div class="session-list-header">
      <h3>会话历史</h3>
      <x-button variant="ghost" size="small" circle @click="handleClose">
        <template #icon>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        </template>
      </x-button>
    </div>

    <div class="session-items">
      <div v-if="sessions.length === 0" class="no-sessions">
        <p>暂无会话历史</p>
      </div>

      <div
        v-for="session in sessions"
        :key="session.id"
        class="session-item"
        :class="{ active: session.id === currentSessionId }"
        @click="handleSelectSession(session.id)"
      >
        <div class="session-content">
          <div class="session-title">{{ session.title }}</div>
          <div class="session-time">{{ formatSessionTime(session.updatedAt) }}</div>
        </div>
        <x-button variant="ghost" size="small" circle @click="handleDeleteSession(session.id, $event)">
          <template #icon>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
            </svg>
          </template>
        </x-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .session-list {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .session-list-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px;
    border-bottom: 1px solid var(--color-border);
    background-color: var(--color-ai-sidebar-background);
  }

  .session-list-header h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 500;
    color: var(--color-text);
  }

  .session-items {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
  }

  .no-sessions {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 200px;
    color: var(--color-text-secondary);
    font-size: 14px;
  }

  .session-item {
    display: flex;
    align-items: center;
    padding: 12px;
    margin-bottom: 4px;
    border-radius: 6px;
    cursor: pointer;
    transition: background-color 0.2s ease;
  }

  .session-item:hover {
    background-color: var(--color-background-hover);
  }

  .session-item.active {
    background-color: var(--color-primary-background);
  }

  .session-content {
    flex: 1;
    min-width: 0;
  }

  .session-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--color-text);
    margin-bottom: 4px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .session-time {
    font-size: 12px;
    color: var(--color-text-secondary);
  }
</style>
