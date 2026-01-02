<script setup lang="ts">
  import { computed, ref } from 'vue'
  import { useI18n } from 'vue-i18n'
  import dayjs from 'dayjs'
  import relativeTime from 'dayjs/plugin/relativeTime'
  import type { CommitFileChange, CommitInfo, CommitRef } from '@/api/git/types'
  import { gitApi } from '@/api/git'
  import { useGitStore } from '@/stores/git'

  dayjs.extend(relativeTime)

  const LANE_COLORS = ['#f97316', '#eab308', '#22c55e', '#06b6d4', '#3b82f6', '#8b5cf6', '#ec4899', '#ef4444']

  interface Props {
    commits: CommitInfo[]
    aheadCount?: number
  }

  interface Emits {
    (e: 'showDiff', hash: string, filePath: string): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()
  const { t } = useI18n()
  const gitStore = useGitStore()

  const expandedCommits = ref<Set<string>>(new Set())
  const commitFiles = ref<Map<string, CommitFileChange[]>>(new Map())
  const loadingCommits = ref<Set<string>>(new Set())

  interface GraphNode {
    commit: CommitInfo
    subject: string
    relativeDate: string
    lane: number
    lanes: LaneState[]
    index: number
  }

  interface LaneState {
    active: boolean
    color: string
    connects: 'none' | 'up' | 'down' | 'both' | 'merge-left' | 'merge-right' | 'fork-left' | 'fork-right'
  }

  const graphNodes = computed((): GraphNode[] => {
    const commits = props.commits
    if (commits.length === 0) return []

    const nodes: GraphNode[] = []
    const hashToIndex = new Map<string, number>()
    commits.forEach((c, i) => hashToIndex.set(c.shortHash, i))

    let lanes: (string | null)[] = []

    for (let i = 0; i < commits.length; i++) {
      const commit = commits[i]
      const shortHash = commit.shortHash

      let myLane = lanes.indexOf(shortHash)
      if (myLane === -1) {
        myLane = lanes.indexOf(null)
        if (myLane === -1) {
          myLane = lanes.length
          lanes.push(null)
        }
      }

      const laneStates: LaneState[] = []
      const maxLanes = Math.max(lanes.length, myLane + 1)

      for (let l = 0; l < maxLanes; l++) {
        const color = LANE_COLORS[l % LANE_COLORS.length]
        if (l === myLane) {
          laneStates.push({ active: true, color, connects: 'both' })
        } else if (lanes[l] !== null) {
          laneStates.push({ active: true, color, connects: 'both' })
        } else {
          laneStates.push({ active: false, color, connects: 'none' })
        }
      }

      lanes[myLane] = null

      const parents = commit.parents || []
      if (parents.length >= 1) {
        const firstParent = parents[0]
        if (hashToIndex.has(firstParent)) {
          lanes[myLane] = firstParent
        }
      }

      if (parents.length >= 2) {
        const secondParent = parents[1]
        if (hashToIndex.has(secondParent)) {
          let mergeLane = -1
          for (let l = 0; l < lanes.length; l++) {
            if (l !== myLane && lanes[l] === secondParent) {
              mergeLane = l
              break
            }
          }
          if (mergeLane === -1) {
            mergeLane = lanes.indexOf(null)
            if (mergeLane === -1 || mergeLane === myLane) {
              mergeLane = lanes.length
              lanes.push(null)
            }
            lanes[mergeLane] = secondParent
            while (laneStates.length <= mergeLane) {
              laneStates.push({
                active: false,
                color: LANE_COLORS[laneStates.length % LANE_COLORS.length],
                connects: 'none',
              })
            }
            laneStates[mergeLane] = {
              active: true,
              color: LANE_COLORS[mergeLane % LANE_COLORS.length],
              connects: mergeLane > myLane ? 'merge-right' : 'merge-left',
            }
          }
        }
      }

      while (lanes.length > 0 && lanes[lanes.length - 1] === null) {
        lanes.pop()
      }

      nodes.push({
        commit,
        subject: commit.message.split('\n')[0] || '',
        relativeDate: dayjs(commit.date).isValid() ? dayjs(commit.date).fromNow() : commit.date,
        lane: myLane,
        lanes: laneStates,
        index: i,
      })
    }

    return nodes
  })

  const isUnpushed = (index: number) => {
    const ahead = props.aheadCount ?? 0
    return index < ahead
  }

  const getRefClass = (ref: CommitRef) => {
    switch (ref.refType) {
      case 'head':
        return 'ref--head'
      case 'localBranch':
        return 'ref--local'
      case 'remoteBranch':
        return 'ref--remote'
      case 'tag':
        return 'ref--tag'
      default:
        return ''
    }
  }

  const toggleCommit = async (hash: string) => {
    if (expandedCommits.value.has(hash)) {
      expandedCommits.value.delete(hash)
      expandedCommits.value = new Set(expandedCommits.value)
    } else {
      expandedCommits.value.add(hash)
      expandedCommits.value = new Set(expandedCommits.value)

      if (!commitFiles.value.has(hash) && !loadingCommits.value.has(hash)) {
        loadingCommits.value.add(hash)
        loadingCommits.value = new Set(loadingCommits.value)

        const rootPath = gitStore.repositoryRoot || gitStore.currentPath
        if (rootPath) {
          try {
            const files = await gitApi.getCommitFiles(rootPath, hash)
            commitFiles.value.set(hash, files)
            commitFiles.value = new Map(commitFiles.value)
          } catch (e) {
            console.error('Failed to load commit files:', e)
            commitFiles.value.set(hash, [])
            commitFiles.value = new Map(commitFiles.value)
          }
        }

        loadingCommits.value.delete(hash)
        loadingCommits.value = new Set(loadingCommits.value)
      }
    }
  }

  const getStatusClass = (status: string) => {
    switch (status) {
      case 'added':
        return 'file--added'
      case 'modified':
        return 'file--modified'
      case 'deleted':
        return 'file--deleted'
      case 'renamed':
        return 'file--renamed'
      default:
        return ''
    }
  }

  const getStatusIcon = (status: string) => {
    switch (status) {
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
      default:
        return '?'
    }
  }

  const getFileName = (path: string) => {
    const parts = path.split('/')
    return parts[parts.length - 1]
  }

  const getFilePath = (path: string) => {
    const parts = path.split('/')
    if (parts.length > 1) {
      return parts.slice(0, -1).join('/') + '/'
    }
    return ''
  }
</script>

<template>
  <div class="history">
    <details class="section" open>
      <summary class="section__header">
        <span class="section__caret">▸</span>
        <span class="section__title">{{ t('git.commits') }}</span>
        <span v-if="graphNodes.length > 0" class="section__count">{{ graphNodes.length }}</span>
      </summary>

      <div v-if="graphNodes.length === 0" class="history__empty">
        {{ t('git.no_commits') }}
      </div>

      <div v-else class="history__list">
        <div
          v-for="node in graphNodes"
          :key="node.commit.hash"
          class="commit-wrapper"
          :class="{ unpushed: isUnpushed(node.index) }"
        >
          <div class="commit" @click="toggleCommit(node.commit.hash)">
            <div class="commit__graph" :style="{ width: `${Math.max(node.lanes.length * 14, 20)}px` }">
              <svg :width="Math.max(node.lanes.length * 14, 20)" height="36" class="graph-svg">
                <template v-for="(lane, laneIdx) in node.lanes" :key="laneIdx">
                  <line
                    v-if="lane.active && lane.connects !== 'none'"
                    :x1="laneIdx * 14 + 7"
                    y1="0"
                    :x2="laneIdx * 14 + 7"
                    y2="36"
                    :stroke="lane.color"
                    stroke-width="2"
                  />
                  <path
                    v-if="lane.connects === 'merge-right' && laneIdx > node.lane"
                    :d="`M${node.lane * 14 + 7},18 Q${(node.lane + laneIdx) * 7 + 7},18 ${laneIdx * 14 + 7},36`"
                    :stroke="lane.color"
                    stroke-width="2"
                    fill="none"
                  />
                  <path
                    v-if="lane.connects === 'merge-left' && laneIdx < node.lane"
                    :d="`M${node.lane * 14 + 7},18 Q${(node.lane + laneIdx) * 7 + 7},18 ${laneIdx * 14 + 7},36`"
                    :stroke="lane.color"
                    stroke-width="2"
                    fill="none"
                  />
                </template>
                <circle
                  :cx="node.lane * 14 + 7"
                  cy="18"
                  r="4"
                  :fill="node.lanes[node.lane]?.color || LANE_COLORS[0]"
                  stroke="var(--bg-50)"
                  stroke-width="2"
                />
              </svg>
            </div>
            <div class="commit__content">
              <div class="commit__top">
                <svg
                  class="commit__chevron"
                  :class="{ expanded: expandedCommits.has(node.commit.hash) }"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <polyline points="9 18 15 12 9 6"></polyline>
                </svg>
                <span class="commit__msg">{{ node.subject }}</span>
                <span v-for="ref in node.commit.refs" :key="ref.name" class="ref" :class="getRefClass(ref)">
                  {{ ref.name }}
                </span>
              </div>
              <div class="commit__meta">
                <span class="commit__hash">{{ node.commit.shortHash }}</span>
                <span class="commit__author">{{ node.commit.authorName }}</span>
                <span class="commit__date">{{ node.relativeDate }}</span>
              </div>
            </div>
          </div>

          <div v-if="expandedCommits.has(node.commit.hash)" class="commit__files">
            <div v-if="loadingCommits.has(node.commit.hash)" class="commit__files-loading">
              {{ t('git.loading') }}...
            </div>
            <template v-else-if="commitFiles.get(node.commit.hash)?.length">
              <div
                v-for="file in commitFiles.get(node.commit.hash)"
                :key="file.path"
                class="file-item"
                @click.stop="emit('showDiff', node.commit.hash, file.path)"
              >
                <span class="file-status" :class="getStatusClass(file.status)">
                  {{ getStatusIcon(file.status) }}
                </span>
                <span class="file-path">{{ getFilePath(file.path) }}</span>
                <span class="file-name">{{ getFileName(file.path) }}</span>
                <span v-if="file.additions > 0 || file.deletions > 0" class="file-stats">
                  <span v-if="file.additions > 0" class="additions">+{{ file.additions }}</span>
                  <span v-if="file.deletions > 0" class="deletions">-{{ file.deletions }}</span>
                </span>
              </div>
            </template>
            <div v-else class="commit__files-empty">
              {{ t('git.no_files') }}
            </div>
          </div>
        </div>
      </div>
    </details>
  </div>
</template>

<style scoped>
  .history {
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
    flex-shrink: 0;
  }

  .section__header::-webkit-details-marker {
    display: none;
  }

  .section__header:hover {
    background: var(--bg-100);
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

  .history__empty {
    padding: 16px;
    text-align: center;
    font-size: 12px;
    color: var(--text-300);
  }

  .history__list {
    overflow: visible;
  }

  .commit-wrapper {
    border-bottom: 1px solid var(--border-100);
  }

  .commit-wrapper:last-child {
    border-bottom: none;
  }

  .commit-wrapper.unpushed {
    background: rgba(34, 197, 94, 0.05);
    border-left: 2px solid #22c55e;
  }

  .commit-wrapper.unpushed .commit__hash {
    color: #22c55e;
  }

  .commit {
    display: flex;
    cursor: pointer;
    transition: background 0.1s ease;
    padding-right: 12px;
  }

  .commit:hover {
    background: var(--bg-100);
  }

  .commit__graph {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: flex-start;
  }

  .graph-svg {
    display: block;
  }

  .commit__content {
    flex: 1;
    min-width: 0;
    padding: 6px 0;
  }

  .commit__top {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .commit__chevron {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
    color: var(--text-400);
    transition: transform 0.15s ease;
  }

  .commit__chevron.expanded {
    transform: rotate(90deg);
  }

  .commit__msg {
    font-size: 12px;
    color: var(--text-100);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .ref {
    display: inline-flex;
    align-items: center;
    font-size: 10px;
    font-weight: 500;
    padding: 1px 6px;
    border-radius: 4px;
    white-space: nowrap;
    flex-shrink: 0;
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ref--head {
    background: rgba(239, 68, 68, 0.15);
    color: #ef4444;
  }

  .ref--local {
    background: rgba(34, 197, 94, 0.15);
    color: #22c55e;
  }

  .ref--remote {
    background: rgba(59, 130, 246, 0.15);
    color: #60a5fa;
  }

  .ref--tag {
    background: rgba(168, 85, 247, 0.15);
    color: #c084fc;
  }

  .commit__meta {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 2px;
    margin-left: 18px;
    font-size: 11px;
    color: var(--text-300);
  }

  .commit__hash {
    font-family: var(--font-mono);
    color: var(--text-200);
  }

  .commit__author {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  .commit__date {
    flex-shrink: 0;
  }

  .commit__files {
    padding: 4px 12px 8px 32px;
    background: var(--bg-50);
  }

  .commit__files-loading,
  .commit__files-empty {
    font-size: 11px;
    color: var(--text-400);
    padding: 4px 0;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 6px;
    border-radius: 4px;
    cursor: pointer;
    transition: background 0.1s ease;
  }

  .file-item:hover {
    background: var(--bg-200);
  }

  .file-status {
    font-size: 10px;
    font-weight: 600;
    width: 14px;
    text-align: center;
    flex-shrink: 0;
  }

  .file--added {
    color: #22c55e;
  }

  .file--modified {
    color: #eab308;
  }

  .file--deleted {
    color: #ef4444;
  }

  .file--renamed {
    color: #8b5cf6;
  }

  .file-path {
    font-size: 11px;
    color: var(--text-400);
    font-family: var(--font-mono);
  }

  .file-name {
    font-size: 11px;
    color: var(--text-200);
    font-family: var(--font-mono);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-stats {
    display: flex;
    gap: 4px;
    font-size: 10px;
    font-family: var(--font-mono);
    flex-shrink: 0;
  }

  .additions {
    color: #22c55e;
  }

  .deletions {
    color: #ef4444;
  }

  .icon-btn {
    width: 24px;
    height: 24px;
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

  .icon-btn:hover {
    background: var(--bg-200);
    color: var(--text-100);
  }

  .icon-btn svg {
    width: 14px;
    height: 14px;
  }
</style>
