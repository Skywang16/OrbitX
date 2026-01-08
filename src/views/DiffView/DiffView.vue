<script setup lang="ts">
  import { computed, onMounted, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { confirm } from '@tauri-apps/plugin-dialog'
  import { useGitStore } from '@/stores/git'
  import type { DiffContent } from '@/api/git/types'
  import { gitApi } from '@/api'

  interface Props {
    repoPath: string
    filePath: string
    staged?: boolean
    commitHash?: string
  }

  const props = defineProps<Props>()
  const { t } = useI18n()
  const gitStore = useGitStore()

  const diff = ref<DiffContent | null>(null)
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  const isCommitDiff = computed(() => !!props.commitHash)

  const fileName = computed(() => {
    const parts = props.filePath.split('/')
    return parts[parts.length - 1]
  })

  const dirPath = computed(() => {
    const parts = props.filePath.split('/')
    if (parts.length <= 1) return ''
    return parts.slice(0, -1).join('/')
  })

  const loadDiff = async () => {
    if (!props.repoPath) {
      error.value = 'No repository path'
      return
    }

    isLoading.value = true
    error.value = null

    try {
      diff.value = await gitApi.getDiff({
        path: props.repoPath,
        filePath: props.filePath,
        staged: props.staged,
        commitHash: props.commitHash,
      })
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      diff.value = null
    } finally {
      isLoading.value = false
    }
  }

  onMounted(() => {
    loadDiff()
  })

  watch(
    () => [props.repoPath, props.filePath, props.staged, props.commitHash],
    () => {
      loadDiff()
    }
  )

  const getMarker = (lineType: string, content: string) => {
    if (lineType === 'added') return '+'
    if (lineType === 'removed') return '-'
    if (lineType === 'context') return ' '
    if (content.startsWith('\\')) return ' '
    return ''
  }

  const getDisplayText = (lineType: string, content: string) => {
    if (lineType === 'added' || lineType === 'removed' || lineType === 'context') {
      return content.length > 0 ? content.slice(1) : ''
    }
    return content
  }

  const stageFile = () => {
    gitStore.stageFile({ path: props.filePath, status: 'modified' }, props.repoPath)
  }

  const unstageFile = () => {
    gitStore.unstageFile({ path: props.filePath, status: 'modified' }, props.repoPath)
  }

  const discardFile = async () => {
    const message = t('git.discard_confirm', { file: fileName.value })
    const confirmed = await confirm(message, { title: t('git.discard'), kind: 'warning' })
    if (confirmed) {
      gitStore.discardFile({ path: props.filePath, status: 'modified' })
    }
  }
</script>

<template>
  <div class="diff-view">
    <div class="diff-view__header">
      <div class="diff-view__file-info">
        <span
          class="diff-view__badge"
          :class="{
            'diff-view__badge--commit': isCommitDiff,
            'diff-view__badge--staged': !isCommitDiff && props.staged,
            'diff-view__badge--unstaged': !isCommitDiff && !props.staged,
          }"
        >
          {{ isCommitDiff ? props.commitHash?.slice(0, 7) : props.staged ? t('git.staged') : t('git.changes') }}
        </span>
        <span class="diff-view__filename">{{ fileName }}</span>
        <span v-if="dirPath" class="diff-view__dirpath">{{ dirPath }}</span>
      </div>
      <div class="diff-view__actions">
        <template v-if="!isCommitDiff">
          <button v-if="props.staged" class="action-btn" :title="t('git.unstage')" @click="unstageFile">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
            <span>{{ t('git.unstage') }}</span>
          </button>
          <template v-else>
            <button class="action-btn" :title="t('git.discard')" @click="discardFile">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M3 7v6h6" />
                <path d="M21 17a9 9 0 0 0-9-9 9 9 0 0 0-6 2.3L3 13" />
              </svg>
              <span>{{ t('git.discard') }}</span>
            </button>
            <button class="action-btn action-btn--primary" :title="t('git.stage')" @click="stageFile">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="12" y1="5" x2="12" y2="19" />
                <line x1="5" y1="12" x2="19" y2="12" />
              </svg>
              <span>{{ t('git.stage') }}</span>
            </button>
          </template>
        </template>
        <button class="action-btn" :title="t('git.refresh')" @click="loadDiff">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 12a9 9 0 1 1-3-6.7" />
            <path d="M21 3v6h-6" />
          </svg>
        </button>
      </div>
    </div>

    <div v-if="isLoading" class="diff-view__loading">
      <span class="spinner" />
      <span>Loading diff...</span>
    </div>

    <div v-else-if="error" class="diff-view__error">
      {{ error }}
    </div>

    <div v-else-if="!diff || diff.hunks.length === 0" class="diff-view__empty">
      {{ t('git.no_diff') }}
    </div>

    <div v-else class="diff-view__content">
      <div v-for="(hunk, hunkIdx) in diff.hunks" :key="`hunk-${hunkIdx}`" class="hunk">
        <div class="hunk__header">{{ hunk.header }}</div>
        <div class="hunk__lines">
          <div
            v-for="(line, lineIdx) in hunk.lines"
            :key="`line-${hunkIdx}-${lineIdx}`"
            class="line"
            :class="`line--${line.lineType}`"
          >
            <span class="line__marker">{{ getMarker(line.lineType, line.content) }}</span>
            <span class="line__num line__num--old">{{ line.oldLineNumber ?? '' }}</span>
            <span class="line__num line__num--new">{{ line.newLineNumber ?? '' }}</span>
            <pre class="line__code">{{ getDisplayText(line.lineType, line.content) }}</pre>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .diff-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-50);
    overflow: hidden;
  }

  .diff-view__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-200);
    background: var(--bg-100);
    flex-shrink: 0;
  }

  .diff-view__file-info {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .diff-view__badge {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    padding: 2px 8px;
    border-radius: 4px;
    flex-shrink: 0;
  }

  .diff-view__badge--staged {
    background: rgba(34, 197, 94, 0.15);
    color: #22c55e;
  }

  .diff-view__badge--unstaged {
    background: rgba(234, 179, 8, 0.15);
    color: #eab308;
  }

  .diff-view__badge--commit {
    background: rgba(59, 130, 246, 0.15);
    color: #60a5fa;
    font-family: var(--font-mono);
  }

  .diff-view__filename {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-100);
    font-family: var(--font-mono);
  }

  .diff-view__dirpath {
    font-size: 12px;
    color: var(--text-300);
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .diff-view__actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 28px;
    padding: 0 10px;
    border: 1px solid var(--border-200);
    border-radius: 6px;
    background: var(--bg-50);
    color: var(--text-200);
    font-size: 12px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-btn:hover {
    background: var(--bg-200);
    color: var(--text-100);
  }

  .action-btn svg {
    width: 14px;
    height: 14px;
  }

  .action-btn--primary {
    background: var(--bg-200);
    border-color: var(--border-300);
  }

  .diff-view__loading,
  .diff-view__error,
  .diff-view__empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--text-300);
    font-size: 13px;
  }

  .diff-view__error {
    color: #ef4444;
  }

  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border-200);
    border-top-color: var(--text-200);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .diff-view__content {
    flex: 1;
    overflow: auto;
    padding: 16px;
  }

  .hunk {
    margin-bottom: 16px;
  }

  .hunk:last-child {
    margin-bottom: 0;
  }

  .hunk__header {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-300);
    padding: 8px 12px;
    background: var(--bg-200);
    border-radius: 6px 6px 0 0;
    border: 1px solid var(--border-200);
    border-bottom: none;
  }

  .hunk__lines {
    border: 1px solid var(--border-200);
    border-radius: 0 0 6px 6px;
    overflow: hidden;
    background: var(--bg-50);
  }

  .line {
    display: flex;
    align-items: stretch;
    font-family: var(--font-mono);
    font-size: 13px;
    line-height: 1.5;
  }

  .line__marker {
    width: 20px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-300);
    user-select: none;
  }

  .line__num {
    width: 40px;
    flex-shrink: 0;
    padding: 0 8px;
    text-align: right;
    color: var(--text-400);
    user-select: none;
    border-right: 1px solid var(--border-200);
  }

  .line__num--old {
    background: var(--bg-100);
  }

  .line__num--new {
    background: var(--bg-100);
  }

  .line__code {
    flex: 1;
    margin: 0;
    padding: 0 12px;
    white-space: pre;
    overflow-x: auto;
    color: var(--text-100);
  }

  .line--added {
    background: rgba(34, 197, 94, 0.1);
  }

  .line--added .line__marker {
    color: #22c55e;
    background: rgba(34, 197, 94, 0.15);
  }

  .line--added .line__num--new {
    background: rgba(34, 197, 94, 0.08);
  }

  .line--removed {
    background: rgba(239, 68, 68, 0.1);
  }

  .line--removed .line__marker {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.15);
  }

  .line--removed .line__num--old {
    background: rgba(239, 68, 68, 0.08);
  }

  .line--header {
    background: var(--bg-200);
    color: var(--text-300);
  }
</style>
