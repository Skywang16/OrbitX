<script setup lang="ts">
  import { computed, nextTick, onMounted, ref, watch } from 'vue'
  import type { ComponentPublicInstance } from 'vue'
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
  const commitRowRefs = new Map<string, HTMLElement>()
  const commitRowCenterY = ref<Map<string, number>>(new Map())

  interface GraphNode {
    commit: CommitInfo
    subject: string
    relativeDate: string
    x: number
    index: number
    color: string
  }

  interface GraphEdge {
    key: string
    fromHash: string
    fromX: number
    fromIndex: number
    toHash: string
    toX: number
    toIndex: number
    color: string
  }

  const graphData = computed(() => {
    const commits = props.commits
    if (commits.length === 0) return { nodes: [] as GraphNode[], edges: [] as GraphEdge[], maxX: 0 }

    const hashToIndex = new Map<string, number>()
    commits.forEach((c, i) => {
      hashToIndex.set(c.hash, i)
    })

    const nodes: GraphNode[] = []
    const edges: GraphEdge[] = []

    // columns[x] = hash of commit this column is reserved for (null = free)
    const columns: (string | null)[] = []
    const columnColors: string[] = []
    const getLaneColor = (lane: number) => LANE_COLORS[lane % LANE_COLORS.length]

    for (let i = 0; i < commits.length; i++) {
      const commit = commits[i]
      const hash = commit.hash
      const parents = commit.parents || []

      // Find which column this commit should occupy
      let myX = columns.indexOf(hash)

      if (myX === -1) {
        // No column reserved for us, find a free one
        myX = columns.indexOf(null)
        if (myX === -1) {
          myX = columns.length
          columns.push(null)
          columnColors.push(getLaneColor(myX))
        }
      }

      const myColor = columnColors[myX] || getLaneColor(myX)
      if (!columnColors[myX]) columnColors[myX] = myColor

      // Clear this column (we've arrived)
      columns[myX] = null

      nodes.push({
        commit,
        subject: commit.message.split('\n')[0] || '',
        relativeDate: dayjs(commit.date).isValid() ? dayjs(commit.date).fromNow() : commit.date,
        x: myX,
        index: i,
        color: myColor,
      })

      // Process parents
      for (let p = 0; p < parents.length; p++) {
        const parentHash = parents[p]
        const parentIndex = hashToIndex.get(parentHash)
        if (parentIndex === undefined) {
          continue
        }

        // Check if parent already has a reserved column
        let parentX = columns.indexOf(parentHash)

        if (parentX === -1) {
          // Parent doesn't have a column yet
          if (p === 0) {
            // First parent: stays in same column (main line continues)
            parentX = myX
            columns[myX] = parentHash
          } else {
            // Additional parent (merge): needs a new column
            parentX = columns.indexOf(null)
            if (parentX === -1) {
              parentX = columns.length
              columns.push(null)
              columnColors.push(getLaneColor(parentX))
            }
            columns[parentX] = parentHash
          }
        }

        edges.push({
          key: `${hash}->${parentHash}`,
          fromHash: hash,
          fromX: myX,
          fromIndex: i,
          toHash: parentHash,
          toX: parentX,
          toIndex: parentIndex,
          color: columnColors[parentX],
        })
      }
    }

    const maxX = Math.max(
      nodes.reduce((max, n) => Math.max(max, n.x), 0),
      edges.reduce((max, e) => Math.max(max, e.fromX, e.toX), 0)
    )
    return { nodes, edges, maxX }
  })

  const graphNodes = computed(() => graphData.value.nodes)

  const graphWidth = computed(() => {
    if (graphNodes.value.length === 0) return 24
    return Math.max((graphData.value.maxX + 1) * 16 + 8, 24)
  })

  const LANE_WIDTH = 16
  const ROW_HEIGHT = 28

  const svgHeight = computed(() => {
    let maxY = graphNodes.value.length * ROW_HEIGHT
    for (const node of graphNodes.value) {
      const y = getNodeY(node.commit.hash, node.index)
      maxY = Math.max(maxY, y + ROW_HEIGHT / 2)
    }
    return maxY
  })

  // Get Y position for a node - always use fixed calculation
  const getNodeY = (hash: string, index: number): number => {
    return commitRowCenterY.value.get(hash) ?? index * ROW_HEIGHT + ROW_HEIGHT / 2
  }

  // Generate SVG path for an edge
  const getEdgePath = (edge: GraphEdge) => {
    const x1 = edge.fromX * LANE_WIDTH + LANE_WIDTH / 2
    const y1 = getNodeY(edge.fromHash, edge.fromIndex)
    const x2 = edge.toX * LANE_WIDTH + LANE_WIDTH / 2
    const y2 = getNodeY(edge.toHash, edge.toIndex)

    if (edge.fromX === edge.toX) {
      return `M ${x1} ${y1} L ${x2} ${y2}`
    } else {
      const controlY = y1 + Math.abs(y2 - y1) * 0.3
      return `M ${x1} ${y1} C ${x1} ${controlY}, ${x2} ${controlY}, ${x2} ${y2}`
    }
  }

  const setCommitRowRef = (hash: string, el: HTMLElement | null) => {
    if (!el) {
      commitRowRefs.delete(hash)
      return
    }

    commitRowRefs.set(hash, el)
  }

  const updateCommitRowCenters = () => {
    const next = new Map<string, number>()
    for (const [hash, rowEl] of commitRowRefs) {
      const infoEl = rowEl.querySelector<HTMLElement>('.commit-info')
      const infoHeight = infoEl?.offsetHeight ?? ROW_HEIGHT
      next.set(hash, rowEl.offsetTop + infoHeight / 2)
    }
    commitRowCenterY.value = next
  }

  onMounted(() => {
    void nextTick(() => {
      updateCommitRowCenters()
    })
  })

  watch(
    () => props.commits.length,
    async () => {
      await nextTick()
      updateCommitRowCenters()
    }
  )

  watch(
    () => expandedCommits.value,
    async () => {
      await nextTick()
      updateCommitRowCenters()
    }
  )

  watch(commitFiles, async () => {
    await nextTick()
    updateCommitRowCenters()
  })

  const hasMoreVisible = computed(() => props.hasMore ?? true)
  const canLoadMore = computed(() => hasMoreVisible.value && !isLoadingMore.value)

  const shouldAutoLoad = () => {
    const el = listRef.value
    if (!el) return false
    const threshold = 100
    return el.scrollHeight - el.scrollTop - el.clientHeight < threshold
  }

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
          return
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
      case 'typeChanged':
        return 'file--type-changed'
      case 'deleted':
        return 'file--deleted'
      case 'renamed':
        return 'file--renamed'
      case 'unknown':
        return 'file--unknown'
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
      case 'typeChanged':
        return 'T'
      case 'deleted':
        return 'D'
      case 'renamed':
        return 'R'
      case 'copied':
        return 'C'
      case 'unknown':
        return '?'
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

  const loadMore = async () => {
    if (!canLoadMore.value || !shouldAutoLoad()) return

    // Avoid runaway: at most N pages per scroll burst.
    const maxPages = 3
    for (let i = 0; i < maxPages; i++) {
      if (!canLoadMore.value || !shouldAutoLoad()) break
      isLoadingMore.value = true
      await emit('loadMore')
      isLoadingMore.value = false
      await nextTick()
      updateCommitRowCenters()
    }
  }

  const handleScroll = () => {
    void loadMore()
  }
</script>

<template>
  <div class="history">
    <div class="section__header">
      <span class="section__title">{{ t('git.commits') }}</span>
      <span v-if="graphNodes.length > 0" class="section__count">{{ graphNodes.length }}</span>
    </div>

    <div v-if="graphNodes.length === 0" class="history__empty">
      {{ t('git.no_commits') }}
    </div>

    <div v-else ref="listRef" class="history__list" @scroll="handleScroll">
      <!-- SVG Graph Layer -->
      <svg class="graph-svg" :width="graphWidth" :height="svgHeight">
        <!-- Draw all edges first -->
        <path
          v-for="edge in graphData.edges"
          :key="edge.key"
          :d="getEdgePath(edge)"
          :stroke="edge.color"
          stroke-width="2"
          fill="none"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <!-- Draw all nodes on top -->
        <g v-for="node in graphNodes" :key="`node-${node.commit.hash}`">
          <circle
            :cx="node.x * LANE_WIDTH + LANE_WIDTH / 2"
            :cy="getNodeY(node.commit.hash, node.index)"
            r="5"
            fill="var(--bg-50)"
          />
          <circle
            :cx="node.x * LANE_WIDTH + LANE_WIDTH / 2"
            :cy="getNodeY(node.commit.hash, node.index)"
            r="4"
            :fill="node.color"
          />
        </g>
      </svg>

      <!-- Commit rows -->
      <div
        v-for="node in graphNodes"
        :key="node.commit.hash"
        :ref="
          (el: Element | ComponentPublicInstance | null) => setCommitRowRef(node.commit.hash, el as HTMLElement | null)
        "
        class="commit-row"
        :class="{ unpushed: isUnpushed(node.index) }"
        :style="{ paddingLeft: `${graphWidth}px` }"
      >
        <div class="commit-info" @click="toggleCommit(node.commit.hash)">
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
          <span class="commit__author">{{ node.commit.authorName }}</span>
          <span class="commit__date">{{ node.relativeDate }}</span>
        </div>

        <!-- Expanded files -->
        <div v-if="expandedCommits.has(node.commit.hash)" class="commit__files">
          <div v-if="loadingCommits.has(node.commit.hash)" class="commit__files-loading">{{ t('git.loading') }}...</div>
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
              <span v-if="file.isBinary" class="file-stats">
                <span class="file-binary">binary</span>
              </span>
              <span v-else-if="(file.additions ?? 0) > 0 || (file.deletions ?? 0) > 0" class="file-stats">
                <span v-if="(file.additions ?? 0) > 0" class="additions">+{{ file.additions }}</span>
                <span v-if="(file.deletions ?? 0) > 0" class="deletions">-{{ file.deletions }}</span>
              </span>
            </div>
          </template>
          <div v-else class="commit__files-empty">
            {{ t('git.no_files') }}
          </div>
        </div>
      </div>

      <div v-if="isLoadingMore" class="history__loading">{{ t('git.loading') }}...</div>
    </div>
  </div>
</template>

<style scoped>
  .history {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .section__header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-200);
    flex-shrink: 0;
    border-bottom: 1px solid var(--border-100);
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

  .history__empty {
    padding: 16px;
    text-align: center;
    font-size: 12px;
    color: var(--text-300);
  }

  .history__list {
    flex: 1;
    overflow-y: auto;
    overflow-x: auto;
    position: relative;
    user-select: none;
  }

  .graph-svg {
    position: absolute;
    top: 0;
    left: 0;
    pointer-events: none;
  }

  .history__loading {
    padding: 12px;
    text-align: center;
    font-size: 12px;
    color: var(--text-300);
  }

  .commit-row {
    min-height: 28px;
    display: flex;
    flex-direction: column;
  }

  .commit-row.unpushed {
    background: color-mix(in srgb, var(--color-success) 8%, transparent);
  }

  .commit-info {
    min-height: 28px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 8px;
    cursor: pointer;
    border-radius: 4px;
    font-size: 12px;
  }

  .commit-info:hover {
    background: var(--color-hover);
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
    font-size: 9px;
    font-weight: 500;
    padding: 1px 5px;
    border-radius: 3px;
    white-space: nowrap;
    flex-shrink: 0;
    max-width: 100px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ref--head {
    background: color-mix(in srgb, var(--color-error) 25%, transparent);
    color: var(--color-error);
  }

  .ref--local {
    background: color-mix(in srgb, var(--color-success) 25%, transparent);
    color: var(--color-success);
  }

  .ref--remote {
    background: color-mix(in srgb, var(--color-info) 25%, transparent);
    color: var(--color-info);
  }

  .ref--tag {
    background: color-mix(in srgb, var(--color-primary) 25%, transparent);
    color: var(--color-primary);
  }

  .commit__author {
    font-size: 11px;
    color: var(--text-300);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 80px;
    flex-shrink: 0;
  }

  .commit__date {
    font-size: 11px;
    color: var(--text-400);
    flex-shrink: 0;
    white-space: nowrap;
  }

  .commit__files {
    width: 100%;
    margin-top: 2px;
    margin-bottom: 4px;
    padding: 4px 8px;
    border-radius: 4px;
    background: color-mix(in srgb, var(--bg-100) 30%, transparent);
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
  }

  .file-item:hover {
    background: var(--color-hover);
  }

  .file-status {
    font-size: 10px;
    font-weight: 600;
    width: 14px;
    text-align: center;
    flex-shrink: 0;
  }

  .file--added {
    color: var(--color-success);
  }

  .file--modified {
    color: var(--color-warning);
  }

  .file--type-changed {
    color: var(--color-warning);
  }

  .file--deleted {
    color: var(--color-error);
  }

  .file--renamed {
    color: var(--color-info);
  }

  .file--unknown {
    color: var(--text-400);
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
    color: var(--color-success);
  }

  .deletions {
    color: var(--color-error);
  }

  .file-binary {
    color: var(--text-300);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.4px;
  }
</style>
