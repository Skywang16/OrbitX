<script setup lang="ts">
  import { computed, ref } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { showContextMenu } from '@/ui/composables/popover-api'
  import type { BranchInfo } from '@/api/git/types'

  interface Props {
    branch: string | null
    ahead?: number | null
    behind?: number | null
    stagedCount: number
    branches?: BranchInfo[]
  }

  interface Emits {
    (e: 'commit', message: string): void
    (e: 'push'): void
    (e: 'pull'): void
    (e: 'sync'): void
    (e: 'fetch'): void
    (e: 'checkout', branchName: string): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const branchText = computed(() => props.branch || t('git.unknown_branch'))
  const aheadCount = computed(() => props.ahead ?? 0)
  const behindCount = computed(() => props.behind ?? 0)

  const hasRemoteChanges = computed(() => aheadCount.value > 0 || behindCount.value > 0)

  const commitMessage = ref('')
  const canCommit = computed(() => props.stagedCount > 0 && commitMessage.value.trim().length > 0)

  const submitCommit = () => {
    if (!canCommit.value) return
    emit('commit', commitMessage.value.trim())
    commitMessage.value = ''
  }

  const localBranches = computed(() => props.branches?.filter(b => !b.isRemote) ?? [])

  const handleBranchClick = async (event: MouseEvent) => {
    if (localBranches.value.length === 0) return

    const items = localBranches.value.map(b => ({
      label: b.isCurrent ? `✓ ${b.name}` : `   ${b.name}`,
      value: b.name,
      disabled: b.isCurrent,
      onClick: () => emit('checkout', b.name),
    }))

    await showContextMenu({
      x: event.clientX,
      y: event.clientY,
      items,
    })
  }
</script>

<template>
  <div class="git-header">
    <div class="git-header__top">
      <div
        class="git-header__branch"
        :class="{ 'git-header__branch--clickable': localBranches.length > 0 }"
        @click="handleBranchClick"
      >
        <svg class="git-header__branch-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M6 3v12" />
          <circle cx="18" cy="6" r="3" />
          <circle cx="6" cy="18" r="3" />
          <path d="M18 9a9 9 0 0 1-9 9" />
        </svg>
        <span class="git-header__branch-name">{{ branchText }}</span>
        <svg
          v-if="localBranches.length > 0"
          class="git-header__branch-caret"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <path d="m6 9 6 6 6-6" />
        </svg>
        <span v-if="hasRemoteChanges" class="git-header__sync">
          <span v-if="aheadCount > 0" class="git-header__sync-item git-header__sync-item--ahead">
            ↑{{ aheadCount }}
          </span>
          <span v-if="behindCount > 0" class="git-header__sync-item git-header__sync-item--behind">
            ↓{{ behindCount }}
          </span>
        </span>
      </div>
    </div>

    <div class="git-header__actions">
      <div class="git-header__commit">
        <input
          v-model="commitMessage"
          class="git-header__commit-input"
          :placeholder="t('git.commit')"
          @keydown.enter.prevent="submitCommit"
        />
        <button
          class="action-btn action-btn--primary"
          :disabled="!canCommit"
          :title="t('git.commit')"
          @click="submitCommit"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="4" />
            <line x1="1.05" y1="12" x2="7" y2="12" />
            <line x1="17.01" y1="12" x2="22.96" y2="12" />
          </svg>
          <span>{{ t('git.commit') }}</span>
        </button>
      </div>

      <div class="action-group">
        <button class="action-btn" :title="t('git.pull')" @click="emit('pull')">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 5v14" />
            <path d="m19 12-7 7-7-7" />
          </svg>
        </button>

        <button class="action-btn" :title="t('git.push')" @click="emit('push')">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 19V5" />
            <path d="m5 12 7-7 7 7" />
          </svg>
        </button>

        <button class="action-btn" :title="t('git.sync')" @click="emit('sync')">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M17 2l4 4-4 4" />
            <path d="M3 11V9a4 4 0 0 1 4-4h14" />
            <path d="M7 22l-4-4 4-4" />
            <path d="M21 13v2a4 4 0 0 1-4 4H3" />
          </svg>
        </button>

        <button class="action-btn" :title="t('git.fetch')" @click="emit('fetch')">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
            <polyline points="7 10 12 15 17 10" />
            <line x1="12" y1="15" x2="12" y2="3" />
          </svg>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .git-header {
    padding: 12px;
    border-bottom: 1px solid var(--border-200);
    background: var(--bg-100);
  }

  .git-header__top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .git-header__branch {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .git-header__branch-icon {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    color: var(--text-200);
  }

  .git-header__branch-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-100);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .git-header__branch--clickable {
    cursor: pointer;
    padding: 4px 8px;
    margin: -4px -8px;
    border-radius: 6px;
    transition: background 0.15s ease;
  }

  .git-header__branch--clickable:hover {
    background: var(--bg-200);
  }

  .git-header__branch-caret {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    color: var(--text-300);
  }

  .git-header__sync {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .git-header__sync-item {
    font-size: 11px;
    font-weight: 500;
    padding: 1px 4px;
    border-radius: 4px;
  }

  .git-header__sync-item--ahead {
    background: rgba(34, 197, 94, 0.15);
    color: #22c55e;
  }

  .git-header__sync-item--behind {
    background: rgba(59, 130, 246, 0.15);
    color: #60a5fa;
  }

  .git-header__actions {
    margin-top: 10px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .git-header__commit {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .git-header__commit-input {
    flex: 1;
    min-width: 0;
    height: 28px;
    padding: 0 10px;
    border-radius: 6px;
    border: 1px solid var(--border-200);
    background: var(--bg-50);
    color: var(--text-100);
    font-size: 12px;
    outline: none;
  }

  .git-header__commit-input:focus {
    border-color: var(--border-300);
    background: var(--bg-100);
  }

  .git-header__commit-input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .action-btn {
    height: 28px;
    padding: 0 10px;
    border-radius: 6px;
    border: 1px solid var(--border-200);
    background: var(--bg-50);
    color: var(--text-200);
    font-size: 12px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-btn:hover {
    background: var(--bg-200);
    color: var(--text-100);
  }

  .action-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .action-btn svg {
    width: 14px;
    height: 14px;
  }

  .action-btn--primary {
    background: var(--bg-200);
    border-color: var(--border-300);
  }

  .action-btn--primary:not(:disabled):hover {
    background: var(--bg-300);
  }

  .action-group {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: auto;
  }

  .action-group .action-btn {
    padding: 0 8px;
  }
</style>
