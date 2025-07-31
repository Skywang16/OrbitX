<script setup lang="ts">
  // Props定义
  interface Props {
    showSessionList?: boolean
    isLoading?: boolean
  }

  // Emits定义
  interface Emits {
    (e: 'toggle-session-list'): void
    (e: 'create-new-session'): void
  }

  withDefaults(defineProps<Props>(), {
    showSessionList: false,
    isLoading: false,
  })

  const emit = defineEmits<Emits>()

  // 方法
  const handleToggleSessionList = () => {
    emit('toggle-session-list')
  }

  const handleCreateNewSession = () => {
    emit('create-new-session')
  }
</script>

<template>
  <div class="chat-header">
    <div class="header-actions">
      <!-- 会话列表切换按钮 -->
      <x-button
        variant="ghost"
        size="small"
        circle
        :class="{ active: showSessionList }"
        @click="handleToggleSessionList"
      >
        <template #icon>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01" />
          </svg>
        </template>
      </x-button>

      <!-- 新建会话按钮 -->
      <x-button variant="ghost" size="small" circle :disabled="isLoading" @click="handleCreateNewSession">
        <template #icon>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="12" y1="5" x2="12" y2="19" />
            <line x1="5" y1="12" x2="19" y2="12" />
          </svg>
        </template>
      </x-button>
    </div>
  </div>
</template>

<style scoped>
  .chat-header {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding: 16px 20px;
    border-bottom: 1px solid var(--color-border);
    background-color: var(--color-ai-sidebar-background);
    min-height: 56px;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .header-actions .active {
    background-color: var(--color-primary-background);
    color: var(--color-primary);
  }
</style>
