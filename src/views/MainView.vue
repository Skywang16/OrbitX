<script setup lang="ts">
  import { windowApi } from '@/api'
  import AppSidebar from '@/components/AppSidebar/index.vue'
  import MainChatArea from '@/components/MainChatArea/index.vue'
  import TerminalPanel from '@/components/TerminalPanel/index.vue'
  import { useLayoutStore } from '@/stores/layout'
  import { useWindowStore } from '@/stores/Window'
  import { useWorkspaceStore } from '@/stores/workspace'
  import SettingsView from '@/views/Settings/SettingsView.vue'
  import SkillsView from '@/views/Skills/SkillsView.vue'
  import type { UnlistenFn } from '@tauri-apps/api/event'
  import { storeToRefs } from 'pinia'
  import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
  import { useI18n } from 'vue-i18n'

  const { t } = useI18n()
  const workspaceStore = useWorkspaceStore()
  const layoutStore = useLayoutStore()
  const windowStore = useWindowStore()
  const { showSettings, terminalPanelVisible: isTerminalVisible } = storeToRefs(layoutStore)

  // Window pin state
  const isAlwaysOnTop = computed(() => windowStore.alwaysOnTop)
  const toggleAlwaysOnTop = async () => {
    const newState = await windowApi.toggleAlwaysOnTop()
    windowStore.setAlwaysOnTop(newState)
  }

  // ============ Layout State ============
  const isDraggingDivider = ref(false)
  const showSkills = ref(false)

  // Sidebar Ref for parent control if needed
  const sidebarRef = ref<InstanceType<typeof AppSidebar> | null>(null)

  // Toggle Sidebar Action
  const toggleSidebar = () => {
    layoutStore.setSidebarVisible(!layoutStore.sidebarVisible)
  }

  // Handle Sidebar Events
  const handleUpdateSidebarWidth = (width: number) => {
    layoutStore.setSidebarWidth(width)
  }

  const handleUpdateShowSkills = (show: boolean) => {
    showSkills.value = show
  }

  const handleSidebarDragStart = () => {
    document.body.classList.add('orbitx-resizing', 'orbitx-resize-col')
  }

  const handleSidebarDragEnd = () => {
    document.body.classList.remove('orbitx-resizing', 'orbitx-resize-col')
  }

  // ============ Settings State ============
  const activeSettingsSection = computed(() => sidebarRef.value?.activeSettingsSection ?? 'general')

  // ============ Terminal Actions ============
  const handleFilePath = async (filePath: string) => {
    const directory = await windowApi.handleFileOpen(filePath)
    if (directory) {
      await workspaceStore.loadTree()
      await workspaceStore.loadThreadViews(directory)
    }
  }

  const startDividerDrag = (event: MouseEvent) => {
    event.preventDefault()
    isDraggingDivider.value = true
    document.body.classList.add('orbitx-resizing', 'orbitx-resize-row')

    const startY = event.clientY
    const startHeight = layoutStore.terminalPanelHeight

    const handleMouseMove = (e: MouseEvent) => {
      e.preventDefault()
      const deltaY = startY - e.clientY
      const newHeight = Math.max(100, Math.min(startHeight + deltaY, 600))
      layoutStore.setTerminalPanelHeight(newHeight)
    }

    const handleMouseUp = () => {
      isDraggingDivider.value = false
      document.body.classList.remove('orbitx-resizing', 'orbitx-resize-row')
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
    }

    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)
  }

  // ============ Lifecycle ============
  let unlistenStartupFile: UnlistenFn | null = null
  let unlistenFileDropped: UnlistenFn | null = null

  onMounted(async () => {
    unlistenStartupFile = await windowApi.onStartupFile(handleFilePath)
    unlistenFileDropped = await windowApi.onFileDropped(handleFilePath)
    await workspaceStore.loadTree()
  })

  onBeforeUnmount(() => {
    unlistenStartupFile?.()
    unlistenFileDropped?.()
  })
</script>

<template>
  <div class="app-container">
    <!-- Toggle Buttons (only show when not in settings) -->
    <div v-if="!showSettings" class="top-buttons">
      <button
        class="top-btn"
        :class="{ 'top-btn--collapsed': !layoutStore.sidebarVisible }"
        :title="t('sidebar.toggle')"
        @click="toggleSidebar"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <rect x="3" y="4" width="18" height="16" rx="3" ry="3" />
          <line class="sidebar-line" x1="9" y1="7" x2="9" y2="17" stroke-linecap="round" />
        </svg>
      </button>
      <button
        class="top-btn"
        :class="{ 'top-btn--pinned': isAlwaysOnTop }"
        :title="t('shortcuts.toggle_window_pin')"
        @click="toggleAlwaysOnTop"
      >
        <svg class="pin-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <circle cx="12" cy="8" r="4" />
          <path d="M12 12v9" stroke-linecap="round" />
        </svg>
      </button>
    </div>

    <div class="main-layout">
      <!-- ========== SIDEBAR COMPONENT ========== -->
      <AppSidebar
        ref="sidebarRef"
        :is-visible="layoutStore.sidebarVisible"
        :width="layoutStore.sidebarWidth"
        :show-skills="showSkills"
        @update:width="handleUpdateSidebarWidth"
        @update:show-skills="handleUpdateShowSkills"
        @toggle-sidebar="toggleSidebar"
        @drag-start="handleSidebarDragStart"
        @drag-end="handleSidebarDragEnd"
      />

      <!-- ========== MAIN CONTENT ========== -->
      <main class="content">
        <div class="content-area">
          <!-- Settings Content -->
          <template v-if="showSettings">
            <div class="content-drag-region" data-tauri-drag-region />
            <SettingsView :active-section="activeSettingsSection" />
          </template>

          <!-- Skills Content -->
          <template v-else-if="showSkills">
            <SkillsView />
          </template>

          <!-- Main App Content -->
          <template v-else>
            <div class="chat-area">
              <MainChatArea :sidebar-collapsed="!layoutStore.sidebarVisible" />
            </div>

            <div
              v-if="isTerminalVisible"
              class="terminal-divider"
              :class="{ active: isDraggingDivider }"
              @mousedown="startDividerDrag"
            />

            <div
              v-if="isTerminalVisible"
              class="terminal-area"
              :style="{ height: `${layoutStore.terminalPanelHeight}px` }"
            >
              <TerminalPanel />
            </div>
          </template>
        </div>
      </main>
    </div>
  </div>
</template>

<style scoped>
  .app-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: transparent;
    position: relative;
  }

  .top-buttons {
    position: fixed;
    top: 6px;
    left: 78px;
    display: flex;
    gap: 4px;
    z-index: 200;
    -webkit-app-region: no-drag;
  }

  .top-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-md);
    color: var(--text-300);
    cursor: pointer;
    transition:
      color 0.15s ease,
      background 0.15s ease;
  }

  .top-btn:hover {
    color: var(--text-100);
    background: var(--color-hover);
  }

  .top-btn svg {
    width: 16px;
    height: 16px;
  }

  /* Sidebar toggle animation */
  .top-btn .sidebar-line {
    transition: transform 0.25s ease;
    transform-origin: center;
    transform: translateX(3px);
  }

  .top-btn--collapsed .sidebar-line {
    transform: translateX(0);
  }

  /* Pin button animation */
  .top-btn .pin-icon {
    transition: transform 0.25s ease;
  }

  .top-btn--pinned {
    color: var(--color-primary);
  }

  .top-btn--pinned .pin-icon {
    transform: rotate(45deg);
  }

  .top-btn--pinned:hover {
    color: var(--color-primary);
  }

  .main-layout {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
    background: var(--sidebar-glass-bg);
  }

  /* ========== MAIN CONTENT ========== */
  .content {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--sidebar-glass-bg);
  }

  .content-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
    background: var(--bg-100-solid);
    border-radius: var(--border-radius-2xl);
    box-shadow: var(--shadow-md);
    position: relative;
  }

  .content-drag-region {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 40px;
    -webkit-app-region: drag;
    z-index: 100;
  }

  /* Chat area */
  .chat-area {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .terminal-divider {
    flex-shrink: 0;
    height: 12px;
    background: linear-gradient(to bottom, transparent 50%, var(--bg-200) 50%);
    cursor: row-resize;
    position: relative;
    z-index: 10;
  }

  .terminal-divider::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    transform: translateY(-50%);
    height: 1px;
    background: var(--border-200);
    transition: background 0.15s ease;
    pointer-events: none;
  }

  .terminal-divider:hover::after,
  .terminal-divider.active::after {
    background: var(--color-primary);
  }

  .terminal-area {
    flex-shrink: 0;
    min-height: 100px;
    max-height: 600px;
    overflow: hidden;
  }
</style>
