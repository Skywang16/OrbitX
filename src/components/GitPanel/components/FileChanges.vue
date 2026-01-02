<script setup lang="ts">
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import type { FileChange } from '@/api/git/types'

  interface Props {
    staged: FileChange[]
    modified: FileChange[]
    untracked: FileChange[]
    conflicted: FileChange[]
    selectedFile: FileChange | null
    selectedIsStaged: boolean
  }

  interface Emits {
    (e: 'select', file: FileChange, staged: boolean): void
    (e: 'stage', file: FileChange): void
    (e: 'unstage', file: FileChange): void
    (e: 'discard', file: FileChange): void
    (e: 'stageAll'): void
    (e: 'unstageAll'): void
    (e: 'discardAll'): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const hasStaged = computed(() => props.staged.length > 0)
  const hasUnstaged = computed(() => props.modified.length > 0 || props.untracked.length > 0)
  const hasConflicted = computed(() => props.conflicted.length > 0)

  const select = (file: FileChange, staged: boolean) => {
    emit('select', file, staged)
  }

  const isSelected = (file: FileChange, staged: boolean) => {
    return props.selectedFile?.path === file.path && props.selectedIsStaged === staged
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

  const getBadgeLabel = (file: FileChange) => {
    switch (file.status) {
      case 'added':
        return 'A'
      case 'modified':
        return 'M'
      case 'deleted':
        return 'D'
      case 'renamed':
        return 'R'
      case 'copied':
        return 'C'
      case 'untracked':
        return 'U'
      case 'conflicted':
        return '!'
      default:
        return '?'
    }
  }

  const getBadgeClass = (file: FileChange) => {
    switch (file.status) {
      case 'added':
        return 'badge--added'
      case 'modified':
        return 'badge--modified'
      case 'deleted':
        return 'badge--deleted'
      case 'renamed':
        return 'badge--renamed'
      case 'copied':
        return 'badge--copied'
      case 'untracked':
        return 'badge--untracked'
      case 'conflicted':
        return 'badge--conflicted'
      default:
        return ''
    }
  }

  const stopPropagation = (e: Event) => {
    e.stopPropagation()
  }
</script>

<template>
  <div class="changes">
    <!-- Conflicted Files -->
    <details class="section" open>
      <summary class="section__header section__header--conflicted">
        <span class="section__caret">▸</span>
        <span class="section__title">{{ t('git.merge_conflicts') }}</span>
        <span class="section__count">{{ conflicted.length }}</span>
      </summary>
      <div class="section__content">
        <div v-if="!hasConflicted" class="section__empty">{{ t('git.no_files') }}</div>
        <div
          v-for="file in conflicted"
          v-else
          :key="`conflicted:${file.path}`"
          class="file-item file-item--conflicted"
          :class="{ 'file-item--selected': isSelected(file, false) }"
          @click="select(file, false)"
        >
          <span class="file-badge badge--conflicted">!</span>
          <span class="file-name">{{ getFileName(file.path) }}</span>
          <span v-if="getDirectory(file.path)" class="file-dir">{{ getDirectory(file.path) }}</span>
        </div>
      </div>
    </details>

    <!-- Staged Changes -->
    <details class="section" open>
      <summary class="section__header">
        <span class="section__caret">▸</span>
        <span class="section__title">{{ t('git.staged_changes') }}</span>
        <span class="section__count">{{ staged.length }}</span>
        <div class="section__actions" @click="stopPropagation">
          <button class="icon-btn" :title="t('git.unstage_all')" :disabled="!hasStaged" @click="emit('unstageAll')">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
          </button>
        </div>
      </summary>
      <div class="section__content">
        <div v-if="!hasStaged" class="section__empty">{{ t('git.no_files') }}</div>
        <div
          v-for="file in staged"
          v-else
          :key="`staged:${file.path}`"
          class="file-item"
          :class="{ 'file-item--selected': isSelected(file, true) }"
          @click="select(file, true)"
        >
          <span class="file-badge" :class="getBadgeClass(file)">{{ getBadgeLabel(file) }}</span>
          <span class="file-name">{{ getFileName(file.path) }}</span>
          <span v-if="getDirectory(file.path)" class="file-dir">{{ getDirectory(file.path) }}</span>
          <div class="file-actions">
            <button class="icon-btn" :title="t('git.unstage')" @click.stop="emit('unstage', file)">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="5" y1="12" x2="19" y2="12" />
              </svg>
            </button>
          </div>
        </div>
      </div>
    </details>

    <!-- Unstaged Changes -->
    <details class="section" open>
      <summary class="section__header">
        <span class="section__caret">▸</span>
        <span class="section__title">{{ t('git.changes') }}</span>
        <span class="section__count">{{ modified.length + untracked.length }}</span>
        <div class="section__actions" @click="stopPropagation">
          <button class="icon-btn" :title="t('git.discard_all')" :disabled="!hasUnstaged" @click="emit('discardAll')">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M3 6h18" />
              <path d="M8 6V4h8v2" />
              <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" />
            </svg>
          </button>
          <button class="icon-btn" :title="t('git.stage_all')" :disabled="!hasUnstaged" @click="emit('stageAll')">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="12" y1="5" x2="12" y2="19" />
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
          </button>
        </div>
      </summary>
      <div class="section__content">
        <div v-if="!hasUnstaged" class="section__empty">{{ t('git.no_files') }}</div>
        <template v-else>
          <div
            v-for="file in modified"
            :key="`modified:${file.path}`"
            class="file-item"
            :class="{ 'file-item--selected': isSelected(file, false) }"
            @click="select(file, false)"
          >
            <span class="file-badge" :class="getBadgeClass(file)">{{ getBadgeLabel(file) }}</span>
            <span class="file-name">{{ getFileName(file.path) }}</span>
            <span v-if="getDirectory(file.path)" class="file-dir">{{ getDirectory(file.path) }}</span>
            <div class="file-actions">
              <button class="icon-btn" :title="t('git.discard')" @click.stop="emit('discard', file)">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M3 6h18" />
                  <path d="M8 6V4h8v2" />
                  <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" />
                </svg>
              </button>
              <button class="icon-btn" :title="t('git.stage')" @click.stop="emit('stage', file)">
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
            @click="select(file, false)"
          >
            <span class="file-badge badge--untracked">U</span>
            <span class="file-name">{{ getFileName(file.path) }}</span>
            <span v-if="getDirectory(file.path)" class="file-dir">{{ getDirectory(file.path) }}</span>
            <div class="file-actions">
              <button class="icon-btn" :title="t('git.discard')" @click.stop="emit('discard', file)">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M3 6h18" />
                  <path d="M8 6V4h8v2" />
                  <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" />
                </svg>
              </button>
              <button class="icon-btn" :title="t('git.stage')" @click.stop="emit('stage', file)">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <line x1="12" y1="5" x2="12" y2="19" />
                  <line x1="5" y1="12" x2="19" y2="12" />
                </svg>
              </button>
            </div>
          </div>
        </template>
      </div>
    </details>
  </div>
</template>

<style scoped>
  .changes {
    padding: 8px 0;
  }

  .section {
    margin-bottom: 4px;
  }

  .section__header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    cursor: pointer;
    list-style: none;
    user-select: none;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-200);
  }

  .section__header::-webkit-details-marker {
    display: none;
  }

  .section__header:hover {
    background: var(--bg-100);
  }

  .section__header--conflicted {
    color: #ef4444;
  }

  details[open] > .section__header .section__caret {
    transform: rotate(90deg);
  }

  .section__caret {
    width: 10px;
    font-size: 10px;
    transition: transform 0.12s ease;
    color: var(--text-300);
  }

  .section__title {
    flex: 1;
  }

  .section__count {
    font-size: 10px;
    font-weight: 500;
    padding: 1px 6px;
    border-radius: 10px;
    background: var(--bg-200);
    color: var(--text-300);
  }

  .section__actions {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: 4px;
  }

  .section__content {
    display: flex;
    flex-direction: column;
  }

  .section__empty {
    padding: 6px 12px 6px 28px;
    font-size: 12px;
    color: var(--text-300);
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 12px 4px 28px;
    cursor: pointer;
    font-size: 12px;
    color: var(--text-100);
    transition: background 0.1s ease;
  }

  .file-item:hover {
    background: var(--bg-100);
  }

  .file-item:hover .file-actions {
    opacity: 1;
  }

  .file-item--selected {
    background: var(--bg-200);
  }

  .file-item--conflicted {
    color: #ef4444;
  }

  .file-badge {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    font-size: 10px;
    font-weight: 700;
    font-family: var(--font-mono);
  }

  .badge--added {
    background: rgba(34, 197, 94, 0.2);
    color: #22c55e;
  }

  .badge--modified {
    background: rgba(234, 179, 8, 0.2);
    color: #eab308;
  }

  .badge--deleted {
    background: rgba(239, 68, 68, 0.2);
    color: #ef4444;
  }

  .badge--renamed {
    background: rgba(59, 130, 246, 0.2);
    color: #60a5fa;
  }

  .badge--copied {
    background: rgba(168, 85, 247, 0.2);
    color: #c084fc;
  }

  .badge--untracked {
    background: rgba(148, 163, 184, 0.15);
    color: #94a3b8;
  }

  .badge--conflicted {
    background: rgba(239, 68, 68, 0.25);
    color: #ef4444;
  }

  .file-name {
    flex-shrink: 0;
    font-family: var(--font-mono);
  }

  .file-dir {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11px;
    color: var(--text-300);
    font-family: var(--font-mono);
  }

  .file-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: auto;
    opacity: 0;
    transition: opacity 0.1s ease;
  }

  .icon-btn {
    width: 22px;
    height: 22px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-200);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.1s ease;
  }

  .icon-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .icon-btn:hover {
    background: var(--bg-300);
    color: var(--text-100);
  }

  .icon-btn svg {
    width: 14px;
    height: 14px;
  }
</style>
