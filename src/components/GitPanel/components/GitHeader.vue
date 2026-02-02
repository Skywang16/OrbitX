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

  const handleActionsClick = async (event: MouseEvent) => {
    await showContextMenu({
      x: event.clientX,
      y: event.clientY,
      items: [
        {
          label: 'Fetch',
          onClick: () => emit('fetch'),
        },
        {
          label: 'Pull',
          onClick: () => emit('pull'),
        },
        {
          label: 'Push',
          onClick: () => emit('push'),
        },
      ],
    })
  }
</script>

<template>
  <div class="git-header">
    <!-- Branch Row -->
    <div class="header-row">
      <button
        class="branch-btn"
        :class="{ 'branch-btn--clickable': localBranches.length > 0 }"
        @click="handleBranchClick"
      >
        <svg class="branch-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M6 3v12" />
          <circle cx="18" cy="6" r="3" />
          <circle cx="6" cy="18" r="3" />
          <path d="M18 9a9 9 0 0 1-9 9" />
        </svg>
        <span class="branch-name">{{ branchText }}</span>
        <svg v-if="localBranches.length > 0" class="branch-caret" viewBox="0 0 24 24" fill="currentColor">
          <path d="m6 9 6 6 6-6" />
        </svg>
      </button>

      <!-- Sync Status -->
      <div v-if="hasRemoteChanges" class="sync-status">
        <span v-if="aheadCount > 0" class="sync-badge sync-badge--ahead">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
            <path d="M12 19V5m-7 7 7-7 7 7" />
          </svg>
          {{ aheadCount }}
        </span>
        <span v-if="behindCount > 0" class="sync-badge sync-badge--behind">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
            <path d="M12 5v14m-7-7 7 7 7-7" />
          </svg>
          {{ behindCount }}
        </span>
      </div>

      <!-- Actions Menu -->
      <button class="actions-btn" :title="t('git.actions')" @click="handleActionsClick">
        <svg viewBox="0 0 24 24" fill="currentColor">
          <circle cx="12" cy="5" r="2" />
          <circle cx="12" cy="12" r="2" />
          <circle cx="12" cy="19" r="2" />
        </svg>
      </button>
    </div>

    <!-- Commit Input -->
    <div class="commit-row">
      <input
        v-model="commitMessage"
        type="text"
        class="commit-input"
        :placeholder="t('git.commit_message_placeholder')"
        @keydown.enter.prevent="submitCommit"
      />
      <button
        class="commit-btn"
        :class="{ 'commit-btn--active': canCommit }"
        :disabled="!canCommit"
        @click="submitCommit"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
          <polyline points="20 6 9 17 4 12" />
        </svg>
      </button>
    </div>

    <!-- Staged hint -->
    <div v-if="props.stagedCount > 0" class="commit-hint">
      {{ props.stagedCount }} {{ props.stagedCount === 1 ? 'file' : 'files' }} staged
    </div>
  </div>
</template>

<style scoped>
  .git-header {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    border-bottom: 1px solid var(--border-100);
    background: var(--bg-50);
  }

  .header-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .branch-btn {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--bg-100);
    border: 1px solid var(--border-100);
    border-radius: 8px;
    color: var(--text-100);
    font-size: 13px;
    font-weight: 500;
    cursor: default;
    transition: all 0.15s ease;
  }

  .branch-btn--clickable {
    cursor: pointer;
  }

  .branch-btn--clickable:hover {
    background: var(--bg-200);
    border-color: var(--border-200);
  }

  .branch-icon {
    width: 16px;
    height: 16px;
    color: var(--text-400);
    flex-shrink: 0;
  }

  .branch-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .branch-caret {
    width: 14px;
    height: 14px;
    color: var(--text-500);
    flex-shrink: 0;
  }

  /* Sync Status */
  .sync-status {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }

  .sync-badge {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 10px;
    border-radius: 8px;
    font-size: 12px;
    font-weight: 500;
  }

  .sync-badge svg {
    width: 12px;
    height: 12px;
  }

  .sync-badge--ahead {
    background: color-mix(in srgb, var(--color-success) 12%, transparent);
    color: var(--color-success);
  }

  .sync-badge--behind {
    background: color-mix(in srgb, var(--color-info) 12%, transparent);
    color: var(--color-info);
  }

  /* Actions Button */
  .actions-btn {
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    background: var(--bg-100);
    border: 1px solid var(--border-100);
    border-radius: 8px;
    color: var(--text-400);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .actions-btn:hover {
    background: var(--bg-200);
    border-color: var(--border-200);
    color: var(--text-200);
  }

  .actions-btn svg {
    width: 16px;
    height: 16px;
  }

  /* Commit Row */
  .commit-row {
    display: flex;
    gap: 8px;
  }

  .commit-input {
    flex: 1;
    min-width: 0;
    height: 36px;
    padding: 0 12px;
    background: var(--bg-100);
    border: 1px solid var(--border-100);
    border-radius: 8px;
    color: var(--text-100);
    font-size: 13px;
    outline: none;
    transition: all 0.15s ease;
  }

  .commit-input::placeholder {
    color: var(--text-500);
  }

  .commit-input:focus {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 12%, transparent);
  }

  .commit-btn {
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    background: var(--bg-200);
    border: 1px solid var(--border-100);
    border-radius: 8px;
    color: var(--text-500);
    cursor: not-allowed;
    transition: all 0.15s ease;
  }

  .commit-btn--active {
    background: var(--color-primary);
    border-color: var(--color-primary);
    color: white;
    cursor: pointer;
  }

  .commit-btn--active:hover {
    background: var(--color-primary-hover);
    transform: translateY(-1px);
  }

  .commit-btn svg {
    width: 18px;
    height: 18px;
  }

  .commit-hint {
    font-size: 11px;
    color: var(--text-500);
    padding-left: 2px;
  }
</style>
