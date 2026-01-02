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

  const onMouseEnter = () => {
    isHovering.value = true
  }

  const onMouseLeave = () => {
    isHovering.value = false
  }

  const onDoubleClick = () => {
    gitStore.setPanelWidth(320)
  }

  const refreshAll = async () => {
    await gitStore.refreshStatus()
    if (gitStore.isRepository) {
      gitStore.loadBranches()
      gitStore.loadCommits()
    }
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
        :is-loading="gitStore.isLoading"
        :ahead="gitStore.status?.ahead"
        :behind="gitStore.status?.behind"
        :staged-count="gitStore.stagedCount"
        @refresh="refreshAll"
        @commit="gitStore.commit"
        @push="gitStore.push"
        @pull="gitStore.pull"
        @sync="gitStore.sync"
        @fetch="gitStore.fetch"
      />

      <div v-if="gitStore.error" class="git-panel__error">
        {{ gitStore.error }}
      </div>

      <template v-if="gitStore.isRepository">
        <div class="git-panel__sections">
          <FileChanges
            :staged="gitStore.status?.stagedFiles ?? []"
            :modified="gitStore.status?.modifiedFiles ?? []"
            :untracked="gitStore.status?.untrackedFiles ?? []"
            :conflicted="gitStore.status?.conflictedFiles ?? []"
            :selected-file="gitStore.selectedFile"
            :selected-is-staged="gitStore.selectedFileIsStaged"
            @select="gitStore.openDiffTab"
            @stage="gitStore.stageFile"
            @unstage="gitStore.unstageFile"
            @discard="gitStore.discardFile"
            @stage-all="gitStore.stageAllFiles"
            @unstage-all="gitStore.unstageAllFiles"
            @discard-all="gitStore.discardAllChanges"
          />

          <CommitHistory
            :commits="gitStore.commits"
            :ahead-count="gitStore.status?.ahead ?? 0"
            @show-diff="gitStore.showCommitFileDiff"
          />
        </div>
      </template>

      <div v-else class="git-panel__empty">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <circle cx="12" cy="12" r="10" />
          <path d="M12 16v-4" />
          <path d="M12 8h.01" />
        </svg>
        <span>{{ t('git.no_repository') }}</span>
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

  .git-panel__sections {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
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
    gap: 12px;
    padding: 24px;
    color: var(--text-300);
  }

  .git-panel__empty svg {
    width: 40px;
    height: 40px;
    opacity: 0.5;
  }

  .git-panel__empty span {
    font-size: 13px;
    text-align: center;
  }
</style>
