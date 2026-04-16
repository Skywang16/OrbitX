<script setup lang="ts">
  import { workspaceApi, type ThreadRecord, type WorkspaceRecord } from '@/api/workspace'
  import { useAIChatStore } from '@/components/AIChatSidebar'
  import { useLayoutStore } from '@/stores/layout'
  import { useTerminalStore } from '@/stores/Terminal'
  import { useWorkspaceStore } from '@/stores/workspace'
  import { formatRelativeTime } from '@/utils/dateFormatter'
  import { showPopoverAt } from '@/ui'
  import { onBeforeUnmount } from 'vue'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { open } from '@tauri-apps/plugin-dialog'
  import { storeToRefs } from 'pinia'
  import { computed, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'

  const { t } = useI18n()
  const layoutStore = useLayoutStore()
  const workspaceStore = useWorkspaceStore()
  const aiChatStore = useAIChatStore()
  const terminalStore = useTerminalStore()

  const { showSettings } = storeToRefs(layoutStore)
  const { selectedThread } = storeToRefs(workspaceStore)

  const props = defineProps<{
    isVisible: boolean
    width: number
    showSkills: boolean
  }>()

  const emit = defineEmits<{
    (e: 'update:width', width: number): void
    (e: 'update:showSkills', show: boolean): void
    (e: 'toggle-sidebar'): void
    (e: 'drag-start'): void
    (e: 'drag-end'): void
  }>()

  // ============ Sidebar Resize ============
  const isDragging = ref(false)

  const startResize = (event: MouseEvent) => {
    event.preventDefault()
    isDragging.value = true
    emit('drag-start')

    const startX = event.clientX
    const startWidth = props.width

    const handleMouseMove = (e: MouseEvent) => {
      e.preventDefault()
      const deltaX = e.clientX - startX
      const newWidth = Math.max(180, Math.min(startWidth + deltaX, 400))
      emit('update:width', newWidth)
    }

    const handleMouseUp = () => {
      isDragging.value = false
      emit('drag-end')
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
    }

    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)
  }

  // ============ Window Actions ============
  const startWindowDrag = async () => {
    await getCurrentWindow().startDragging()
  }

  const handleDoubleClick = async () => {
    const win = getCurrentWindow()
    const isMaximized = await win.isMaximized()
    if (isMaximized) {
      await win.unmaximize()
    } else {
      await win.maximize()
    }
  }

  // ============ Settings Navigation ============
  const activeSettingsSection = ref('general')
  const settingsNavItems = computed(() => [
    { id: 'general', label: t('settings.general.title') },
    { id: 'ai', label: t('settings.ai.title') },
    { id: 'mcp', label: t('mcp_settings.title') },
    { id: 'theme', label: t('settings.theme.title') },
    { id: 'shortcuts', label: t('settings.shortcuts.title') },
    { id: 'language', label: t('settings.language.title') },
  ])

  const handleSettingsNavChange = (section: string) => {
    activeSettingsSection.value = section
  }

  const handleBackFromSettings = () => {
    layoutStore.closeSettings()
  }

  // ============ Skills Actions ============
  const handleOpenSkills = () => {
    emit('update:showSkills', true)
  }

  const expandedPaths = ref<Set<string>>(new Set())
  const currentThreadId = computed(() => selectedThread.value?.id ?? null)

  // Auto-expand workspace folder when a session is selected (e.g. on restore)
  watch(
    selectedThread,
    thread => {
      if (thread && !expandedPaths.value.has(thread.workspacePath)) {
        expandedPaths.value.add(thread.workspacePath)
        expandedPaths.value = new Set(expandedPaths.value)
      }
    },
    { immediate: true }
  )

  const getWorkspaceName = (workspace: WorkspaceRecord) => {
    if (workspace.displayName) return workspace.displayName
    return workspace.path.split('/').pop() || workspace.path
  }

  const isExpanded = (path: string) => expandedPaths.value.has(path)
  const getNode = (path: string) => workspaceStore.getNode(path)
  const getThreads = (path: string): ThreadRecord[] => workspaceStore.getTopLevelThreads(path)

  const handleToggleWorkspace = async (path: string) => {
    workspaceStore.setActiveWorkspace(path)
    if (expandedPaths.value.has(path)) {
      expandedPaths.value.delete(path)
    } else {
      expandedPaths.value.add(path)
      const node = workspaceStore.getNode(path)
      if (node && node.threadViews.length === 0 && !node.isLoading) {
        await workspaceStore.loadThreadViews(path)
      }
    }
    expandedPaths.value = new Set(expandedPaths.value)
  }

  const isWorkspaceActive = (path: string) => workspaceStore.activeWorkspacePath === path

  const handleSelectThread = async (thread: ThreadRecord) => {
    emit('update:showSkills', false)
    await workspaceStore.selectThread(thread)
  }

  const handleNewThread = async () => {
    emit('update:showSkills', false)
    await aiChatStore.startNewChat()
  }

  const handleOpenFolder = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Select Workspace Folder',
    })
    if (selected && typeof selected === 'string') {
      // First, ensure the workspace is created/registered in the backend
      await workspaceApi.getOrCreate(selected)
      // Now reload the tree to include the new workspace
      await workspaceStore.loadTree()
      // Set as active workspace
      workspaceStore.setActiveWorkspace(selected)
      // Expand and load sessions
      expandedPaths.value.add(selected)
      expandedPaths.value = new Set(expandedPaths.value)
      await workspaceStore.loadThreadViews(selected)
    }
  }

  const handleOpenSettings = () => {
    layoutStore.openSettings()
  }

  const getThreadTitle = (thread: {
    title?: string | null
    id: number
    threadType: string
    workspacePath?: string
  }) => {
    if (thread.threadType === 'shell') {
      const terminal = terminalStore.terminals.find(t => t.threadId === thread.id && t.kind === 'workspace')
      if (terminal?.displayTitle) return terminal.displayTitle
    }

    if (thread.title) return thread.title

    return thread.threadType === 'shell' ? t('sidebar.shell_session') : t('sidebar.agent_session')
  }

  const isThreadActive = (thread: ThreadRecord) => thread.id === currentThreadId.value
  const isThreadLoading = (thread: ThreadRecord) => aiChatStore.isThreadRunning(thread.id)

  const createNewSession = async (workspacePath: string, threadType: string) => {
    emit('update:showSkills', false)
    workspaceStore.setActiveWorkspace(workspacePath)
    // Expand the workspace folder
    if (!expandedPaths.value.has(workspacePath)) {
      expandedPaths.value.add(workspacePath)
      expandedPaths.value = new Set(expandedPaths.value)
    }
    // Set default title based on thread type
    const defaultTitle = threadType === 'shell' ? t('sidebar.shell_session') : t('sidebar.agent_session')
    await workspaceStore.createThread(workspacePath, defaultTitle, threadType)
  }

  const handleShowSessionMenu = async (event: MouseEvent, workspacePath: string) => {
    event.stopPropagation()
    const rect = (event.target as HTMLElement).getBoundingClientRect()
    await showPopoverAt(rect.left, rect.bottom + 4, [
      {
        label: t('sidebar.new_agent_session'),
        onClick: () => createNewSession(workspacePath, 'agent'),
      },
      {
        label: t('sidebar.new_shell_session'),
        onClick: () => createNewSession(workspacePath, 'shell'),
      },
    ])
  }

  const confirmingDeleteId = ref<number | null>(null)
  const confirmingDeleteWorkspace = ref<string | null>(null)
  let confirmingTimer: ReturnType<typeof setTimeout> | null = null

  const clearConfirmingTimer = () => {
    if (confirmingTimer) {
      clearTimeout(confirmingTimer)
      confirmingTimer = null
    }
  }

  onBeforeUnmount(clearConfirmingTimer)

  const resetConfirming = () => {
    clearConfirmingTimer()
    confirmingDeleteId.value = null
    confirmingDeleteWorkspace.value = null
  }

  const handleDeleteWorkspace = async (event: MouseEvent, workspacePath: string) => {
    event.stopPropagation()
    if (confirmingDeleteWorkspace.value === workspacePath) {
      clearConfirmingTimer()
      confirmingDeleteWorkspace.value = null
      await workspaceStore.deleteWorkspace(workspacePath)
    } else {
      clearConfirmingTimer()
      confirmingDeleteWorkspace.value = workspacePath
      confirmingTimer = setTimeout(() => {
        confirmingDeleteWorkspace.value = null
      }, 2000)
    }
  }

  const handleDeleteThread = async (event: MouseEvent, thread: ThreadRecord) => {
    event.stopPropagation()
    if (confirmingDeleteId.value === thread.id) {
      clearConfirmingTimer()
      confirmingDeleteId.value = null
      await workspaceStore.deleteThread(thread.id, thread.workspacePath)
    } else {
      clearConfirmingTimer()
      confirmingDeleteId.value = thread.id
      confirmingTimer = setTimeout(() => {
        confirmingDeleteId.value = null
      }, 2000)
    }
  }

  defineExpose({ activeSettingsSection })
</script>

<template>
  <aside
    class="sidebar"
    :class="{ 'sidebar--collapsed': !isVisible && !showSettings, 'sidebar--dragging': isDragging }"
    :style="{ width: isVisible || showSettings ? `${width}px` : '0' }"
  >
    <div class="sidebar-inner" :style="{ width: `${width}px` }">
      <!-- Settings Sidebar -->
      <template v-if="showSettings">
        <div class="sidebar-header" @mousedown="startWindowDrag" @dblclick="handleDoubleClick" />
        <button class="back-btn" @click="handleBackFromSettings">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M15 19l-7-7 7-7" />
          </svg>
          {{ t('settings.back_to_app') }}
        </button>
        <nav class="settings-nav">
          <button
            v-for="item in settingsNavItems"
            :key="item.id"
            class="nav-item"
            :class="{ active: activeSettingsSection === item.id }"
            @click="handleSettingsNavChange(item.id)"
          >
            {{ item.label }}
          </button>
        </nav>
      </template>

      <!-- Threads Sidebar -->
      <template v-else>
        <div class="sidebar-header" @mousedown="startWindowDrag" @dblclick="handleDoubleClick" />
        <button class="new-thread-btn" @click="handleNewThread">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M12 3c7.2 0 9 1.8 9 9s-1.8 9-9 9-9-1.8-9-9 1.8-9 9-9" />
            <path d="M12 8v8" />
            <path d="M8 12h8" />
          </svg>
          <span>{{ t('sidebar.new_thread') }}</span>
        </button>

        <button class="new-thread-btn skills-btn" :class="{ active: showSkills }" @click="handleOpenSkills">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H20v20H6.5a2.5 2.5 0 0 1 0-5H20" />
            <path d="M8 7h8" />
            <path d="M8 11h8" />
            <path d="M15 15h1" />
          </svg>
          <span>{{ t('sidebar.skills') }}</span>
        </button>

        <div class="threads-section">
          <div class="section-header">
            <span class="section-title">{{ t('sidebar.threads') }}</span>
            <button class="icon-btn" :title="t('sidebar.open_folder')" @click="handleOpenFolder">
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <path d="M12 5v14m7-7H5" />
              </svg>
            </button>
          </div>

          <div class="threads-list">
            <div v-for="workspace in workspaceStore.workspaces" :key="workspace.path" class="workspace-group">
              <div
                class="workspace-item"
                :class="{ expanded: isExpanded(workspace.path), active: isWorkspaceActive(workspace.path) }"
                @click="handleToggleWorkspace(workspace.path)"
                @mouseleave="resetConfirming"
              >
                <span class="workspace-icon-slot" :class="{ expanded: isExpanded(workspace.path) }">
                  <!-- Arrow (shown on hover or when expanded) -->
                  <svg
                    class="expand-icon"
                    :class="{ expanded: isExpanded(workspace.path) }"
                    viewBox="0 0 16 16"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.5"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <path d="M6 4l4 4-4 4" />
                  </svg>
                  <!-- Folder open -->
                  <svg
                    v-if="isExpanded(workspace.path)"
                    class="folder-icon"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.5"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <path
                      d="M6 14l1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.55 6a2 2 0 0 1-1.94 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2"
                    />
                  </svg>
                  <!-- Folder closed -->
                  <svg
                    v-else
                    class="folder-icon"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.5"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <path
                      d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2h16z"
                    />
                  </svg>
                </span>
                <span class="workspace-name">{{ getWorkspaceName(workspace) }}</span>
                <span v-if="getNode(workspace.path)?.isLoading" class="loading-indicator">...</span>
                <span class="workspace-actions">
                  <!-- New session button with native menu -->
                  <button
                    class="action-btn add-btn"
                    :title="t('chat.new_session')"
                    @click="handleShowSessionMenu($event, workspace.path)"
                  >
                    <svg
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="1.5"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    >
                      <path d="M12 5v14" />
                      <path d="M5 12h14" />
                    </svg>
                  </button>
                  <button
                    class="action-btn delete-btn"
                    :class="{ confirming: confirmingDeleteWorkspace === workspace.path }"
                    :title="
                      confirmingDeleteWorkspace === workspace.path ? t('common.confirm') : t('sidebar.delete_workspace')
                    "
                    @click="handleDeleteWorkspace($event, workspace.path)"
                  >
                    <svg
                      v-if="confirmingDeleteWorkspace === workspace.path"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    >
                      <path d="M20 6L9 17l-5-5" />
                    </svg>
                    <svg
                      v-else
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="1.5"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    >
                      <path d="M3 6h18" />
                      <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
                      <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
                    </svg>
                  </button>
                </span>
              </div>

              <Transition name="tree-expand">
                <div v-if="isExpanded(workspace.path)" class="sessions-list">
                  <div v-if="!getThreads(workspace.path).length" class="no-sessions">
                    {{ t('sidebar.no_threads') }}
                  </div>
                  <template v-for="thread in getThreads(workspace.path)" :key="thread.id">
                    <div
                      class="session-item"
                      :class="{ active: isThreadActive(thread), loading: isThreadLoading(thread) }"
                      @click.stop="handleSelectThread(thread)"
                      @mouseleave="resetConfirming"
                    >
                      <span v-if="isThreadLoading(thread)" class="session-loading">
                        <svg viewBox="0 0 16 16" fill="none">
                          <path
                            d="M8 2a6 6 0 0 1 0 12"
                            stroke="currentColor"
                            stroke-width="1.5"
                            stroke-linecap="round"
                          />
                        </svg>
                      </span>
                      <span class="session-title">{{ getThreadTitle(thread) }}</span>
                      <!-- Thread type icon -->
                      <span class="session-type-icon" :class="thread.threadType">
                        <!-- Agent: sparkle/AI icon -->
                        <svg
                          v-if="thread.threadType === 'agent'"
                          viewBox="0 0 16 16"
                          fill="none"
                          stroke="currentColor"
                          stroke-width="1.5"
                          stroke-linecap="round"
                          stroke-linejoin="round"
                        >
                          <path d="M8 1l1.5 3.5L13 6l-3.5 1.5L8 11l-1.5-3.5L3 6l3.5-1.5z" />
                          <path d="M12 10l.5 1.5L14 12l-1.5.5L12 14l-.5-1.5L10 12l1.5-.5z" />
                        </svg>
                        <!-- Shell: terminal symbol -->
                        <svg
                          v-else
                          viewBox="0 0 16 16"
                          fill="none"
                          stroke="currentColor"
                          stroke-width="1.5"
                          stroke-linecap="round"
                          stroke-linejoin="round"
                        >
                          <path d="M3 5l3 3-3 3" />
                          <path d="M7 11h6" />
                        </svg>
                      </span>
                      <span class="session-trailing">
                        <span class="session-time">{{ formatRelativeTime(thread.updatedAt * 1000) }}</span>
                        <button
                          class="delete-btn"
                          :class="{ confirming: confirmingDeleteId === thread.id }"
                          :title="confirmingDeleteId === thread.id ? t('common.confirm') : t('sidebar.delete_session')"
                          @click="handleDeleteThread($event, thread)"
                        >
                          <svg
                            v-if="confirmingDeleteId === thread.id"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                          >
                            <path d="M20 6L9 17l-5-5" />
                          </svg>
                          <svg
                            v-else
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.5"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                          >
                            <path d="M3 6h18" />
                            <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
                            <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
                          </svg>
                        </button>
                      </span>
                    </div>
                  </template>
                </div>
              </Transition>
            </div>

            <div v-if="workspaceStore.workspaces.length === 0" class="empty-state">
              <span>{{ t('sidebar.no_workspaces') }}</span>
            </div>
          </div>
        </div>

        <div class="sidebar-footer">
          <button class="settings-btn" @click="handleOpenSettings">
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path
                d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"
              />
              <circle cx="12" cy="12" r="3" />
            </svg>
            <span>{{ t('settings.title') }}</span>
          </button>
        </div>
      </template>
    </div>

    <!-- Sidebar Resize Handle -->
    <div class="sidebar-resize-handle" :class="{ active: isDragging }" @mousedown="startResize" />
  </aside>
</template>

<style scoped>
  /* ========== SIDEBAR ========== */
  .sidebar {
    display: flex;
    flex-direction: column;
    background: var(--sidebar-glass-bg);
    transition:
      width 0.25s ease,
      background 0.25s ease;
    overflow: hidden;
    position: relative;
    flex-shrink: 0;
  }

  /* Disable transition during drag for responsive feel */
  .sidebar--dragging {
    transition: background 0.25s ease !important;
  }

  .sidebar--collapsed {
    width: 0 !important;
  }

  .sidebar-inner {
    display: flex;
    flex-direction: column;
    height: 100%;
    flex-shrink: 0;
  }

  .sidebar-resize-handle {
    position: absolute;
    top: 0;
    right: -4px;
    width: 12px;
    height: 100%;
    background: transparent;
    cursor: col-resize;
    z-index: 10;
  }

  .sidebar-resize-handle::after {
    content: '';
    position: absolute;
    top: 12px;
    bottom: 12px;
    left: 50%;
    transform: translateX(-50%);
    width: 1px;
    background: transparent;
    transition: background 0.15s ease;
    pointer-events: none;
  }

  .sidebar-resize-handle:hover::after,
  .sidebar-resize-handle.active::after {
    background: var(--color-primary);
  }

  .sidebar-header {
    height: 40px;
    flex-shrink: 0;
    -webkit-app-region: drag;
  }

  /* Back button for settings */
  .back-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 4px 8px;
    padding: 8px 10px;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-lg);
    font-size: 13px;
    font-weight: 500;
    color: var(--text-200);
    cursor: pointer;
  }

  .back-btn:hover {
    background: var(--color-hover);
    color: var(--text-100);
  }

  .back-btn svg {
    width: 16px;
    height: 16px;
  }

  /* Settings nav */
  .settings-nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px;
  }

  .nav-item {
    padding: 8px 10px;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-lg);
    font-size: 13px;
    font-weight: 500;
    color: var(--text-200);
    cursor: pointer;
    text-align: left;
  }

  .nav-item:hover {
    background: var(--color-hover);
    color: var(--text-100);
  }

  .nav-item.active {
    background: var(--sidebar-item-active-bg);
    color: var(--text-100);
  }

  /* Threads sidebar */
  .new-thread-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 4px 8px;
    padding: 8px 10px;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-lg);
    color: var(--text-200);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    width: calc(100% - 16px);
  }

  .new-thread-btn:hover {
    background: var(--color-hover);
    color: var(--text-100);
  }

  .new-thread-btn svg {
    width: 18px;
    height: 18px;
    opacity: 0.9;
    flex-shrink: 0;
  }

  /* Skills button active state */
  .skills-btn.active {
    background: var(--sidebar-item-active-bg);
    color: var(--text-100);
  }

  .skills-btn.active svg {
    color: var(--text-300);
    opacity: 0.7;
  }

  .threads-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    margin-top: 8px;
    overflow: hidden;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 16px 6px;
  }

  .section-title {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-300);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .icon-btn {
    width: 22px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-sm);
    color: var(--text-300);
    cursor: pointer;
  }

  .icon-btn:hover {
    background: var(--color-hover);
  }

  .icon-btn svg {
    width: 16px;
    height: 16px;
  }

  .threads-list {
    flex: 1;
    overflow-y: auto;
    padding: 0 8px 8px;
  }

  .threads-list::-webkit-scrollbar {
    width: 4px;
  }

  .threads-list::-webkit-scrollbar-thumb {
    background: var(--border-200);
    border-radius: var(--border-radius-xs);
  }

  .workspace-group {
    margin-bottom: 4px;
  }

  .workspace-item {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 8px 10px;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-lg);
    color: var(--text-200);
    font-size: 13px;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
  }

  .workspace-item:hover {
    background: var(--color-hover);
    color: var(--text-100);
  }

  .workspace-item.expanded {
    color: var(--text-100);
  }

  .workspace-item.active {
    color: var(--text-100);
  }

  .workspace-item.active .folder-icon {
    opacity: 0.9;
  }

  .workspace-icon-slot {
    position: relative;
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .workspace-icon-slot .expand-icon,
  .workspace-icon-slot .folder-icon {
    position: absolute;
  }

  /* Default: show folder, hide arrow */
  .workspace-icon-slot .expand-icon {
    opacity: 0;
  }

  .workspace-icon-slot .folder-icon {
    opacity: 0.8;
  }

  /* Hover: show arrow, hide folder */
  .workspace-item:hover .workspace-icon-slot .expand-icon {
    opacity: 0.5;
  }

  .workspace-item:hover .workspace-icon-slot .folder-icon {
    opacity: 0;
  }

  /* Expanded + hover: show arrow, hide folder */
  .workspace-item:hover .workspace-icon-slot.expanded .expand-icon {
    opacity: 0.7;
  }

  .workspace-item:hover .workspace-icon-slot.expanded .folder-icon {
    opacity: 0;
  }

  .expand-icon {
    width: 14px;
    height: 14px;
    transition: transform 0.15s ease;
  }

  .expand-icon.expanded {
    transform: rotate(90deg);
  }

  .folder-icon {
    width: 16px;
    height: 16px;
  }

  .workspace-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  /* Workspace action buttons container */
  .workspace-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: auto;
    flex-shrink: 0;
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .workspace-item:hover .workspace-actions {
    opacity: 1;
  }

  /* Shared action button style (add + delete) */
  .action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-sm);
    color: var(--text-300);
    cursor: pointer;
    transition:
      background 0.15s ease,
      color 0.15s ease;
    flex-shrink: 0;
  }

  .action-btn svg {
    width: 14px;
    height: 14px;
  }

  .action-btn:hover {
    background: var(--color-hover);
    color: var(--text-100);
  }

  .action-btn.delete-btn:hover {
    color: var(--color-error, #ef4444);
  }

  .action-btn.delete-btn.confirming {
    color: var(--text-400);
  }

  /* Session-level delete button */
  .session-trailing .delete-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    position: absolute;
    right: 0;
    width: 20px;
    height: 20px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-sm);
    color: var(--text-400);
    cursor: pointer;
    opacity: 0;
    transition:
      opacity 0.15s ease,
      background 0.15s ease,
      color 0.15s ease;
    flex-shrink: 0;
  }

  .session-trailing .delete-btn svg {
    width: 14px;
    height: 14px;
  }

  .session-trailing .delete-btn:hover {
    background: var(--color-hover);
    color: var(--color-error, #ef4444);
  }

  .session-trailing .delete-btn.confirming {
    opacity: 1;
    color: var(--text-400);
  }

  .loading-indicator {
    font-size: 12px;
    color: var(--text-300);
    flex-shrink: 0;
  }

  .no-sessions {
    padding: 10px 12px;
    font-size: 12px;
    color: var(--text-300);
  }

  .sessions-list {
    padding-left: 20px;
    padding-top: 4px;
    padding-bottom: 4px;
    overflow: visible;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  /* Tree expand/collapse animation */
  .tree-expand-enter-active,
  .tree-expand-leave-active {
    transition:
      max-height 0.2s ease,
      opacity 0.2s ease;
    max-height: 500px;
  }

  .tree-expand-enter-from,
  .tree-expand-leave-to {
    max-height: 0;
    opacity: 0;
  }

  .session-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 10px;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-lg);
    color: var(--text-200);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    overflow: visible;
  }

  .session-item:hover {
    background: var(--color-hover);
  }

  .session-item.active {
    background: var(--sidebar-item-active-bg);
    color: var(--text-100);
  }

  .session-item.loading {
    color: var(--text-100);
  }

  /* Loading indicator */
  .session-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    flex-shrink: 0;
  }

  .session-loading svg {
    width: 12px;
    height: 12px;
    color: var(--text-300);
    animation: spin-loading 1s linear infinite;
  }

  @keyframes spin-loading {
    to {
      transform: rotate(360deg);
    }
  }

  .session-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .session-type-icon {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    margin-left: 6px;

    svg {
      width: 14px;
      height: 14px;
    }
  }

  .session-type-icon.agent {
    color: var(--text-300);
  }

  .session-type-icon.shell {
    color: var(--text-300);
  }

  .session-item.active .session-type-icon.agent {
    color: var(--text-200);
  }

  .session-item.active .session-type-icon.shell {
    color: var(--text-200);
  }

  .session-trailing {
    position: relative;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    overflow: visible;
  }

  .session-item:hover .session-trailing .session-time {
    opacity: 0;
  }

  .session-item:hover .session-trailing .delete-btn {
    opacity: 1;
  }

  .session-time {
    font-size: 12px;
    color: var(--text-300);
    white-space: nowrap;
  }

  .empty-state {
    padding: 24px;
    text-align: center;
    font-size: 13px;
    color: var(--text-300);
  }

  .sidebar-footer {
    padding: 8px 12px;
    margin-top: auto;
  }

  .settings-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 12px;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-md);
    color: var(--text-200);
    font-size: 14px;
    cursor: pointer;
  }

  .settings-btn:hover {
    background: var(--color-hover);
    color: var(--text-100);
  }

  .settings-btn svg {
    width: 18px;
    height: 18px;
    opacity: 0.9;
  }
</style>
