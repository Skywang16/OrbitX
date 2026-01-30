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

  // 多选状态
  const selectedItems = ref<Set<string>>(new Set())
  const lastClickedKey = ref<string | null>(null)

  const hasStaged = computed(() => props.staged.length > 0)
  const hasUnstaged = computed(() => props.modified.length > 0 || props.untracked.length > 0)
  const hasConflicted = computed(() => props.conflicted.length > 0)

  // 生成唯一 key
  const getItemKey = (file: FileChange, staged: boolean) => `${staged ? 'staged' : 'unstaged'}:${file.path}`

  // 获取所有 unstaged 文件列表（按渲染顺序）
  const unstagedFiles = computed(() => [...props.modified, ...props.untracked])

  // 获取选中的文件
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

  // 点击选择逻辑
  const handleClick = (event: MouseEvent, file: FileChange, staged: boolean) => {
    const key = getItemKey(file, staged)
    const sourceFiles = staged ? props.staged : unstagedFiles.value

    if (event.metaKey || event.ctrlKey) {
      // Ctrl/Cmd 点击：切换单个选择
      if (selectedItems.value.has(key)) {
        selectedItems.value.delete(key)
      } else {
        selectedItems.value.add(key)
      }
      lastClickedKey.value = key
    } else if (event.shiftKey && lastClickedKey.value) {
      // Shift 点击：范围选择
      const lastPrefix = lastClickedKey.value.split(':')[0]
      const currentPrefix = staged ? 'staged' : 'unstaged'

      // 只在同一区域内进行范围选择
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
      // 普通点击：清除选择，选中当前项，打开 diff
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

  const getBadgeLabel = (file: FileChange) => {
    switch (file.status) {
      case 'added':
        return 'A'
      case 'modified':
        return 'M'
      case 'typeChanged':
        return 'T'
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
      case 'unknown':
        return '?'
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
      case 'typeChanged':
        return 'badge--type-changed'
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
      case 'unknown':
        return 'badge--unknown'
      default:
        return ''
    }
  }

  const stopPropagation = (e: Event) => {
    e.stopPropagation()
  }

  // 批量操作
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

  // 单文件操作
  const stageFile = (file: FileChange) => {
    emit('stage', [file])
  }

  const unstageFile = (file: FileChange) => {
    emit('unstage', [file])
  }

  const confirmDiscard = async (file: FileChange) => {
    const message = t('git.discard_confirm', { file: getFileName(file.path) })
    const confirmed = await confirm(message, { title: t('git.discard'), kind: 'warning' })
    if (confirmed) {
      emit('discard', [file])
    }
  }

  const confirmDiscardAll = async () => {
    const message = t('git.discard_all_confirm')
    const confirmed = await confirm(message, { title: t('git.discard_all'), kind: 'warning' })
    if (confirmed) {
      emit('discardAll')
    }
  }
</script>

<template>
  <div class="changes">
    <!-- Conflicted Files -->
    <details v-if="hasConflicted" class="section" open>
      <summary class="section__header section__header--conflicted">
        <span class="section__caret">▸</span>
        <span class="section__title">{{ t('git.merge_conflicts') }}</span>
        <span class="section__count">{{ conflicted.length }}</span>
      </summary>
      <div class="section__content">
        <div
          v-for="file in conflicted"
          :key="`conflicted:${file.path}`"
          class="file-item file-item--conflicted"
          :class="{ 'file-item--selected': isSelected(file, false) }"
          @click="handleClick($event, file, false)"
        >
          <span class="file-badge badge--conflicted">!</span>
          <span class="file-name">{{ getFileName(file.path) }}</span>
          <span v-if="getDirectory(file.path)" class="file-dir">{{ getDirectory(file.path) }}</span>
        </div>
      </div>
    </details>

    <!-- Staged Changes -->
    <details v-if="hasStaged" class="section" open>
      <summary class="section__header">
        <span class="section__caret">▸</span>
        <span class="section__title">{{ t('git.staged_changes') }}</span>
        <span class="section__count">{{ staged.length }}</span>
        <div class="section__actions" @click="stopPropagation">
          <button v-if="hasSelectedStaged" class="icon-btn" :title="t('git.unstage_selected')" @click="unstageSelected">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
          </button>
          <button v-else class="icon-btn" :title="t('git.unstage_all')" @click="emit('unstageAll')">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
          </button>
        </div>
      </summary>
      <div class="section__content">
        <div
          v-for="file in staged"
          :key="`staged:${file.path}`"
          class="file-item"
          :class="{ 'file-item--selected': isSelected(file, true) }"
          @click="handleClick($event, file, true)"
        >
          <span class="file-badge" :class="getBadgeClass(file)">{{ getBadgeLabel(file) }}</span>
          <span class="file-name">{{ getFileName(file.path) }}</span>
          <span v-if="getDirectory(file.path)" class="file-dir">{{ getDirectory(file.path) }}</span>
          <div class="file-actions">
            <button class="icon-btn" :title="t('git.unstage')" @click.stop="unstageFile(file)">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="5" y1="12" x2="19" y2="12" />
              </svg>
            </button>
          </div>
        </div>
      </div>
    </details>

    <!-- Unstaged Changes -->
    <details v-if="hasUnstaged" class="section" open>
      <summary class="section__header">
        <span class="section__caret">▸</span>
        <span class="section__title">{{ t('git.changes') }}</span>
        <span class="section__count">{{ modified.length + untracked.length }}</span>
        <div class="section__actions" @click="stopPropagation">
          <template v-if="hasSelectedUnstaged">
            <button class="icon-btn" :title="t('git.discard_selected')" @click="discardSelected">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M3 7v6h6" />
                <path d="M21 17a9 9 0 0 0-9-9 9 9 0 0 0-6 2.3L3 13" />
              </svg>
            </button>
            <button class="icon-btn" :title="t('git.stage_selected')" @click="stageSelected">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="12" y1="5" x2="12" y2="19" />
                <line x1="5" y1="12" x2="19" y2="12" />
              </svg>
            </button>
          </template>
          <template v-else>
            <button class="icon-btn" :title="t('git.discard_all')" @click="confirmDiscardAll">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M3 7v6h6" />
                <path d="M21 17a9 9 0 0 0-9-9 9 9 0 0 0-6 2.3L3 13" />
              </svg>
            </button>
            <button class="icon-btn" :title="t('git.stage_all')" @click="emit('stageAll')">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="12" y1="5" x2="12" y2="19" />
                <line x1="5" y1="12" x2="19" y2="12" />
              </svg>
            </button>
          </template>
        </div>
      </summary>
      <div class="section__content">
        <div
          v-for="file in modified"
          :key="`modified:${file.path}`"
          class="file-item"
          :class="{ 'file-item--selected': isSelected(file, false) }"
          @click="handleClick($event, file, false)"
        >
          <span class="file-badge" :class="getBadgeClass(file)">{{ getBadgeLabel(file) }}</span>
          <span class="file-name">{{ getFileName(file.path) }}</span>
          <span v-if="getDirectory(file.path)" class="file-dir">{{ getDirectory(file.path) }}</span>
          <div class="file-actions">
            <button class="icon-btn" :title="t('git.discard')" @click.stop="confirmDiscard(file)">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M3 7v6h6" />
                <path d="M21 17a9 9 0 0 0-9-9 9 9 0 0 0-6 2.3L3 13" />
              </svg>
            </button>
            <button class="icon-btn" :title="t('git.stage')" @click.stop="stageFile(file)">
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
          <span class="file-badge badge--untracked">U</span>
          <span class="file-name">{{ getFileName(file.path) }}</span>
          <span v-if="getDirectory(file.path)" class="file-dir">{{ getDirectory(file.path) }}</span>
          <div class="file-actions">
            <button class="icon-btn" :title="t('git.discard')" @click.stop="confirmDiscard(file)">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M3 7v6h6" />
                <path d="M21 17a9 9 0 0 0-9-9 9 9 0 0 0-6 2.3L3 13" />
              </svg>
            </button>
            <button class="icon-btn" :title="t('git.stage')" @click.stop="stageFile(file)">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="12" y1="5" x2="12" y2="19" />
                <line x1="5" y1="12" x2="19" y2="12" />
              </svg>
            </button>
          </div>
        </div>
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
    border-radius: 6px;
  }

  .section__header::-webkit-details-marker {
    display: none;
  }

  .section__header:hover {
    background: var(--color-hover);
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
    background: color-mix(in srgb, var(--color-primary) 25%, transparent);
    color: var(--color-primary);
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

  .file-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 12px 4px 28px;
    cursor: pointer;
    font-size: 12px;
    color: var(--text-100);
    border-radius: 8px;
  }

  .file-item:hover {
    background: var(--color-hover);
  }

  .file-item:hover .file-actions {
    opacity: 1;
  }

  .file-item--selected {
    background: color-mix(in srgb, var(--color-primary) 15%, transparent);
  }

  .file-item--selected:hover {
    background: color-mix(in srgb, var(--color-primary) 20%, transparent);
  }

  .file-item--selected .file-actions {
    opacity: 1;
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
    background: color-mix(in srgb, var(--color-success) 20%, transparent);
    color: var(--color-success);
  }

  .badge--modified {
    background: color-mix(in srgb, var(--color-warning) 25%, transparent);
    color: var(--color-warning);
  }

  .badge--type-changed {
    background: color-mix(in srgb, var(--color-warning) 25%, transparent);
    color: var(--color-warning);
  }

  .badge--deleted {
    background: color-mix(in srgb, var(--color-error) 25%, transparent);
    color: var(--color-error);
  }

  .badge--renamed {
    background: color-mix(in srgb, var(--color-info) 25%, transparent);
    color: var(--color-info);
  }

  .badge--copied {
    background: color-mix(in srgb, var(--color-primary) 25%, transparent);
    color: var(--color-primary);
  }

  .badge--untracked {
    background: color-mix(in srgb, var(--text-300) 20%, transparent);
    color: var(--text-300);
  }

  .badge--conflicted {
    background: color-mix(in srgb, var(--color-error) 35%, transparent);
    color: var(--color-error);
  }

  .badge--unknown {
    background: color-mix(in srgb, var(--text-300) 18%, transparent);
    color: var(--text-300);
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
