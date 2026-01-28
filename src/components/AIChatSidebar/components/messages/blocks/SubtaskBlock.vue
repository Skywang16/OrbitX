<script setup lang="ts">
  import { computed, ref, watch } from 'vue'
  import { useWorkspaceStore } from '@/stores/workspace'
  import type { Block } from '@/types'
  import AIMessage from '../AIMessage.vue'
  import UserMessage from '../UserMessage.vue'

  type SubtaskBlock = Extract<Block, { type: 'subtask' }>

  interface Props {
    block: SubtaskBlock
  }

  const props = defineProps<Props>()
  const expanded = ref(props.block.status === 'running' || props.block.status === 'pending')
  const userToggled = ref(false)
  const workspaceStore = useWorkspaceStore()
  const loading = ref(false)
  const loadError = ref<string | null>(null)

  const statusLabel = computed(() => {
    switch (props.block.status) {
      case 'pending':
        return 'Pending'
      case 'running':
        return 'Running'
      case 'completed':
        return 'Completed'
      case 'cancelled':
        return 'Cancelled'
      case 'error':
        return 'Error'
      default:
        return props.block.status
    }
  })

  const childMessages = computed(() => {
    return workspaceStore.getCachedMessages(props.block.childSessionId)
  })

  const toggleDetails = async () => {
    userToggled.value = true
    expanded.value = !expanded.value
    if (!expanded.value) return
    if (childMessages.value.length > 0 || loading.value) return
    loading.value = true
    loadError.value = null
    try {
      await workspaceStore.fetchMessages(props.block.childSessionId)
    } catch (err) {
      loadError.value = err instanceof Error ? err.message : String(err)
    } finally {
      loading.value = false
    }
  }

  // When the subtask is running, default to inline streaming unless the user explicitly collapsed it.
  watch(
    () => props.block.status,
    status => {
      if (userToggled.value) return
      if (status === 'running' || status === 'pending') {
        expanded.value = true
      }
    }
  )
</script>

<template>
  <div class="subtask-inline step-block" :data-status="block.status">
    <div class="subtask-line" :class="{ expanded }" @click="toggleDetails">
      <span class="caret" aria-hidden="true">{{ expanded ? '▾' : '▸' }}</span>
      <span class="label">Subtask</span>
      <span class="agent">{{ block.agentType }}</span>
      <span class="sep">·</span>
      <span class="desc">{{ block.description }}</span>
      <span class="status">{{ statusLabel }}</span>
    </div>

    <div v-if="block.summary" class="subtask-summary">{{ block.summary }}</div>

    <transition name="expand">
      <div v-if="expanded" class="subtask-thread" @click.stop>
        <div v-if="loading" class="subtask-thread-loading">Loading…</div>
        <div v-else-if="loadError" class="subtask-thread-error">{{ loadError }}</div>
        <div v-else-if="childMessages.length === 0" class="subtask-thread-empty">No messages</div>
        <div v-else class="subtask-thread-messages">
          <template v-for="msg in childMessages" :key="msg.id">
            <UserMessage v-if="msg.role === 'user'" :message="msg" />
            <AIMessage v-else-if="msg.role === 'assistant'" :message="msg" />
          </template>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
  .subtask-inline {
    color: var(--text-200);
  }

  .subtask-line {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
    cursor: pointer;
    color: var(--text-400);
    user-select: none;
  }

  .caret {
    width: 14px;
    flex: 0 0 14px;
    color: var(--text-500);
  }

  .label {
    font-size: var(--font-size-sm);
    color: var(--text-400);
  }

  .agent {
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    color: var(--text-300);
  }

  .sep {
    color: var(--text-500);
  }

  .desc {
    font-size: var(--font-size-sm);
    color: var(--text-200);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1 1 auto;
  }

  .status {
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    color: var(--text-500);
    flex: 0 0 auto;
  }

  .subtask-summary {
    margin-top: 8px;
    font-size: var(--font-size-sm);
    color: var(--text-400);
    white-space: pre-wrap;
  }

  .subtask-inline[data-status='error'] .status {
    color: var(--color-error);
  }

  .subtask-inline[data-status='cancelled'] {
    opacity: 0.85;
  }

  .subtask-thread {
    margin-top: 10px;
    padding-left: 12px;
    margin-left: 10px;
    border-left: 2px solid var(--border-200);
  }

  .subtask-thread-loading,
  .subtask-thread-empty,
  .subtask-thread-error {
    padding: 8px 0;
    color: var(--text-500);
  }

  .subtask-thread-error {
    color: var(--color-error);
    white-space: pre-wrap;
  }

  .subtask-thread-messages {
    padding-top: 4px;
  }

  .expand-enter-active,
  .expand-leave-active {
    transition: all 0.15s ease;
  }
  .expand-enter-from,
  .expand-leave-to {
    opacity: 0;
    transform: translateY(-4px);
  }

  .subtask-thread-messages :deep(.ai-message) {
    margin-bottom: var(--spacing-sm);
  }

  .subtask-thread-messages :deep(.ai-message-footer) {
    display: none;
  }
</style>
