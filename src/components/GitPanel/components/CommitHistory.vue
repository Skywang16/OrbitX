<script setup lang="ts">
  import { computed, ref } from 'vue'
  import { useI18n } from 'vue-i18n'
  import dayjs from 'dayjs'
  import relativeTime from 'dayjs/plugin/relativeTime'
  import type { CommitFileChange, CommitInfo, CommitRef } from '@/api/git/types'
  import { gitApi } from '@/api/git'
  import { useGitStore } from '@/stores/git'

  dayjs.extend(relativeTime)

  interface Props {
    commits: CommitInfo[]
    aheadCount?: number
    hasMore?: boolean
  }

  interface Emits {
    (e: 'showDiff', hash: string, filePath: string): void
    (e: 'loadMore'): Promise<{ loaded: number; hasMore: boolean }>
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()
  const { t } = useI18n()
  const gitStore = useGitStore()

  const expandedCommits = ref<Set<string>>(new Set())
  const commitFiles = ref<Map<string, CommitFileChange[]>>(new Map())
  const loadingCommits = ref<Set<string>>(new Set())
  const isLoadingMore = ref(false)
  const listRef = ref<HTMLElement | null>(null)

  const hasMoreVisible = computed(() => props.hasMore ?? true)
  const canLoadMore = computed(() => hasMoreVisible.value && !isLoadingMore.value)

  const shouldAutoLoad = () => {
    const el = listRef.value
    if (!el) return false
    return el.scrollHeight - el.scrollTop - el.clientHeight < 100
  }

  const isUnpushed = (index: number) => index < (props.aheadCount ?? 0)

  const getSubject = (message: string) => message.split('\n')[0] || ''
  const getRelativeDate = (date: string) => (dayjs(date).isValid() ? dayjs(date).fromNow() : date)
  const getShortHash = (hash: string) => hash.slice(0, 7)

  const getRefType = (ref: CommitRef) => {
    const map: Record<string, string> = { head: 'head', localBranch: 'branch', remoteBranch: 'remote', tag: 'tag' }
    return map[ref.refType] || 'branch'
  }

  const toggleCommit = async (hash: string) => {
    if (expandedCommits.value.has(hash)) {
      expandedCommits.value.delete(hash)
      expandedCommits.value = new Set(expandedCommits.value)
      return
    }

    expandedCommits.value.add(hash)
    expandedCommits.value = new Set(expandedCommits.value)

    if (!commitFiles.value.has(hash) && !loadingCommits.value.has(hash)) {
      loadingCommits.value.add(hash)
      loadingCommits.value = new Set(loadingCommits.value)

      const rootPath = gitStore.repositoryRoot || gitStore.currentPath
      if (rootPath) {
        await gitApi
          .getCommitFiles(rootPath, hash)
          .then(files => {
            commitFiles.value.set(hash, files)
            commitFiles.value = new Map(commitFiles.value)
          })
          .finally(() => {
            loadingCommits.value.delete(hash)
            loadingCommits.value = new Set(loadingCommits.value)
          })
      } else {
        loadingCommits.value.delete(hash)
        loadingCommits.value = new Set(loadingCommits.value)
      }
    }
  }

  const getStatusBadge = (status: string) => {
    const map: Record<string, { label: string; type: string }> = {
      added: { label: 'A', type: 'added' },
      modified: { label: 'M', type: 'modified' },
      deleted: { label: 'D', type: 'deleted' },
      renamed: { label: 'R', type: 'renamed' },
      copied: { label: 'C', type: 'copied' },
    }
    return map[status] || { label: '?', type: 'unknown' }
  }

  const getFileName = (path: string) => path.split('/').pop() || path
  const getFilePath = (path: string) => {
    const parts = path.split('/')
    return parts.length > 1 ? parts.slice(0, -1).join('/') + '/' : ''
  }

  const loadMore = async () => {
    if (!canLoadMore.value || !shouldAutoLoad()) return
    for (let i = 0; i < 3; i++) {
      if (!canLoadMore.value || !shouldAutoLoad()) break
      isLoadingMore.value = true
      await emit('loadMore')
      isLoadingMore.value = false
    }
  }

  const handleScroll = () => void loadMore()
</script>

<template>
  <div class="commit-history">
    <div class="history-header">
      <span class="history-header__title">{{ t('git.commits') }}</span>
      <span v-if="commits.length > 0" class="history-header__count">{{ commits.length }}</span>
    </div>

    <div v-if="commits.length === 0" class="history-empty">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
        <circle cx="12" cy="12" r="3" />
        <path d="M12 3v6m0 6v6" />
      </svg>
      <span>{{ t('git.no_commits') }}</span>
    </div>

    <div v-else ref="listRef" class="history-list" @scroll="handleScroll">
      <div
        v-for="(commit, index) in commits"
        :key="commit.hash"
        class="commit-card"
        :class="{
          'commit-card--unpushed': isUnpushed(index),
          'commit-card--expanded': expandedCommits.has(commit.hash),
        }"
      >
        <div class="commit-card__main" @click="toggleCommit(commit.hash)">
          <div class="commit-card__indicator">
            <div class="commit-dot" :class="{ 'commit-dot--unpushed': isUnpushed(index) }" />
          </div>

          <div class="commit-card__content">
            <div class="commit-card__message">{{ getSubject(commit.message) }}</div>
            <div v-if="commit.refs && commit.refs.length > 0" class="commit-card__refs">
              <span
                v-for="ref in commit.refs"
                :key="ref.name"
                class="commit-ref"
                :class="`commit-ref--${getRefType(ref)}`"
              >
                {{ ref.name }}
              </span>
            </div>
            <div class="commit-card__meta">
              <span class="commit-card__hash">{{ getShortHash(commit.hash) }}</span>
              <span class="commit-card__sep">•</span>
              <span class="commit-card__author">{{ commit.authorName }}</span>
              <span class="commit-card__sep">•</span>
              <span class="commit-card__time">{{ getRelativeDate(commit.date) }}</span>
            </div>
          </div>

          <div class="commit-card__expand">
            <svg
              class="expand-icon"
              :class="{ 'expand-icon--expanded': expandedCommits.has(commit.hash) }"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <polyline points="6 9 12 15 18 9" />
            </svg>
          </div>
        </div>

        <div v-if="expandedCommits.has(commit.hash)" class="commit-card__files">
          <div v-if="loadingCommits.has(commit.hash)" class="files-loading">
            <div class="loading-spinner" />
            <span>{{ t('git.loading') }}</span>
          </div>
          <template v-else-if="commitFiles.get(commit.hash)?.length">
            <div
              v-for="file in commitFiles.get(commit.hash)"
              :key="file.path"
              class="file-row"
              @click.stop="emit('showDiff', commit.hash, file.path)"
            >
              <span class="file-badge" :class="`file-badge--${getStatusBadge(file.status).type}`">
                {{ getStatusBadge(file.status).label }}
              </span>
              <span class="file-row__path">{{ getFilePath(file.path) }}</span>
              <span class="file-row__name">{{ getFileName(file.path) }}</span>
              <span v-if="file.isBinary" class="file-row__binary">binary</span>
              <span v-else-if="(file.additions ?? 0) > 0 || (file.deletions ?? 0) > 0" class="file-row__stats">
                <span v-if="(file.additions ?? 0) > 0" class="stat-add">+{{ file.additions }}</span>
                <span v-if="(file.deletions ?? 0) > 0" class="stat-del">-{{ file.deletions }}</span>
              </span>
            </div>
          </template>
          <div v-else class="files-empty">{{ t('git.no_files') }}</div>
        </div>
      </div>

      <div v-if="isLoadingMore" class="history-loading">
        <div class="loading-spinner" />
      </div>
    </div>
  </div>
</template>

<style scoped>
  .commit-history {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .history-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border-100);
  }

  .history-header__title {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-300);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .history-header__count {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-400);
    background: var(--bg-200);
    padding: 2px 8px;
    border-radius: 10px;
  }

  .history-empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 40px 20px;
    color: var(--text-500);
    font-size: 13px;
  }

  .history-empty svg {
    width: 40px;
    height: 40px;
    opacity: 0.4;
  }

  .history-list {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 6px;
  }

  .history-list::-webkit-scrollbar {
    width: 6px;
  }

  .history-list::-webkit-scrollbar-track {
    background: transparent;
  }

  .history-list::-webkit-scrollbar-thumb {
    background: var(--border-300);
    border-radius: 3px;
  }

  .commit-card {
    margin-bottom: 2px;
    border-radius: 8px;
    overflow: hidden;
  }

  .commit-card:hover {
    background: var(--bg-300);
  }

  .commit-card--expanded {
    background: var(--bg-300);
    margin-bottom: 8px;
  }

  .commit-card--unpushed {
    background: color-mix(in srgb, var(--color-success) 8%, transparent);
  }

  .commit-card--unpushed:hover {
    background: color-mix(in srgb, var(--color-success) 12%, transparent);
  }

  .commit-card__main {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px;
    cursor: pointer;
  }

  .commit-card__indicator {
    flex-shrink: 0;
    padding-top: 4px;
  }

  .commit-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-500);
  }

  .commit-dot--unpushed {
    background: var(--color-success);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-success) 20%, transparent);
  }

  .commit-card__content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .commit-card__message {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-100);
    line-height: 1.4;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .commit-card__refs {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .commit-ref {
    font-size: 10px;
    font-weight: 500;
    padding: 2px 6px;
    border-radius: 4px;
    white-space: nowrap;
  }

  .commit-ref--head {
    background: color-mix(in srgb, var(--color-error) 15%, transparent);
    color: var(--color-error);
  }

  .commit-ref--branch {
    background: color-mix(in srgb, var(--color-success) 15%, transparent);
    color: var(--color-success);
  }

  .commit-ref--remote {
    background: color-mix(in srgb, var(--color-info) 15%, transparent);
    color: var(--color-info);
  }

  .commit-ref--tag {
    background: color-mix(in srgb, var(--color-warning) 15%, transparent);
    color: var(--color-warning);
  }

  .commit-card__meta {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text-500);
  }

  .commit-card__hash {
    font-family: var(--font-family-mono);
    color: var(--text-400);
  }

  .commit-card__sep {
    opacity: 0.5;
  }

  .commit-card__author {
    max-width: 100px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .commit-card__expand {
    flex-shrink: 0;
    padding-top: 2px;
  }

  .expand-icon {
    width: 16px;
    height: 16px;
    color: var(--text-500);
    transition: transform 0.2s ease;
  }

  .expand-icon--expanded {
    transform: rotate(180deg);
  }

  .commit-card__files {
    padding: 0 12px 12px 32px;
  }

  .files-loading,
  .files-empty {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px;
    font-size: 12px;
    color: var(--text-500);
  }

  .file-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 6px;
    cursor: pointer;
  }

  .file-row:hover {
    background: var(--bg-400);
  }

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

  .file-badge--renamed,
  .file-badge--copied {
    background: color-mix(in srgb, var(--color-info) 15%, transparent);
    color: var(--color-info);
  }

  .file-badge--unknown {
    background: var(--bg-300);
    color: var(--text-400);
  }

  .file-row__path {
    font-size: 11px;
    color: var(--text-500);
    font-family: var(--font-family-mono);
  }

  .file-row__name {
    flex: 1;
    font-size: 12px;
    color: var(--text-200);
    font-family: var(--font-family-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-row__binary {
    font-size: 10px;
    color: var(--text-500);
    text-transform: uppercase;
  }

  .file-row__stats {
    display: flex;
    gap: 6px;
    font-size: 11px;
    font-family: var(--font-family-mono);
  }

  .stat-add {
    color: var(--color-success);
  }

  .stat-del {
    color: var(--color-error);
  }

  .history-loading {
    display: flex;
    justify-content: center;
    padding: 16px;
  }

  .loading-spinner {
    width: 16px;
    height: 16px;
    border: 2px solid var(--border-200);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
