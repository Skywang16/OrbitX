<script setup lang="ts">
  import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { listen } from '@tauri-apps/api/event'
  import { useAIChatStore } from './store'
  import { useAISettingsStore } from '@/components/settings/components/AI'
  import { useSessionStore } from '@/stores/session'
  import { workspaceIndexApi } from '@/api/workspace-index'
  import { terminalContextApi } from '@/api/terminal-context'
  import type { WorkspaceIndex } from '@/api/workspace-index'

  import ChatHeader from './components/ChatHeader.vue'
  import MessageList from './components/MessageList.vue'
  import ChatInput from './components/ChatInput.vue'
  import ResizeHandle from './components/ResizeHandle.vue'
  import TaskList from './components/TaskList.vue'
  import WorkspaceIndexPrompt from './components/WorkspaceIndexPrompt.vue'

  const aiChatStore = useAIChatStore()
  const aiSettingsStore = useAISettingsStore()
  const sessionStore = useSessionStore()

  const { t } = useI18n()

  const messageInput = ref('')
  const chatInputRef = ref<InstanceType<typeof ChatInput>>()

  const isDragging = ref(false)
  const isHovering = ref(false)

  // 工作区索引状态
  const currentWorkspacePath = ref<string | null>(null)
  const currentWorkspaceIndex = ref<WorkspaceIndex | null>(null)
  const isCheckingIndex = ref(false)
  const indexCheckError = ref<string>('')

  const canSend = computed(() => {
    return messageInput.value.trim().length > 0 && aiChatStore.canSendMessage
  })

  const hasWorkspaceIndex = computed(() => {
    return currentWorkspaceIndex.value?.status === 'ready'
  })

  const sendMessage = async () => {
    if (!canSend.value) return

    const message = messageInput.value.trim()
    messageInput.value = ''

    try {
      await aiChatStore.sendMessage(message)
    } catch (error) {
      // Error handling is done by the store
    }
  }

  const selectSession = (sessionId: number) => {
    aiChatStore.switchToConversation(sessionId)
  }

  const deleteSession = (sessionId: number) => {
    aiChatStore.deleteConversation(sessionId)
  }

  const refreshSessions = async () => {
    try {
      await aiChatStore.refreshConversations()
    } catch {
      // Refresh failures are non-critical
    }
  }

  const createNewSession = () => {
    aiChatStore.createConversation()
  }

  const handleSwitchMode = async (mode: 'chat' | 'agent') => {
    aiChatStore.chatMode = mode
    await aiChatStore.initializeEko()
    await aiChatStore.ekoInstance?.setMode(mode)
  }

  // 拖拽调整功能
  const startDrag = (event: MouseEvent) => {
    isDragging.value = true
    const startX = event.clientX
    const startWidth = aiChatStore.sidebarWidth

    const handleMouseMove = (e: MouseEvent) => {
      const deltaX = startX - e.clientX
      const newWidth = Math.max(100, Math.min(800, startWidth + deltaX))

      aiChatStore.setSidebarWidth(newWidth)
    }

    const handleMouseUp = () => {
      isDragging.value = false

      // 如果宽度太小，退出聊天模式
      if (aiChatStore.sidebarWidth <= 120) {
        aiChatStore.isVisible = false
      }

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
    aiChatStore.setSidebarWidth(250)
  }

  const selectedModelId = ref<string | null>(null)

  const modelOptions = computed(() => {
    return aiSettingsStore.chatModels.map(model => ({
      label: model.name,
      value: model.id,
    }))
  })

  const handleModelChange = async (modelId: string | null) => {
    selectedModelId.value = modelId
    sessionStore.updateAiState({ selectedModelId: modelId })

    // 更新EKO实例的模型配置
    await aiChatStore.updateSelectedModel(modelId)
  }

  const stopMessage = () => {
    if (aiChatStore.isLoading) {
      if (aiChatStore.cancelFunction) {
        aiChatStore.cancelFunction()
        aiChatStore.cancelFunction = null
      }
      aiChatStore.isLoading = false
    }
  }

  // 检查当前工作区索引状态
  const checkWorkspaceIndex = async () => {
    try {
      isCheckingIndex.value = true
      indexCheckError.value = ''

      // 获取当前工作目录
      const currentPath = await terminalContextApi.getCurrentWorkingDirectory()
      currentWorkspacePath.value = currentPath

      if (currentPath) {
        // 检查索引状态
        const workspace = await workspaceIndexApi.checkCurrentWorkspace()
        currentWorkspaceIndex.value = workspace
      } else {
        currentWorkspaceIndex.value = null
      }
    } catch (error) {
      console.error('检查工作区索引失败:', error)
      indexCheckError.value = error instanceof Error ? error.message : '检查工作区索引失败'
      currentWorkspaceIndex.value = null
      currentWorkspacePath.value = null // 确保路径也被重置
    } finally {
      isCheckingIndex.value = false
    }
  }

  // 处理索引构建完成
  const handleIndexBuilt = (workspace: WorkspaceIndex) => {
    currentWorkspaceIndex.value = workspace
    // 可以在这里添加成功提示
  }

  // 处理索引构建开始
  const handleBuildStarted = () => {
    // 可以在这里添加构建开始的处理逻辑
  }

  // 处理索引构建取消
  const handleBuildCancelled = () => {
    // 可以在这里添加构建取消的处理逻辑
  }

  watch(
    () => aiSettingsStore.chatModels,
    newModels => {
      if (newModels.length === 0) return

      const targetModelId = sessionStore.sessionState.ai.selectedModelId
      const validModel = newModels.find(m => m.id === targetModelId) || newModels[0]

      if (selectedModelId.value !== validModel.id) {
        selectedModelId.value = validModel.id
        sessionStore.updateAiState({ selectedModelId: validModel.id })
      }
    },
    { immediate: true }
  )

  // 事件监听器存储
  let unlistenActivePane: (() => void) | null = null
  let unlistenPaneContext: (() => void) | null = null

  // 设置事件监听
  const setupEventListeners = async () => {
    // 监听活跃面板变化事件
    unlistenActivePane = await listen('active_pane_changed', async () => {
      // 延迟检查，确保后端状态已稳定
      setTimeout(async () => {
        await checkWorkspaceIndex()
      }, 300)
    })

    // 监听面板上下文更新事件
    unlistenPaneContext = await listen('pane_context_updated', async () => {
      await checkWorkspaceIndex()
    })
  }

  onMounted(async () => {
    if (!aiSettingsStore.isInitialized) {
      await aiSettingsStore.loadSettings()
    }

    if (!aiChatStore.isInitialized) {
      await aiChatStore.initialize()
    }

    // 设置事件监听
    await setupEventListeners()

    // 检查工作区索引状态
    await checkWorkspaceIndex()
  })

  onUnmounted(() => {
    // 清理事件监听器
    if (unlistenActivePane) {
      unlistenActivePane()
    }
    if (unlistenPaneContext) {
      unlistenPaneContext()
    }
  })
</script>

<template>
  <div v-if="aiChatStore.isVisible" class="ai-chat-sidebar" :style="{ width: `${aiChatStore.sidebarWidth}px` }">
    <ResizeHandle
      :is-dragging="isDragging"
      :is-hovering="isHovering"
      @mousedown="startDrag"
      @mouseenter="onMouseEnter"
      @mouseleave="onMouseLeave"
      @dblclick="onDoubleClick"
    />

    <ChatHeader
      :sessions="aiChatStore.conversations"
      :current-session-id="aiChatStore.currentConversationId"
      :is-loading="aiChatStore.isLoading"
      @select-session="selectSession"
      @create-new-session="createNewSession"
      @delete-session="deleteSession"
      @refresh-sessions="refreshSessions"
    />

    <!-- 工作区索引状态检测 -->
    <div v-if="!hasWorkspaceIndex && !isCheckingIndex" class="workspace-index-container">
      <WorkspaceIndexPrompt
        :current-path="currentWorkspacePath || undefined"
        @index-built="handleIndexBuilt"
        @build-started="handleBuildStarted"
        @build-cancelled="handleBuildCancelled"
      />
    </div>

    <!-- 正常聊天界面 -->
    <template v-else-if="hasWorkspaceIndex">
      <MessageList :messages="aiChatStore.messageList" />

      <TaskList :task-nodes="aiChatStore.currentTaskNodes" :task-id="aiChatStore.currentTaskId || ''" />

      <ChatInput
        ref="chatInputRef"
        v-model="messageInput"
        :loading="aiChatStore.isLoading"
        :can-send="canSend"
        :selected-model="selectedModelId"
        :model-options="modelOptions"
        :chat-mode="aiChatStore.chatMode"
        :placeholder="t('session.chat_placeholder')"
        :has-tasks="aiChatStore.currentTaskNodes.length > 0"
        @send="sendMessage"
        @stop="stopMessage"
        @model-change="handleModelChange"
        @mode-change="handleSwitchMode"
      />
    </template>

    <!-- 检查索引状态时的加载提示 -->
    <div v-else-if="isCheckingIndex" class="checking-index">
      <div class="loading-spinner"></div>
      <p>{{ t('storage.checking_workspace_index', '检查工作区索引状态...') }}</p>
    </div>
  </div>
</template>

<style scoped>
  .ai-chat-sidebar {
    height: 100%;
    background-color: var(--bg-300);
    border-left: 1px solid var(--border-300);
    display: flex;
    flex-direction: column;
    position: relative;
  }

  .ai-chat-sidebar > :not(.resize-handle):not(.exit-hint) {
    position: relative;
    z-index: 1;
  }

  .exit-hint {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    background: var(--bg-500);
    color: var(--text-200);
    padding: 8px 16px;
    border-radius: var(--border-radius-sm);
    border: 1px solid var(--border-300);
    font-size: 12px;
    z-index: 100;
    white-space: nowrap;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  }

  .workspace-index-container {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
  }

  .checking-index {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 40px 20px;
    color: var(--text-200);
  }

  .loading-spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border-300);
    border-top: 2px solid var(--accent-500);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 16px;
  }

  @keyframes spin {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }
</style>
