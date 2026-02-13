<script setup lang="ts">
  import { computed, onMounted, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useGitStore } from '@/stores/git'

  import ResizeHandle from '@/components/AIChatSidebar/components/layout/ResizeHandle.vue'
  import GitHeader from './components/GitHeader.vue'
  import FileChanges from './components/FileChanges.vue'
  import CommitHistory from './components/CommitHistory.vue'

  const gitStore = useGitStore()
  const { t } = useI18n()

  const isDragging = ref(false)
  const isHovering = ref(false)
  const changesHeight = ref(200)
  const isDraggingDivider = ref(false)

  const panelStyle = computed(() => ({
    '--panel-width': `${gitStore.panelWidth}px`,
  }))

  const startDrag = (event: MouseEvent) => {
    isDragging.value = true
    const startX = event.clientX
    const startWidth = gitStore.panelWidth

    const handleMouseMove = (e: MouseEvent) => {
      const deltaX = e.clientX - startX
      gitStore.setPanelWidth(startWidth + deltaX)
    }

    const handleMouseUp = () => {
      isDragging.value = false
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
    }

    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)
  }

  const startDividerDrag = (event: MouseEvent) => {
    isDraggingDivider.value = true
    const startY = event.clientY
    const startHeight = changesHeight.value

    const handleMouseMove = (e: MouseEvent) => {
      const deltaY = e.clientY - startY
      const newHeight = Math.max(100, Math.min(startHeight + deltaY, 500))
      changesHeight.value = newHeight
    }

    const handleMouseUp = () => {
      isDraggingDivider.value = false
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
    }

    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)
  }

  const onMouseEnter = () => {
    isHovering.value = true
  }

  const onMouseLeave = () => {
    isHovering.value = false
  }

  const onDoubleClick = () => {
    gitStore.setPanelWidth(320)
  }

  const loadMoreCommits = async () => {
    return await gitStore.loadMoreCommits(50)
  }

  onMounted(() => {
    gitStore.refreshStatus()
  })

  watch(
    () => gitStore.isRepository,
    isRepo => {
      if (!isRepo) return
      gitStore.loadBranches()
      gitStore.loadCommits()
    },
    { immediate: true }
  )
</script>

<template>
  <div class="git-panel" :style="panelStyle">
    <ResizeHandle
      side="right"
      :is-dragging="isDragging"
      :is-hovering="isHovering"
      @mousedown="startDrag"
      @mouseenter="onMouseEnter"
      @mouseleave="onMouseLeave"
      @dblclick="onDoubleClick"
    />

    <div class="git-panel__content">
      <GitHeader
        :branch="gitStore.currentBranch"
        :ahead="gitStore.status?.ahead"
        :behind="gitStore.status?.behind"
        :staged-count="gitStore.status?.stagedFiles.length ?? 0"
        :branches="gitStore.branches"
        @commit="gitStore.commit"
        @push="gitStore.push"
        @pull="gitStore.pull"
        @sync="gitStore.sync"
        @fetch="gitStore.fetch"
        @checkout="gitStore.checkoutBranch"
      />

      <div v-if="gitStore.error" class="git-panel__error">
        {{ gitStore.error }}
      </div>

      <template v-if="gitStore.isRepository">
        <div class="git-panel__changes" :style="{ height: `${changesHeight}px` }">
          <FileChanges
            :staged="gitStore.status?.stagedFiles ?? []"
            :modified="gitStore.status?.modifiedFiles ?? []"
            :untracked="gitStore.status?.untrackedFiles ?? []"
            :conflicted="gitStore.status?.conflictedFiles ?? []"
            @select="gitStore.openDiffTab"
            @stage="gitStore.stageFiles"
            @unstage="gitStore.unstageFiles"
            @discard="gitStore.discardFiles"
            @stage-all="gitStore.stageAllFiles"
            @unstage-all="gitStore.unstageAllFiles"
            @discard-all="gitStore.discardAllChanges"
          />
        </div>

        <div
          class="git-panel__divider"
          :class="{ 'git-panel__divider--dragging': isDraggingDivider }"
          @mousedown="startDividerDrag"
        />

        <div class="git-panel__history">
          <CommitHistory
            :commits="gitStore.commits"
            :has-more="gitStore.commitsHasMore"
            :ahead-count="gitStore.status?.ahead ?? 0"
            @show-diff="gitStore.showCommitFileDiff"
            @load-more="loadMoreCommits"
          />
        </div>
      </template>

      <div v-else class="git-panel__empty">
        <svg class="git-panel__empty-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M6 3v12" />
          <path d="M6 7h7a3 3 0 0 1 3 3v11" />
          <circle cx="6" cy="5" r="2" />
          <circle cx="6" cy="15" r="2" />
          <circle cx="16" cy="21" r="2" />
        </svg>
        <span class="git-panel__empty-text">{{ t('git.no_repository') }}</span>
        <button class="git-panel__init-btn" @click="gitStore.initRepository">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 5v14M5 12h14" />
          </svg>
          {{ t('git.init_repository') }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .git-panel {
    width: var(--panel-width);
    height: 100%;
    background: var(--bg-50);
    border-right: 1px solid var(--border-200);
    display: flex;
    flex-direction: column;
    min-width: 260px;
    max-width: 50vw;
    position: relative;
  }

  .git-panel__content {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }

  .git-panel__changes {
    flex-shrink: 0;
    min-height: 100px;
    max-height: 500px;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .git-panel__divider {
    flex-shrink: 0;
    height: 4px;
    background: var(--border-200);
    cursor: ns-resize;
    transition: background 0.15s ease;
    position: relative;
  }

  .git-panel__divider::before {
    content: '';
    position: absolute;
    top: -4px;
    left: 0;
    right: 0;
    bottom: -4px;
  }

  .git-panel__divider:hover,
  .git-panel__divider--dragging {
    background: var(--color-primary);
  }

  .git-panel__history {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .git-panel__error {
    padding: 8px 12px;
    font-size: 12px;
    color: #ef4444;
    background: rgba(239, 68, 68, 0.1);
    border-bottom: 1px solid rgba(239, 68, 68, 0.2);
  }

  .git-panel__empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
    padding: 24px;
    color: var(--text-300);
  }

  .git-panel__empty-icon {
    width: 48px;
    height: 48px;
    opacity: 0.4;
  }

  .git-panel__empty-text {
    font-size: 13px;
    text-align: center;
    color: var(--text-400);
  }

  .git-panel__init-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-100);
    background: var(--color-primary);
    border: none;
    border-radius: var(--border-radius-md);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .git-panel__init-btn:hover {
    background: var(--color-primary-hover);
  }

  .git-panel__init-btn svg {
    width: 14px;
    height: 14px;
  }
</style>
