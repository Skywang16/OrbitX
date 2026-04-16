<script setup lang="ts">
  /**
   * MainChatArea - Main chat area
   * Reuses all logic from AIChatSidebar, only changes layout (from sidebar to main body)
   */
  import { useAIChatStore } from '@/components/AIChatSidebar/store'
  import { useAISettingsStore } from '@/components/settings/components/AI'
  import { useGitStore } from '@/stores/git'
  import { useLayoutStore } from '@/stores/layout'
  import { useRunActionsStore, type RunActionRecord } from '@/stores/runActions'
  import { useTerminalStore } from '@/stores/Terminal'
  import { useWorkspaceStore } from '@/stores/workspace'
  import { showPopoverAt } from '@/ui'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { storeToRefs } from 'pinia'
  import { computed, onMounted, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'

  // Reuse AIChatSidebar sub-components
  import ChatInput from '@/components/AIChatSidebar/components/input/ChatInput.vue'
  import ImageLightbox from '@/components/AIChatSidebar/components/input/ImageLightbox.vue'
  import MessageList from '@/components/AIChatSidebar/components/messages/MessageList.vue'
  import RollbackConfirmDialog from '@/components/AIChatSidebar/components/messages/RollbackConfirmDialog.vue'
  import ThreadExecutionTimeline from '@/components/AIChatSidebar/components/messages/ThreadExecutionTimeline.vue'
  import ToolConfirmationDialog from '@/components/AIChatSidebar/components/messages/ToolConfirmationDialog.vue'
  import AddActionDialog from './AddActionDialog.vue'
  import AnimatedNumber from './AnimatedNumber.vue'
  import CommitDialog from './CommitDialog.vue'

  interface Props {
    sidebarCollapsed?: boolean
  }

  defineProps<Props>()

  const aiChatStore = useAIChatStore()
  const aiSettingsStore = useAISettingsStore()
  const workspaceStore = useWorkspaceStore()
  const gitStore = useGitStore()
  const terminalStore = useTerminalStore()
  const runActionsStore = useRunActionsStore()
  const layoutStore = useLayoutStore()

  const { t } = useI18n()
  const { isRepository, changedCount, diffAdditions, diffDeletions } = storeToRefs(gitStore)
  const { selectedThread } = storeToRefs(workspaceStore)

  // Use storeToRefs to ensure reactive tracking
  const { messageList, currentThread, currentWorkspacePath, isSending, isCurrentThreadSending } =
    storeToRefs(aiChatStore)
  const { actions, selectedAction } = storeToRefs(runActionsStore)
  const { terminalPanelVisible } = storeToRefs(layoutStore)

  // Whether there's an active workspace (show Run button only when session is selected)
  const hasActiveWorkspace = computed(() => !!selectedThread.value)

  // Dialog state
  const showCommitDialog = ref(false)
  const showAddActionDialog = ref(false)
  const editingAction = ref<RunActionRecord | null>(null)

  // Commit dropdown menu
  const handleCommitClick = async (event: MouseEvent) => {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()

    const items = [
      {
        label: t('git.commit'),
        onClick: () => {
          showCommitDialog.value = true
        },
      },
      {
        label: t('git.push'),
        disabled: !isRepository.value,
        onClick: async () => {
          await gitStore.push()
        },
      },
      {
        label: t('git.create_pr'),
        disabled: true,
        onClick: () => {
          console.warn('Create PR not yet implemented')
        },
      },
    ]

    await showPopoverAt(rect.left, rect.bottom + 4, items)
  }

  const handleCommitSuccess = () => {
    // Handle after successful commit
  }

  // Run Actions handlers
  const handleRunClick = async (event: MouseEvent) => {
    const group = (event.currentTarget as HTMLElement).closest('.run-button-group') as HTMLElement
    const rect = group.getBoundingClientRect()
    const selectedId = runActionsStore.selectedActionId

    const items = [
      ...actions.value.map(action => ({
        label: action.id === selectedId ? `✓  ${action.name}` : `    ${action.name}`,
        value: action.id,
        onClick: () => {
          runActionsStore.selectAction(action.id)
        },
      })),
      ...(actions.value.length > 0 ? [{ label: '─────', disabled: true, onClick: () => {} }] : []),
      {
        label: t('run_actions.add_action'),
        onClick: () => {
          setTimeout(() => {
            editingAction.value = null
            showAddActionDialog.value = true
          }, 50)
        },
      },
    ]

    await showPopoverAt(rect.left, rect.bottom + 4, items)
  }

  const handleRunButtonClick = async () => {
    await executeSelectedAction()
  }

  const executeSelectedAction = async () => {
    const action = selectedAction.value
    if (!action) return

    // Show terminal panel
    layoutStore.openTerminalPanel()

    // Get current working directory
    const workDir = currentWorkspacePath.value || undefined

    // Try to get active terminal
    let terminalId = terminalStore.activeTerminalId
    if (terminalId === null && terminalStore.terminals.length > 0) {
      terminalId = terminalStore.terminals[0].id
    }

    if (terminalId !== null) {
      await terminalStore.writeToTerminal(terminalId, action.command, true)
    } else {
      terminalId = await terminalStore.createTerminalPane(workDir)
      await terminalStore.setActiveTerminal(terminalId)
      await new Promise(resolve => setTimeout(resolve, 500))
      await terminalStore.writeToTerminal(terminalId, action.command, true)
    }
  }

  const handleSaveAction = async (data: { name: string; command: string }) => {
    if (editingAction.value) {
      await runActionsStore.updateAction(editingAction.value.id, data.name, data.command)
    } else {
      const newAction = await runActionsStore.addAction(data.name, data.command)
      if (newAction) {
        await runActionsStore.selectAction(newAction.id)
      }
    }
  }

  const handleGoBack = async () => {
    const parentId = selectedThread.value?.parentThreadId
    if (parentId) await aiChatStore.switchThread(parentId)
  }

  const handleNewSession = async () => {
    await aiChatStore.startNewChat()
  }

  const messageInput = ref('')
  const chatInputRef = ref<InstanceType<typeof ChatInput>>()

  const canSend = computed(() => {
    const hasContent = messageInput.value.trim().length > 0
    if (aiChatStore.isSending) return hasContent && aiChatStore.hasWorkspace
    return hasContent && aiChatStore.canSendMessage
  })

  const sendMessage = async (
    images?: Array<{ id: string; dataUrl: string; fileName: string; fileSize: number; mimeType: string }>
  ) => {
    if (!canSend.value && (!images || images.length === 0)) return

    const message = messageInput.value.trim()
    messageInput.value = ''

    chatInputRef.value?.adjustTextareaHeight()

    if (images && images.length > 0) {
      chatInputRef.value?.clearImages()
    }

    if (aiChatStore.isSending) {
      aiChatStore.enqueueMessage(message, images)
      return
    }

    await aiChatStore.sendMessage(message, images)
  }

  const selectedModelId = ref<string | null>(null)

  const modelOptions = computed(() => {
    return aiSettingsStore.chatModels.map(model => ({
      label: model.displayName || model.model,
      value: model.id,
    }))
  })

  const handleModelChange = async (modelId: string | null) => {
    selectedModelId.value = modelId
    layoutStore.setSelectedModelId(modelId)
  }

  const stopMessage = () => {
    aiChatStore.stopCurrentTask()
  }

  const handleRollbackResult = async (result: { success: boolean; message: string; restoreContent?: string }) => {
    console.warn('Checkpoint rollback:', result.message)
    if (result.success) {
      const path = currentWorkspacePath.value
      if (path) {
        await workspaceStore.loadThreadViews(path)
      }
      if (result.restoreContent && result.restoreContent.trim().length > 0) {
        messageInput.value = result.restoreContent
        chatInputRef.value?.adjustTextareaHeight()
        chatInputRef.value?.focus()
      }
    }
  }

  onMounted(async () => {
    await aiChatStore.initialize()

    const savedModelId = layoutStore.selectedModelId
    if (savedModelId) {
      selectedModelId.value = savedModelId
    } else if (modelOptions.value.length > 0) {
      selectedModelId.value = String(modelOptions.value[0].value)
    }

    await handleModelChange(selectedModelId.value)

    // Refresh Git status
    await gitStore.refreshStatus()
  })

  // Watch workspace path changes, refresh Git status
  watch(currentWorkspacePath, async () => {
    await gitStore.refreshStatus()
  })

  // Load run actions when workspace changes
  watch(
    currentWorkspacePath,
    async path => {
      await runActionsStore.load(path ?? null)
      // Sync selected action from workspace record
      const wsPath = path
      if (wsPath) {
        const node = workspaceStore.getNode(wsPath)
        runActionsStore.syncSelectedFromWorkspace(node?.workspace.selectedRunActionId)
      }
    },
    { immediate: true }
  )

  // Window drag and double-click to maximize/restore
  const startDrag = async () => {
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
</script>

<template>
  <div class="main-chat-area">
    <!-- Chat Header with Drag Region -->
    <div
      class="chat-header"
      :class="{ 'chat-header--expanded': sidebarCollapsed }"
      @mousedown="startDrag"
      @dblclick="handleDoubleClick"
    >
      <div class="header-left" @mousedown.stop @dblclick.stop>
        <button v-if="selectedThread?.parentThreadId" class="back-btn" @click="handleGoBack">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
            <path d="M19 12H5M12 19l-7-7 7-7" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>
        <h1 class="chat-title">
          {{ currentThread?.title || t('chat.new_session_placeholder') }}
        </h1>
      </div>

      <div class="header-center">
        <!-- Drag area -->
      </div>

      <div class="header-right" @mousedown.stop @dblclick.stop>
        <!-- Run Button (only show when has active session/workspace) -->
        <div v-if="hasActiveWorkspace" class="run-button-group">
          <button
            class="toolbar-btn toolbar-btn--run"
            :class="{ 'toolbar-btn--disabled': !selectedAction }"
            :disabled="!selectedAction"
            :title="selectedAction?.name || t('run_actions.add_action')"
            @click.stop="handleRunButtonClick"
          >
            <svg class="toolbar-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path
                d="M6 6.804a2.5 2.5 0 0 1 3.771-2.148l9.165 5.197a2.5 2.5 0 0 1 0 4.34l-9.165 5.197A2.5 2.5 0 0 1 6 17.242V6.804z"
              />
            </svg>
          </button>
          <button class="toolbar-btn toolbar-btn--dropdown" @click.stop="handleRunClick">
            <svg class="toolbar-chevron" viewBox="0 0 12 12">
              <path
                d="M3 4.5L6 7.5L9 4.5"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </button>
        </div>

        <!-- Commit Button -->
        <button
          class="toolbar-btn"
          :class="{ 'toolbar-btn--active': changedCount > 0 }"
          @click.stop="handleCommitClick"
        >
          <svg class="toolbar-icon" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
            <circle cx="8" cy="8" r="2.5" />
            <line x1="0" y1="8" x2="5.5" y2="8" />
            <line x1="10.5" y1="8" x2="16" y2="8" />
          </svg>
          <span>{{ t('header.commit') }}</span>
          <svg class="toolbar-chevron" viewBox="0 0 12 12">
            <path
              d="M3 4.5L6 7.5L9 4.5"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </button>

        <!-- Icon Buttons -->
        <button class="toolbar-icon-btn" :title="t('chat.new_session')" @click.stop="handleNewSession">
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.25">
            <rect x="2" y="2" width="12" height="12" rx="2" ry="2" />
            <line x1="8" y1="5" x2="8" y2="11" stroke-linecap="round" />
            <line x1="5" y1="8" x2="11" y2="8" stroke-linecap="round" />
          </svg>
        </button>

        <!-- Terminal Toggle Button -->
        <button
          class="toolbar-icon-btn"
          :class="{ 'toolbar-icon-btn--active': terminalPanelVisible }"
          :title="t('layout.toggle_terminal')"
          @click.stop="layoutStore.toggleTerminalPanel"
        >
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.25">
            <rect x="2" y="2" width="12" height="12" rx="2" ry="2" />
            <path d="M5 6l2 2-2 2" stroke-linecap="round" stroke-linejoin="round" />
            <line x1="8" y1="10" x2="11" y2="10" stroke-linecap="round" />
          </svg>
        </button>

        <!-- Git Stats -->
        <div v-if="isRepository && (diffAdditions > 0 || diffDeletions > 0 || changedCount > 0)" class="toolbar-stats">
          <span class="stats-divider" />
          <span class="stat-add"><AnimatedNumber :value="diffAdditions" prefix="+" /></span>
          <span class="stat-del"><AnimatedNumber :value="diffDeletions" prefix="-" /></span>
        </div>
      </div>
    </div>

    <!-- Messages Area -->
    <div class="messages-area">
      <ThreadExecutionTimeline :thread-id="currentThread?.id ?? null" scroll-container-id="main-chat-message-list" />
      <MessageList
        dom-id="main-chat-message-list"
        :messages="messageList"
        :is-loading="isCurrentThreadSending"
        :thread-id="currentThread?.id ?? null"
        :workspace-path="currentWorkspacePath ?? ''"
      />
    </div>

    <!-- Input Area -->
    <div class="input-area">
      <ToolConfirmationDialog />
      <ChatInput
        ref="chatInputRef"
        v-model="messageInput"
        :placeholder="t('chat.input_placeholder')"
        :loading="isSending"
        :can-send="canSend"
        :selected-model="selectedModelId"
        :model-options="modelOptions"
        @send="sendMessage"
        @stop="stopMessage"
        @update:selected-model="handleModelChange"
      />
    </div>

    <ImageLightbox />
    <RollbackConfirmDialog @rollback="handleRollbackResult" />
    <CommitDialog v-model:visible="showCommitDialog" @success="handleCommitSuccess" />
    <AddActionDialog v-model:visible="showAddActionDialog" :edit-action="editingAction" @save="handleSaveAction" />
  </div>
</template>

<style scoped>
  .main-chat-area {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-100);
  }

  .messages-area {
    position: relative;
    flex: 1;
    min-height: 0;
  }

  .chat-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 20px 0 16px;
    flex-shrink: 0;
    -webkit-app-region: drag;
    transition: padding-left 0.25s ease;
    position: relative;
    z-index: 10;
  }

  .chat-header--expanded {
    padding-left: 146px;
  }

  .header-left {
    display: flex;
    align-items: center;
    min-width: 0;
    -webkit-app-region: no-drag;
  }

  .back-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: none;
    color: var(--text-300);
    cursor: pointer;
    padding: 4px;
    border-radius: 6px;
    flex-shrink: 0;
    margin-right: 4px;
  }

  .back-btn:hover {
    color: var(--text-100);
    background: var(--bg-300);
  }

  .chat-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-200);
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .header-center {
    flex: 1;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
    -webkit-app-region: no-drag;
  }

  /* Run Button Group */
  .run-button-group {
    display: inline-flex;
    align-items: center;
    height: 26px;
    background: var(--bg-100);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius-lg);
    overflow: hidden;
  }

  .run-button-group:hover {
    border-color: var(--border-400);
  }

  /* Toolbar Button (Open, Commit, Run) */
  .toolbar-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 26px;
    padding: 0 10px;
    background: var(--bg-100);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius-lg);
    color: var(--text-300);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.12s ease;
    white-space: nowrap;
  }

  .toolbar-btn:hover {
    background: var(--bg-200);
    border-color: var(--border-400);
    color: var(--text-200);
  }

  .toolbar-btn--run {
    height: 24px;
    padding: 0 8px;
    background: transparent;
    border: none;
    border-radius: 0;
    color: var(--text-300);
  }

  .toolbar-btn--run:hover:not(:disabled) {
    background: var(--bg-200);
    color: var(--text-200);
  }

  .toolbar-btn--run:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .toolbar-btn--dropdown {
    height: 24px;
    padding: 0 6px;
    background: transparent;
    border: none;
    border-radius: 0;
    color: var(--text-400);
    min-width: 20px;
  }

  .toolbar-btn--dropdown:hover {
    background: var(--bg-200);
    color: var(--text-200);
  }

  .toolbar-btn--dropdown:active {
    background: var(--bg-300);
  }

  .toolbar-btn--icon {
    padding: 0 6px;
    gap: 2px;
  }

  .toolbar-btn--active {
    color: var(--text-200);
  }

  .toolbar-btn--disabled {
    opacity: 0.5;
  }

  .toolbar-icon {
    width: 13px;
    height: 13px;
    flex-shrink: 0;
    opacity: 0.8;
  }

  .toolbar-chevron {
    width: 10px;
    height: 10px;
    opacity: 0.5;
    flex-shrink: 0;
    margin-left: -2px;
  }

  /* Toolbar Icon Button */
  .toolbar-icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-sm);
    color: var(--text-300);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .toolbar-icon-btn:hover {
    background: var(--color-hover);
    color: var(--color-primary);
  }

  .toolbar-icon-btn:active {
    transform: scale(0.95);
  }

  .toolbar-icon-btn--active {
    background: var(--color-hover);
    color: var(--color-primary);
  }

  .toolbar-icon-btn--active:hover {
    background: var(--color-hover);
    color: var(--color-primary);
  }

  .toolbar-icon-btn--active svg {
    opacity: 1;
  }

  .toolbar-icon-btn svg {
    width: 14px;
    height: 14px;
    opacity: 0.8;
  }

  /* Git Stats */
  .toolbar-stats {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 8px;
    font-size: 12px;
    font-weight: 500;
    font-family: var(--font-family-mono);
  }

  .stats-divider {
    width: 1px;
    height: 14px;
    background: var(--border-300);
    border-radius: var(--border-radius-xs);
  }

  .stat-add,
  .stat-add :deep(.animated-number) {
    color: var(--color-success);
  }

  .stat-del,
  .stat-del :deep(.animated-number) {
    color: var(--color-error);
  }

  .messages-area {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    position: relative;
  }

  /* Top gradient mask - fade-in effect for scroll content */
  .messages-area::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 24px;
    pointer-events: none;
    background: linear-gradient(to bottom, var(--bg-100) 0%, transparent 100%);
    z-index: 1;
  }

  /* Bottom gradient mask - fade-out effect for scroll content */
  .messages-area::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 60px;
    pointer-events: none;
    background: linear-gradient(to top, var(--bg-100) 0%, transparent 100%);
    z-index: 1;
  }

  .input-area {
    flex-shrink: 0;
    padding: 0;
  }
</style>
