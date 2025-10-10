<template>
  <div class="empty-state">
    <h1 class="title">OrbitX</h1>

    <!-- Recent Workspaces Section -->
    <div v-if="recentWorkspaces.length > 0" class="recent-section">
      <h2 class="section-title">{{ t('recent_workspaces.title') }}</h2>
      <div class="workspace-list">
        <div
          v-for="workspace in recentWorkspaces"
          :key="workspace.id"
          class="workspace-item"
          @click="handleOpenWorkspace(workspace.path)"
        >
          <div class="workspace-icon">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
              <path
                d="M10 4H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z"
              />
            </svg>
          </div>
          <div class="workspace-info">
            <div class="workspace-name">{{ getWorkspaceName(workspace.path) }}</div>
            <div class="workspace-path">{{ workspace.path }}</div>
          </div>
        </div>
      </div>
    </div>

    <!-- Shortcuts Section -->
    <div class="shortcuts">
      <div class="shortcut" @click="handleNewTabClick">
        <span class="shortcut-desc">{{ t('shortcuts.actions.new_tab') }}</span>
        <div class="keys">
          <kbd>⌘</kbd>
          <kbd>T</kbd>
        </div>
      </div>
      <div class="shortcut" @click="handleToggleAISidebarClick">
        <span class="shortcut-desc">{{ t('shortcuts.actions.toggle_ai_sidebar') }}</span>
        <div class="keys">
          <kbd>⌘</kbd>
          <kbd>I</kbd>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { shortcutActionsService } from '@/shortcuts/actions'
  import { workspaceApi, type RecentWorkspace } from '@/api/workspace'
  import { useTerminalStore } from '@/stores/Terminal'

  const { t } = useI18n()
  const terminalStore = useTerminalStore()
  const recentWorkspaces = ref<RecentWorkspace[]>([])

  onMounted(async () => {
    try {
      recentWorkspaces.value = await workspaceApi.getRecentWorkspaces(5)
    } catch (error) {
      console.error('Failed to load recent workspaces:', error)
    }
  })

  const handleNewTabClick = async () => {
    await shortcutActionsService.newTab()
  }

  const handleToggleAISidebarClick = () => {
    shortcutActionsService.toggleAISidebar()
  }

  const handleOpenWorkspace = async (path: string) => {
    try {
      await shortcutActionsService.newTab()

      setTimeout(() => {
        const activeTerminal = terminalStore.activeTerminal
        if (activeTerminal) {
          terminalStore.writeToTerminal(activeTerminal.id, `cd "${path}"\n`)
        }
      }, 100)
    } catch (error) {
      console.error('Failed to open workspace:', error)
    }
  }

  const getWorkspaceName = (path: string): string => {
    const parts = path.split('/').filter(Boolean)
    return parts[parts.length - 1] || path
  }
</script>

<style scoped>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    background: var(--bg-200);
    padding: 48px 24px;
  }

  .title {
    font-size: 48px;
    font-weight: 300;
    color: var(--text-200);
    margin: 0 0 48px 0;
    letter-spacing: -0.02em;
  }

  /* Recent Workspaces Section */
  .recent-section {
    width: 100%;
    max-width: 600px;
    margin-bottom: 32px;
  }

  .section-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-400);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0 0 16px 4px;
  }

  .workspace-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .workspace-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    background: var(--bg-300);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-lg);
    cursor: pointer;
    transition: all 0.15s ease;
    user-select: none;
  }

  .workspace-item:hover {
    background: var(--bg-400);
    border-color: var(--border-300);
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  }

  .workspace-item:active {
    transform: translateY(0);
  }

  .workspace-icon {
    flex-shrink: 0;
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-300);
    background: var(--bg-500);
    border-radius: var(--border-radius);
  }

  .workspace-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .workspace-name {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-100);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .workspace-path {
    font-size: 12px;
    color: var(--text-400);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: 'SF Mono', 'Menlo', monospace;
  }

  /* Shortcuts Section */
  .shortcuts {
    display: flex;
    flex-direction: column;
    gap: 16px;
    align-items: center;
  }

  .shortcut {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 12px 20px;
    background: var(--bg-300);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-lg);
    transition: all 0.2s ease;
    min-width: 200px;
    cursor: pointer;
    user-select: none;
  }

  .shortcut:hover {
    background: var(--bg-400);
    border-color: var(--border-300);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }

  .shortcut:active {
    transform: translateY(0);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  }

  .keys {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    height: 24px;
    padding: 0 8px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-200);
    background: var(--bg-500);
    border: 1px solid var(--border-400);
    border-radius: var(--border-radius);
    box-shadow:
      0 2px 4px rgba(0, 0, 0, 0.1),
      0 0 0 1px rgba(255, 255, 255, 0.05) inset;
  }

  .shortcut-desc {
    font-size: 13px;
    color: var(--text-300);
    font-weight: 500;
    text-align: center;
  }
</style>
