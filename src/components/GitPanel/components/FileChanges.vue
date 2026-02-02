<script setup lang="ts">
  import { computed, ref } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { confirm } from '@tauri-apps/plugin-dialog'
  import type { FileChange } from '@/api/git/types'

  interface Props {
    staged: FileChange[]
    modified: FileChange[]
    untracked: FileChange[]
    conflicted: FileChange[]
  }

  interface Emits {
    (e: 'select', file: FileChange, staged: boolean): void
    (e: 'stage', files: FileChange[]): void
    (e: 'unstage', files: FileChange[]): void
    (e: 'discard', files: FileChange[]): void
    (e: 'stageAll'): void
    (e: 'unstageAll'): void
    (e: 'discardAll'): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const selectedItems = ref<Set<string>>(new Set())
  const lastClickedKey = ref<string | null>(null)

  const hasStaged = computed(() => props.staged.length > 0)
  const hasUnstaged = computed(() => props.modified.length > 0 || props.untracked.length > 0)
  const hasConflicted = computed(() => props.conflicted.length > 0)
  const totalChanges = computed(() => props.staged.length + props.modified.length + props.untracked.length)

  const getItemKey = (file: FileChange, staged: boolean) => `${staged ? 'staged' : 'unstaged'}:${file.path}`
  const unstagedFiles = computed(() => [...props.modified, ...props.untracked])

  const getSelectedFiles = (staged: boolean): FileChange[] => {
    const prefix = staged ? 'staged:' : 'unstaged:'
    const files: FileChange[] = []
    const sourceFiles = staged ? props.staged : unstagedFiles.value

    for (const file of sourceFiles) {
      if (selectedItems.value.has(prefix + file.path)) {
        files.push(file)
      }
    }
    return files
  }

  const hasSelectedStaged = computed(() => getSelectedFiles(true).length > 0)
  const hasSelectedUnstaged = computed(() => getSelectedFiles(false).length > 0)

  const handleClick = (event: MouseEvent, file: FileChange, staged: boolean) => {
    const key = getItemKey(file, staged)
    const sourceFiles = staged ? props.staged : unstagedFiles.value

    if (event.metaKey || event.ctrlKey) {
      if (selectedItems.value.has(key)) {
        selectedItems.value.delete(key)
      } else {
        selectedItems.value.add(key)
      }
      lastClickedKey.value = key
    } else if (event.shiftKey && lastClickedKey.value) {
      const lastPrefix = lastClickedKey.value.split(':')[0]
      const currentPrefix = staged ? 'staged' : 'unstaged'

      if (lastPrefix === currentPrefix) {
        const lastPath = lastClickedKey.value.substring(lastPrefix.length + 1)
        const lastIndex = sourceFiles.findIndex(f => f.path === lastPath)
        const currentIndex = sourceFiles.findIndex(f => f.path === file.path)

        if (lastIndex !== -1 && currentIndex !== -1) {
          const start = Math.min(lastIndex, currentIndex)
          const end = Math.max(lastIndex, currentIndex)

          for (let i = start; i <= end; i++) {
            selectedItems.value.add(getItemKey(sourceFiles[i], staged))
          }
        }
      }
    } else {
      selectedItems.value.clear()
      selectedItems.value.add(key)
      lastClickedKey.value = key
      emit('select', file, staged)
    }
  }

  const isSelected = (file: FileChange, staged: boolean) => {
    return selectedItems.value.has(getItemKey(file, staged))
  }

  const getFileName = (path: string) => {
    const parts = path.split('/')
    return parts[parts.length - 1]
  }

  const getDirectory = (path: string) => {
    const parts = path.split('/')
    if (parts.length <= 1) return ''
    return parts.slice(0, -1).join('/')
  }

  const getStatusBadge = (file: FileChange) => {
    const map: Record<string, { label: string; type: string }> = {
      added: { label: 'A', type: 'added' },
      modified: { label: 'M', type: 'modified' },
      typeChanged: { label: 'T', type: 'modified' },
      deleted: { label: 'D', type: 'deleted' },
      renamed: { label: 'R', type: 'renamed' },
      copied: { label: 'C', type: 'copied' },
      untracked: { label: 'U', type: 'untracked' },
      conflicted: { label: '!', type: 'conflicted' },
    }
    return map[file.status] || { label: '?', type: 'unknown' }
  }

  const stopPropagation = (e: Event) => e.stopPropagation()

  const stageSelected = () => {
    const files = getSelectedFiles(false)
    if (files.length > 0) {
      emit('stage', files)
      selectedItems.value.clear()
    }
  }

  const unstageSelected = () => {
    const files = getSelectedFiles(true)
    if (files.length > 0) {
      emit('unstage', files)
      selectedItems.value.clear()
    }
  }

  const discardSelected = async () => {
    const files = getSelectedFiles(false)
    if (files.length === 0) return

    const message =
      files.length === 1
        ? t('git.discard_confirm', { file: getFileName(files[0].path) })
        : t('git.discard_selected_confirm', { count: files.length })

    const confirmed = await confirm(message, { title: t('git.discard'), kind: 'warning' })
    if (confirmed) {
      emit('discard', files)
      selectedItems.value.clear()
    }
  }

  const stageFile = (file: FileChange) => emit('stage', [file])
  const unstageFile = (file: FileChange) => emit('unstage', [file])

  const confirmDiscard = async (file: FileChange) => {
    const message = t('git.discard_confirm', { file: getFileName(file.path) })
    const confirmed = await confirm(message, { title: t('git.discard'), kind: 'warning' })
    if (confirmed) emit('discard', [file])
  }

  const confirmDiscardAll = async () => {
    const message = t('git.discard_all_confirm')
    const confirmed = await confirm(message, { title: t('git.discard_all'), kind: 'warning' })
    if (confirmed) emit('discardAll')
  }
</script>

<template>
  <div class="file-changes">
    <!-- Conflicted Section -->
    <div v-if="hasConflicted" class="change-section change-section--conflict">
      <div class="section-header">
        <div class="section-header__left">
          <svg class="section-icon section-icon--conflict" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 2L2 22h20L12 2zm0 4l7.53 14H4.47L12 6zm-1 6v4h2v-4h-2zm0 6v2h2v-2h-2z" />
          </svg>
          <span class="section-title">{{ t('git.merge_conflicts') }}</span>
        </div>
        <span class="section-badge section-badge--conflict">{{ conflicted.length }}</span>
      </div>
      <div class="file-list">
        <div
          v-for="file in conflicted"
          :key="`conflict:${file.path}`"
          class="file-item file-item--conflict"
          :class="{ 'file-item--selected': isSelected(file, false) }"
          @click="handleClick($event, file, false)"
        >
          <span class="file-badge file-badge--conflicted">!</span>
          <div class="file-info">
            <span class="file-name">{{ getFileName(file.path) }}</span>
            <span v-if="getDirectory(file.path)" class="file-dir">{{ getDirectory(file.path) }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Staged Section -->
    <div v-if="hasStaged" class="change-section">
      <div class="section-header">
        <div class="section-header__left">
          <svg
            class="section-icon section-icon--staged"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <polyline points="20 6 9 17 4 12" />
          </svg>
          <span class="section-title">{{ t('git.staged_changes') }}</span>
        </div>
        <span class="section-badge section-badge--staged">{{ staged.length }}</span>
        <div class="section-actions" @click="stopPropagation">
          <button
            class="action-btn action-btn--unstage"
            :title="hasSelectedStaged ? t('git.unstage_selected') : t('git.unstage_all')"
            @click="hasSelectedStaged ? unstageSelected() : emit('unstageAll')"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
          </button>
        </div>
      </div>
      <div class="file-list">
        <div
          v-for="file in staged"
          :key="`staged:${file.path}`"
          class="file-item"
          :class="{ 'file-item--selected': isSelected(file, true) }"
          @click="handleClick($event, file, true)"
        >
          <span class="file-badge" :class="`file-badge--${getStatusBadge(file).type}`">
            {{ getStatusBadge(file).label }}
          </span>
          <div class="file-info">
            <span class="file-name">{{ getFileName(file.path) }}</span>
            <span v-if="getDirectory(file.path)" class="file-dir">{{ getDirectory(file.path) }}</span>
          </div>
          <div class="file-actions">
            <button class="file-btn" :title="t('git.unstage')" @click.stop="unstageFile(file)">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="5" y1="12" x2="19" y2="12" />
              </svg>
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Unstaged Section -->
    <div v-if="hasUnstaged" class="change-section">
      <div class="section-header">
        <div class="section-header__left">
          <svg class="section-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10" />
            <path d="M12 8v8m-4-4h8" />
          </svg>
          <span class="section-title">{{ t('git.changes') }}</span>
        </div>
        <span class="section-badge">{{ modified.length + untracked.length }}</span>
        <div class="section-actions" @click="stopPropagation">
          <button
            class="action-btn action-btn--discard"
            :title="hasSelectedUnstaged ? t('git.discard_selected') : t('git.discard_all')"
            @click="hasSelectedUnstaged ? discardSelected() : confirmDiscardAll()"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M3 6h18M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2" />
            </svg>
          </button>
          <button
            class="action-btn action-btn--stage"
            :title="hasSelectedUnstaged ? t('git.stage_selected') : t('git.stage_all')"
            @click="hasSelectedUnstaged ? stageSelected() : emit('stageAll')"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="12" y1="5" x2="12" y2="19" />
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
          </button>
        </div>
      </div>
      <div class="file-list">
        <div
          v-for="file in modified"
          :key="`modified:${file.path}`"
          class="file-item"
          :class="{ 'file-item--selected': isSelected(file, false) }"
          @click="handleClick($event, file, false)"
        >
          <span class="file-badge" :class="`file-badge--${getStatusBadge(file).type}`">
            {{ getStatusBadge(file).label }}
          </span>
          <div class="file-info">
            <span class="file-name">{{ getFileName(file.path) }}</span>
            <span v-if="getDirectory(file.path)" class="file-dir">{{ getDirectory(file.path) }}</span>
          </div>
          <div class="file-actions">
            <button class="file-btn file-btn--danger" :title="t('git.discard')" @click.stop="confirmDiscard(file)">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M3 6h18M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2" />
              </svg>
            </button>
            <button class="file-btn" :title="t('git.stage')" @click.stop="stageFile(file)">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="12" y1="5" x2="12" y2="19" />
                <line x1="5" y1="12" x2="19" y2="12" />
              </svg>
            </button>
          </div>
        </div>

        <div
          v-for="file in untracked"
          :key="`untracked:${file.path}`"
          class="file-item"
          :class="{ 'file-item--selected': isSelected(file, false) }"
          @click="handleClick($event, file, false)"
        >
          <span class="file-badge file-badge--untracked">U</span>
          <div class="file-info">
            <span class="file-name">{{ getFileName(file.path) }}</span>
            <span v-if="getDirectory(file.path)" class="file-dir">{{ getDirectory(file.path) }}</span>
          </div>
          <div class="file-actions">
            <button class="file-btn file-btn--danger" :title="t('git.discard')" @click.stop="confirmDiscard(file)">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M3 6h18M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2" />
              </svg>
            </button>
            <button class="file-btn" :title="t('git.stage')" @click.stop="stageFile(file)">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="12" y1="5" x2="12" y2="19" />
                <line x1="5" y1="12" x2="19" y2="12" />
              </svg>
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Empty State -->
    <div v-if="!hasStaged && !hasUnstaged && !hasConflicted" class="empty-state">
      <svg class="empty-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
      <span>{{ t('git.no_changes') }}</span>
    </div>
  </div>
</template>

<style scoped>
  .file-changes {
    padding: 8px;
  }

  /* Section */
  .change-section {
    margin-bottom: 12px;
  }

  .change-section:last-child {
    margin-bottom: 0;
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    background: var(--bg-100);
    border-radius: 8px 8px 0 0;
    border: 1px solid var(--border-100);
    border-bottom: none;
  }

  .section-header__left {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
  }

  .section-icon {
    width: 14px;
    height: 14px;
    color: var(--text-400);
    flex-shrink: 0;
  }

  .section-icon--staged {
    color: var(--color-success);
  }

  .section-icon--conflict {
    color: var(--color-error);
  }

  .section-title {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-300);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .section-badge {
    font-size: 11px;
    font-weight: 500;
    padding: 2px 8px;
    border-radius: 10px;
    background: var(--bg-200);
    color: var(--text-400);
  }

  .section-badge--staged {
    background: color-mix(in srgb, var(--color-success) 15%, transparent);
    color: var(--color-success);
  }

  .section-badge--conflict {
    background: color-mix(in srgb, var(--color-error) 15%, transparent);
    color: var(--color-error);
  }

  .section-actions {
    display: flex;
    gap: 4px;
    margin-left: auto;
  }

  .action-btn {
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 6px;
    background: var(--bg-200);
    color: var(--text-400);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-btn:hover {
    background: var(--bg-300);
    color: var(--text-100);
  }

  .action-btn--stage:hover {
    background: color-mix(in srgb, var(--color-success) 20%, transparent);
    color: var(--color-success);
  }

  .action-btn--unstage:hover {
    background: color-mix(in srgb, var(--color-warning) 20%, transparent);
    color: var(--color-warning);
  }

  .action-btn--discard:hover {
    background: color-mix(in srgb, var(--color-error) 20%, transparent);
    color: var(--color-error);
  }

  .action-btn svg {
    width: 14px;
    height: 14px;
  }

  /* File List */
  .file-list {
    background: var(--bg-50);
    border: 1px solid var(--border-100);
    border-top: none;
    border-radius: 0 0 8px 8px;
    overflow: hidden;
    max-height: 180px;
    overflow-y: auto;
  }

  .file-list::-webkit-scrollbar {
    width: 6px;
  }

  .file-list::-webkit-scrollbar-track {
    background: transparent;
  }

  .file-list::-webkit-scrollbar-thumb {
    background: var(--border-300);
    border-radius: 3px;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    cursor: pointer;
    transition: background 0.12s ease;
    border-bottom: 1px solid var(--border-100);
  }

  .file-item:last-child {
    border-bottom: none;
  }

  .file-item:hover {
    background: var(--bg-200);
  }

  .file-item:hover .file-actions {
    opacity: 1;
  }

  .file-item--selected {
    background: color-mix(in srgb, var(--color-primary) 10%, transparent);
  }

  .file-item--selected:hover {
    background: color-mix(in srgb, var(--color-primary) 15%, transparent);
  }

  .file-item--selected .file-actions {
    opacity: 1;
  }

  .file-item--conflict {
    background: color-mix(in srgb, var(--color-error) 5%, transparent);
  }

  /* File Badge */
  .file-badge {
    flex-shrink: 0;
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 600;
    font-family: var(--font-family-mono);
  }

  .file-badge--added {
    background: color-mix(in srgb, var(--color-success) 15%, transparent);
    color: var(--color-success);
  }

  .file-badge--modified {
    background: color-mix(in srgb, var(--color-warning) 15%, transparent);
    color: var(--color-warning);
  }

  .file-badge--deleted {
    background: color-mix(in srgb, var(--color-error) 15%, transparent);
    color: var(--color-error);
  }

  .file-badge--renamed {
    background: color-mix(in srgb, var(--color-info) 15%, transparent);
    color: var(--color-info);
  }

  .file-badge--copied {
    background: color-mix(in srgb, var(--color-primary) 15%, transparent);
    color: var(--color-primary);
  }

  .file-badge--untracked {
    background: var(--bg-200);
    color: var(--text-400);
  }

  .file-badge--conflicted {
    background: color-mix(in srgb, var(--color-error) 20%, transparent);
    color: var(--color-error);
  }

  /* File Info */
  .file-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .file-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-100);
    font-family: var(--font-family-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-dir {
    font-size: 11px;
    color: var(--text-500);
    font-family: var(--font-family-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* File Actions */
  .file-actions {
    display: flex;
    gap: 4px;
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .file-btn {
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-400);
    cursor: pointer;
    transition: all 0.12s ease;
  }

  .file-btn:hover {
    background: var(--bg-300);
    color: var(--color-success);
  }

  .file-btn--danger:hover {
    background: color-mix(in srgb, var(--color-error) 15%, transparent);
    color: var(--color-error);
  }

  .file-btn svg {
    width: 14px;
    height: 14px;
  }

  /* Empty State */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 32px 20px;
    color: var(--text-500);
    font-size: 13px;
  }

  .empty-icon {
    width: 36px;
    height: 36px;
    opacity: 0.4;
  }
</style>
