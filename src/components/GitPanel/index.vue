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
  const changesHeight = ref(240)
  const isDraggingDivider = ref(false)

  const panelStyle = computed(() => ({
    '--panel-width': `${gitStore.panelWidth}px`,
  }))

  const startDrag = (event: MouseEvent) => {
    event.preventDefault()

    isDragging.value = true
    document.body.classList.add('orbitx-resizing')

    const startX = event.clientX
    const startWidth = gitStore.panelWidth

    const handleMouseMove = (e: MouseEvent) => {
      e.preventDefault()
      const deltaX = e.clientX - startX
      gitStore.setPanelWidth(startWidth + deltaX)
    }

    const handleMouseUp = () => {
      isDragging.value = false
      document.body.classList.remove('orbitx-resizing')
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
    }

    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)
  }

  const startDividerDrag = (event: MouseEvent) => {
    event.preventDefault()

    isDraggingDivider.value = true
    document.body.classList.add('orbitx-resizing')

    const startY = event.clientY
    const startHeight = changesHeight.value

    const handleMouseMove = (e: MouseEvent) => {
      e.preventDefault()
      const deltaY = e.clientY - startY
      const newHeight = Math.max(120, Math.min(startHeight + deltaY, 500))
      changesHeight.value = newHeight
    }

    const handleMouseUp = () => {
      isDraggingDivider.value = false
      document.body.classList.remove('orbitx-resizing')
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
    gitStore.setPanelWidth(360)
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
      <!-- Git Header -->
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

      <!-- Error -->
      <div v-if="gitStore.error" class="git-error">
        <svg class="git-error__icon" viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z" />
        </svg>
        <span>{{ gitStore.error }}</span>
      </div>

      <!-- Repository Content -->
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
          :class="{ 'git-panel__divider--active': isDraggingDivider }"
          @mousedown.stop.prevent="startDividerDrag"
        >
          <div class="divider-handle" />
        </div>

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

      <!-- Empty State -->
      <div v-else class="git-empty">
        <div class="git-empty__visual">
          <svg class="git-empty__icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M6 3v12M6 7h7a3 3 0 0 1 3 3v11" />
            <circle cx="6" cy="5" r="2" />
            <circle cx="6" cy="15" r="2" />
            <circle cx="16" cy="21" r="2" />
          </svg>
        </div>
        <div class="git-empty__text">
          <h3>{{ t('git.no_repository') }}</h3>
          <p>Initialize a Git repository to start tracking changes</p>
        </div>
        <button class="git-empty__btn" @click="gitStore.initRepository">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="12" y1="5" x2="12" y2="19" />
            <line x1="5" y1="12" x2="19" y2="12" />
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
    background: var(--bg-200);
    border-right: 1px solid var(--border-200);
    display: flex;
    flex-direction: column;
    min-width: 300px;
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

  /* Changes */
  .git-panel__changes {
    flex-shrink: 0;
    min-height: 120px;
    max-height: 500px;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .git-panel__changes::-webkit-scrollbar {
    width: 6px;
  }

  .git-panel__changes::-webkit-scrollbar-track {
    background: transparent;
  }

  .git-panel__changes::-webkit-scrollbar-thumb {
    background: var(--border-300);
    border-radius: 3px;
  }

  /* Divider */
  .git-panel__divider {
    flex-shrink: 0;
    height: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: ns-resize;
    background: transparent;
    transition: background 0.15s ease;
  }

  .git-panel__divider:hover,
  .git-panel__divider--active {
    background: var(--bg-300);
  }

  .divider-handle {
    width: 32px;
    height: 3px;
    background: var(--border-300);
    border-radius: 2px;
    transition: all 0.15s ease;
  }

  .git-panel__divider:hover .divider-handle,
  .git-panel__divider--active .divider-handle {
    width: 48px;
    background: var(--color-primary);
  }

  /* History */
  .git-panel__history {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  /* Error */
  .git-error {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
    margin: 8px;
    font-size: 12px;
    color: var(--color-error);
    background: color-mix(in srgb, var(--color-error) 8%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-error) 20%, transparent);
    border-radius: 10px;
  }

  .git-error__icon {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
  }

  /* Empty */
  .git-empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 20px;
    padding: 40px 24px;
    text-align: center;
  }

  .git-empty__visual {
    width: 80px;
    height: 80px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-100);
    border-radius: 20px;
  }

  .git-empty__icon {
    width: 40px;
    height: 40px;
    color: var(--text-500);
  }

  .git-empty__text {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .git-empty__text h3 {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-200);
    margin: 0;
  }

  .git-empty__text p {
    font-size: 13px;
    color: var(--text-500);
    margin: 0;
    max-width: 200px;
  }

  .git-empty__btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 20px;
    font-size: 13px;
    font-weight: 500;
    color: white;
    background: var(--color-primary);
    border: none;
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .git-empty__btn:hover {
    background: var(--color-primary-hover);
    transform: translateY(-1px);
  }

  .git-empty__btn svg {
    width: 16px;
    height: 16px;
  }
</style>
