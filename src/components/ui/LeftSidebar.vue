<script setup lang="ts">
  import { ref, computed } from 'vue'
  import { useLayoutStore } from '@/stores/layout'
  import ResizeHandle from '@/components/AIChatSidebar/components/layout/ResizeHandle.vue'
  import WorkspacePanel from './WorkspacePanel.vue'
  import GitPanel from '@/components/GitPanel/index.vue'
  import ConfigPanel from './ConfigPanel.vue'

  const layoutStore = useLayoutStore()

  const isDragging = ref(false)
  const isHovering = ref(false)

  const panelStyle = computed(() => ({
    '--sidebar-width': `${layoutStore.leftSidebarWidth}px`,
  }))

  const startDrag = (event: MouseEvent) => {
    event.preventDefault()

    isDragging.value = true
    document.body.classList.add('orbitx-resizing')

    const startX = event.clientX
    const startWidth = layoutStore.leftSidebarWidth

    const handleMouseMove = (e: MouseEvent) => {
      e.preventDefault()
      const deltaX = e.clientX - startX
      layoutStore.setLeftSidebarWidth(startWidth + deltaX)
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

  const onMouseEnter = () => {
    isHovering.value = true
  }

  const onMouseLeave = () => {
    isHovering.value = false
  }

  const onDoubleClick = () => {
    layoutStore.setLeftSidebarWidth(280)
  }
</script>

<template>
  <div class="left-sidebar" :style="panelStyle">
    <div class="sidebar-content">
      <WorkspacePanel v-if="layoutStore.activeLeftPanel === 'workspace'" />
      <GitPanel v-else-if="layoutStore.activeLeftPanel === 'git'" />
      <ConfigPanel v-else-if="layoutStore.activeLeftPanel === 'config'" />
    </div>

    <ResizeHandle
      side="right"
      :is-dragging="isDragging"
      :is-hovering="isHovering"
      @mousedown="startDrag"
      @mouseenter="onMouseEnter"
      @mouseleave="onMouseLeave"
      @dblclick="onDoubleClick"
    />
  </div>
</template>

<style scoped>
  .left-sidebar {
    position: relative;
    width: var(--sidebar-width);
    min-width: 200px;
    max-width: 50vw;
    height: 100%;
    background: var(--bg-50);
    border-right: 1px solid var(--border-200);
    display: flex;
    flex-direction: column;
  }

  .sidebar-content {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .sidebar-content > :deep(*) {
    flex: 1;
    min-height: 0;
  }

  /* Override GitPanel styles when inside LeftSidebar */
  .sidebar-content :deep(.git-panel) {
    width: 100%;
    min-width: 0;
    max-width: none;
    border-right: none;
  }

  .sidebar-content :deep(.git-panel .resize-handle) {
    display: none;
  }
</style>
