<template>
  <div class="empty-state">
    <!-- Action Cards -->
    <div class="action-cards">
      <div class="action-card" @click="handleNewTabClick">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="12" y1="5" x2="12" y2="19"></line>
          <line x1="5" y1="12" x2="19" y2="12"></line>
        </svg>
        <span>{{ t('shortcuts.actions.new_tab') }}</span>
      </div>
      <div class="action-card" @click="toggleCloneInput">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
          <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
        </svg>
        <span>{{ t('shortcuts.actions.clone_repository') || '克隆仓库' }}</span>
      </div>
    </div>

    <!-- Inline Clone Input -->
    <Transition name="slide">
      <div v-if="showCloneInput" class="clone-input-container">
        <div class="clone-input-wrapper">
          <input
            ref="cloneInputRef"
            v-model="gitUrl"
            type="text"
            class="clone-input"
            :class="{ 'is-invalid': gitUrlError }"
            :placeholder="t('shortcuts.git_url_placeholder')"
            @keydown.enter="handleCloneConfirm"
            @keydown.escape="closeCloneInput"
          />
          <div class="clone-actions">
            <button class="clone-btn clone-btn--ghost" @click="closeCloneInput">
              {{ t('common.cancel') }}
            </button>
            <button class="clone-btn clone-btn--primary" @click="handleCloneConfirm">
              {{ t('shortcuts.actions.clone_repository') }}
            </button>
          </div>
        </div>
        <div v-if="gitUrlError" class="input-error">{{ gitUrlError }}</div>
      </div>
    </Transition>

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
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
              <path d="M10 4H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z" />
            </svg>
          </div>
          <div class="workspace-info">
            <div class="workspace-name">{{ getWorkspaceName(workspace.path) }}</div>
            <div class="workspace-path">{{ workspace.path }}</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, onMounted, nextTick } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { shortcutActionsService } from '@/shortcuts/actions'
  import { workspaceApi, type RecentWorkspace } from '@/api/workspace'
  import { useTerminalStore } from '@/stores/Terminal'

  const { t } = useI18n()
  const terminalStore = useTerminalStore()
  const recentWorkspaces = ref<RecentWorkspace[]>([])
  const showCloneInput = ref(false)
  const gitUrl = ref('')
  const gitUrlError = ref('')
  const cloneInputRef = ref<HTMLInputElement>()

  onMounted(async () => {
    try {
      recentWorkspaces.value = await workspaceApi.getRecentWorkspaces(5)
    } catch (error) {
      console.error('Failed to load recent workspaces:', error)
    }
  })

  const toggleCloneInput = () => {
    showCloneInput.value = !showCloneInput.value
    if (showCloneInput.value) {
      gitUrl.value = ''
      gitUrlError.value = ''
      nextTick(() => {
        cloneInputRef.value?.focus()
      })
    }
  }

  const closeCloneInput = () => {
    showCloneInput.value = false
    gitUrl.value = ''
    gitUrlError.value = ''
  }

  const handleNewTabClick = async () => {
    await shortcutActionsService.newTab()
  }

  const isValidGitUrl = (url: string) => {
    const ssh = /^(git@|ssh:\/\/git@)[\w.-]+:[\w.-]+\/[\w.-]+(\.git)?$/
    const https = /^(https?:\/\/)[\w.-]+(:\d+)?\/[\w.-]+\/[\w.-]+(\.git)?(#[\w.-]+)?$/
    return ssh.test(url) || https.test(url)
  }

  const handleCloneConfirm = async () => {
    const finalUrl = gitUrl.value.trim()
    if (!finalUrl) {
      gitUrlError.value = '请输入 Git 仓库地址'
      return
    }
    if (!isValidGitUrl(finalUrl)) {
      gitUrlError.value = '无效的 Git 仓库地址，请输入有效的 HTTPS 或 SSH 地址'
      return
    }
    gitUrlError.value = ''

    try {
      const paneId = await terminalStore.createTerminal(terminalStore.currentWorkingDirectory ?? undefined)
      await terminalStore.writeToTerminal(paneId, `git clone ${finalUrl}\n`)

      closeCloneInput()
    } catch (error) {
      console.error('Failed to clone repository:', error)
    }
  }

  const handleOpenWorkspace = async (path: string) => {
    try {
      await terminalStore.createTerminal(path)
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
    padding: var(--spacing-xl);
  }

  .action-cards {
    display: flex;
    gap: var(--spacing-lg);
    margin-bottom: var(--spacing-xl);
  }

  .action-card {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
    padding: var(--spacing-md) var(--spacing-md);
    width: 140px;
    background: var(--color-primary-alpha);
    border: 1px solid transparent;
    border-radius: var(--border-radius-md);
    cursor: pointer;
    transition: all 0.15s cubic-bezier(0.4, 0, 0.2, 1);
    user-select: none;
  }

  .action-card:hover {
    background: var(--color-primary-alpha);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  }

  .action-card svg {
    color: var(--color-primary);
    transition: transform 0.15s ease;
  }

  .action-card:hover svg {
    transform: scale(1.05);
  }

  .action-card span {
    font-size: var(--font-size-md);
    font-weight: 500;
    color: var(--color-primary);
  }

  /* Clone input inline */
  .clone-input-container {
    width: 100%;
    max-width: 500px;
    margin-bottom: var(--spacing-xl);
  }

  .clone-input-wrapper {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px;
    background: var(--bg-300);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius-md);
  }

  .clone-input {
    flex: 1;
    height: 32px;
    padding: 0 12px;
    font-size: 13px;
    font-family: var(--font-family-mono);
    color: var(--text-100);
    background: var(--bg-400);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius-sm);
    outline: none;
    transition: border-color 0.15s ease;
  }

  .clone-input:focus {
    border-color: var(--color-primary);
  }

  .clone-input.is-invalid {
    border-color: var(--color-error);
  }

  .clone-actions {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }

  .clone-btn {
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 500;
    border-radius: var(--border-radius-sm);
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .clone-btn--ghost {
    color: var(--text-300);
    background: transparent;
    border: none;
  }

  .clone-btn--ghost:hover {
    color: var(--text-100);
  }

  .clone-btn--primary {
    color: var(--bg-100);
    background: var(--color-primary);
    border: none;
  }

  .clone-btn--primary:hover {
    background: var(--color-primary-hover);
  }

  .input-error {
    margin-top: 6px;
    padding-left: 8px;
    color: var(--color-error);
    font-size: 12px;
  }

  /* Transition */
  .slide-enter-active,
  .slide-leave-active {
    transition: all 0.2s ease;
  }

  .slide-enter-from,
  .slide-leave-to {
    opacity: 0;
    transform: translateY(-10px);
  }

  .recent-section {
    width: 100%;
    max-width: 500px;
  }

  .section-title {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-300);
    margin: 0 0 var(--spacing-lg) 0;
  }

  .workspace-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
  }

  .workspace-item {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-md) var(--spacing-md);
    background: var(--bg-300);
    border-radius: var(--border-radius-md);
    cursor: pointer;
    transition: background-color 0.15s ease;
    user-select: none;
  }

  .workspace-item:hover {
    background: var(--bg-400);
  }

  .workspace-icon {
    flex-shrink: 0;
    color: var(--text-400);
    display: flex;
    align-items: center;
  }

  .workspace-info {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--spacing-lg);
    min-width: 0;
  }

  .workspace-name {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-200);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .workspace-path {
    font-size: var(--font-size-xs);
    color: var(--text-400);
    font-family: var(--font-family-mono);
    flex-shrink: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    direction: rtl;
    text-align: right;
  }
</style>
