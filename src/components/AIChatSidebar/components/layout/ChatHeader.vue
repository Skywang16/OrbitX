<script setup lang="ts">
  import { computed, ref } from 'vue'
  import { onClickOutside } from '@vueuse/core'
  import { useI18n } from 'vue-i18n'
  import SessionSelect from './SessionSelect.vue'
  import type { Conversation, AgentTerminal, AgentTerminalStatus } from '@/types'
  import { useAgentTerminalStore } from '@/stores/agentTerminal'
  import { useEditorStore } from '@/stores/Editor'
  import { showContextMenu } from '@/ui/composables/popover-api'

  // Props定义
  interface Props {
    sessions: Conversation[]
    currentSessionId?: number | null
    chatMode?: 'chat' | 'agent'
    isLoading?: boolean
  }

  // Emits定义
  interface Emits {
    (e: 'select-session', sessionId: number): void
    (e: 'create-new-session'): void
    (e: 'refresh-sessions'): void
  }

  const props = withDefaults(defineProps<Props>(), {
    isLoading: false,
  })

  const emit = defineEmits<Emits>()
  const { t } = useI18n()
  const agentTerminalStore = useAgentTerminalStore()
  const editorStore = useEditorStore()

  const isMenuOpen = ref(false)
  const agentTerminalRef = ref<HTMLElement | null>(null)

  const terminals = computed(() =>
    agentTerminalStore
      .listForSession(props.currentSessionId ?? undefined)
      .filter(terminal => terminal.mode === 'background')
  )
  const runningCount = computed(() => terminals.value.filter(t => t.status.type === 'running').length)
  const latestRunning = computed(() => terminals.value.find(t => t.status.type === 'running') ?? null)

  const toggleMenu = async () => {
    const nextOpen = !isMenuOpen.value
    isMenuOpen.value = nextOpen
    if (nextOpen && runningCount.value > 0 && latestRunning.value) {
      await openTerminal(latestRunning.value, false)
    }
  }

  const openTerminal = async (terminal: AgentTerminal, closeMenu = true) => {
    await editorStore.openAgentTerminalTab({
      terminalId: terminal.id,
      paneId: terminal.paneId,
      command: terminal.command,
      label: terminal.label ?? undefined,
      activate: true,
    })
    if (closeMenu) {
      isMenuOpen.value = false
    }
  }

  const formatStatus = (status: AgentTerminalStatus): string => {
    switch (status.type) {
      case 'running':
        return 'running'
      case 'completed':
        return status.exitCode == null ? 'completed' : `exit ${status.exitCode}`
      case 'failed':
        return 'failed'
      case 'aborted':
        return 'aborted'
      default:
        return 'initializing'
    }
  }

  const handleContextMenu = async (event: MouseEvent, terminal: AgentTerminal) => {
    event.preventDefault()
    await showContextMenu({
      x: event.clientX,
      y: event.clientY,
      items: [
        {
          label: 'Stop',
          disabled: terminal.status.type !== 'running',
          onClick: () => {
            agentTerminalStore.stopTerminal(terminal.id)
          },
        },
        {
          label: 'Delete',
          onClick: () => {
            agentTerminalStore.removeTerminal(terminal.id)
          },
        },
      ],
    })
  }

  onClickOutside(agentTerminalRef, () => {
    isMenuOpen.value = false
  })

  // 方法
  const handleSelectSession = (sessionId: number) => {
    emit('select-session', sessionId)
  }

  const handleCreateNewSession = () => {
    emit('create-new-session')
  }

  const handleRefreshSessions = () => {
    emit('refresh-sessions')
  }
</script>

<template>
  <div class="chat-header">
    <div class="header-content">
      <SessionSelect
        :sessions="sessions"
        :current-session-id="currentSessionId || null"
        :loading="isLoading"
        @select-session="handleSelectSession"
        @create-new-session="handleCreateNewSession"
        @refresh-sessions="handleRefreshSessions"
      />
    </div>

    <div class="header-actions">
      <div ref="agentTerminalRef" class="agent-terminal" @click.stop>
        <button
          class="agent-terminal-btn"
          :class="{ active: runningCount > 0 }"
          :title="runningCount > 0 ? 'Agent terminal running' : 'Agent terminals'"
          @click.stop="toggleMenu"
        >
          <svg
            class="terminal-icon"
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <rect x="3" y="4" width="18" height="16" rx="2" />
            <polyline points="7 10 10 13 7 16" stroke-linecap="round" stroke-linejoin="round" />
            <line x1="12" y1="16" x2="17" y2="16" stroke-linecap="round" />
          </svg>
          <span v-if="runningCount > 0" class="agent-terminal-badge" />
        </button>

        <div v-if="isMenuOpen" class="agent-terminal-menu">
          <div v-if="terminals.length === 0" class="agent-terminal-empty">No agent terminals</div>
          <button
            v-for="terminal in terminals"
            :key="terminal.id"
            class="agent-terminal-item"
            @click.stop="openTerminal(terminal)"
            @contextmenu.stop="handleContextMenu($event, terminal)"
          >
            <span class="agent-terminal-status" :class="terminal.status.type"></span>
            <span class="agent-terminal-command">{{ terminal.command }}</span>
            <span class="agent-terminal-meta">{{ formatStatus(terminal.status) }}</span>
          </button>
        </div>
      </div>

      <button class="new-session-btn" @click="handleCreateNewSession" :title="t('chat.new_session')">
        <svg
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
        >
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
    border-bottom: 1px solid var(--border-200);
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

  .agent-terminal {
    position: relative;
    display: flex;
    align-items: center;
  }

  .agent-terminal-btn {
    position: relative;
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
    border-radius: var(--border-radius-sm);
  }

  .agent-terminal-btn:hover {
    color: var(--text-200);
    background-color: var(--bg-500);
  }

  .agent-terminal-btn.active {
    color: var(--text-400);
  }

  .agent-terminal-badge {
    position: absolute;
    top: 2px;
    right: 2px;
    width: 6px;
    height: 6px;
    background: var(--color-success);
    border-radius: 999px;
    box-shadow: 0 0 0 2px var(--bg-300);
  }

  .agent-terminal-menu {
    position: absolute;
    right: 0;
    top: calc(100% + 6px);
    min-width: 260px;
    max-width: 420px;
    background: var(--bg-400);
    border-radius: var(--border-radius-sm);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.12), 0 1px 3px rgba(0, 0, 0, 0.08);
    padding: 6px;
    z-index: 1000;
  }

  .agent-terminal-empty {
    padding: 8px;
    color: var(--text-400);
    font-size: 12px;
  }

  .agent-terminal-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    border: none;
    background: none;
    color: var(--text-200);
    padding: 6px 8px;
    border-radius: var(--border-radius-sm);
    cursor: pointer;
    text-align: left;
    transition: background-color 0.15s ease;
  }

  .agent-terminal-item:hover {
    background: var(--bg-500);
  }

  .agent-terminal-status {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-500);
    flex-shrink: 0;
    transition:
      transform 0.15s ease,
      box-shadow 0.15s ease,
      background-color 0.15s ease;
  }

  .agent-terminal-item:hover .agent-terminal-status {
    transform: scale(1.1);
    box-shadow: 0 0 0 2px var(--bg-400);
  }

  .agent-terminal-status.running {
    background: var(--color-success);
  }

  .agent-terminal-status.completed {
    background: var(--color-success);
  }

  .agent-terminal-status.failed,
  .agent-terminal-status.aborted {
    background: var(--color-error);
  }

  .agent-terminal-command {
    flex: 1;
    font-size: 12px;
    color: var(--text-200);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .agent-terminal-meta {
    font-size: 11px;
    color: var(--text-400);
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
    border-radius: var(--border-radius-sm);
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
