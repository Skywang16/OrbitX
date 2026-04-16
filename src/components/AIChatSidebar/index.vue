<script setup lang="ts">
  import { useAISettingsStore } from '@/components/settings/components/AI'
  import { type ImageAttachment } from '@/stores/imageLightbox'
  import { useLayoutStore } from '@/stores/layout'
  import { useWorkspaceStore } from '@/stores/workspace'
  import { computed, onMounted, ref } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useAIChatStore } from './store'

  import ChatInput from './components/input/ChatInput.vue'
  import ImageLightbox from './components/input/ImageLightbox.vue'
  import ChatHeader from './components/layout/ChatHeader.vue'
  import ResizeHandle from './components/layout/ResizeHandle.vue'
  import MessageList from './components/messages/MessageList.vue'
  import RollbackConfirmDialog from './components/messages/RollbackConfirmDialog.vue'
  import ThreadExecutionTimeline from './components/messages/ThreadExecutionTimeline.vue'
  import ToolConfirmationDialog from './components/messages/ToolConfirmationDialog.vue'

  const aiChatStore = useAIChatStore()
  const aiSettingsStore = useAISettingsStore()
  const layoutStore = useLayoutStore()
  const workspaceStore = useWorkspaceStore()

  const currentSessions = computed(() => {
    const path = aiChatStore.currentWorkspacePath
    return workspaceStore.getTopLevelThreads(path)
  })

  const { t } = useI18n()

  const messageInput = ref('')
  const chatInputRef = ref<InstanceType<typeof ChatInput>>()

  const isDragging = ref(false)
  const isHovering = ref(false)

  const canSend = computed(() => {
    const hasContent = messageInput.value.trim().length > 0
    if (aiChatStore.isSending) return hasContent && aiChatStore.hasWorkspace
    return hasContent && aiChatStore.canSendMessage
  })

  const sendMessage = async (images?: ImageAttachment[]) => {
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

  const handleThreadSelect = async (threadId: number) => {
    await aiChatStore.switchThread(threadId)
  }

  const handleCreateThread = async () => {
    await aiChatStore.startNewChat()
  }

  const handleGoBack = async () => {
    const parentId = workspaceStore.selectedThread?.parentThreadId
    if (parentId) await aiChatStore.switchThread(parentId)
  }

  const handleRefreshThreads = async () => {
    const path = aiChatStore.currentWorkspacePath
    if (path) {
      await workspaceStore.loadThreadViews(path)
    }
  }

  const startDrag = (event: MouseEvent) => {
    event.preventDefault()

    isDragging.value = true
    document.body.classList.add('orbitx-resizing')

    const startX = event.clientX
    const startWidth = aiChatStore.sidebarWidth

    const handleMouseMove = (e: MouseEvent) => {
      e.preventDefault()
      const deltaX = startX - e.clientX
      const newWidth = Math.max(100, Math.min(800, startWidth + deltaX))

      aiChatStore.setSidebarWidth(newWidth)
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
    aiChatStore.setSidebarWidth(250)
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
      const path = aiChatStore.currentWorkspacePath
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
  })
</script>

<template>
  <div class="ai-chat-sidebar">
    <ResizeHandle
      :is-dragging="isDragging"
      :is-hovering="isHovering"
      @mousedown="startDrag"
      @mouseenter="onMouseEnter"
      @mouseleave="onMouseLeave"
      @dblclick="onDoubleClick"
    />

    <div class="ai-chat-content">
      <ChatHeader
        :threads="currentSessions"
        :current-thread-id="aiChatStore.currentThread?.id ?? null"
        :parent-thread-id="workspaceStore.selectedThread?.parentThreadId ?? null"
        :selected-label="workspaceStore.selectedThread?.title || null"
        :is-loading="aiChatStore.isCurrentThreadSending"
        @select-thread="handleThreadSelect"
        @create-new-thread="handleCreateThread"
        @refresh-threads="handleRefreshThreads"
        @go-back="handleGoBack"
      />
      <div class="messages-and-tasks">
        <!-- No workspace welcome page -->
        <div v-if="!aiChatStore.hasWorkspace" class="welcome-page">
          <div class="welcome-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
            </svg>
          </div>
          <h2 class="welcome-title">{{ t('welcome.title') }}</h2>
          <p class="welcome-description">{{ t('welcome.description') }}</p>
        </div>

        <!-- Normal message list -->
        <template v-else>
          <ThreadExecutionTimeline
            :thread-id="aiChatStore.currentThread?.id ?? null"
            scroll-container-id="sidebar-chat-message-list"
          />
          <MessageList
            dom-id="sidebar-chat-message-list"
            :messages="aiChatStore.messageList"
            :is-loading="aiChatStore.isCurrentThreadSending"
            :thread-id="aiChatStore.currentThread?.id ?? null"
            :workspace-path="aiChatStore.currentWorkspacePath ?? ''"
          />
        </template>

        <!--  <TaskList /> -->
      </div>

      <ToolConfirmationDialog />
      <ChatInput
        ref="chatInputRef"
        v-model="messageInput"
        :placeholder="t('chat.input_placeholder')"
        :loading="aiChatStore.isSending"
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
  </div>
</template>

<style scoped>
  .ai-chat-sidebar {
    position: relative;
    width: 100%;
    height: 100%;
    background: var(--bg-100-solid);
    border-left: 1px solid var(--border-200);
    display: flex;
    flex-direction: column;
    min-width: 10vw;
  }

  .ai-chat-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .messages-and-tasks {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    padding: 0 20px;
    position: relative;
  }

  .messages-and-tasks::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 60px;
    pointer-events: none;
    background: linear-gradient(to top, var(--bg-100-solid) 0%, transparent 100%);
    z-index: 1;
  }

  /* Welcome page styles */
  .welcome-page {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    text-align: center;
    padding: 40px 20px;
    color: var(--text-400);
  }

  .welcome-icon {
    width: 64px;
    height: 64px;
    margin-bottom: 24px;
    opacity: 0.5;
  }

  .welcome-icon svg {
    width: 100%;
    height: 100%;
  }

  .welcome-title {
    font-size: 20px;
    font-weight: 600;
    color: var(--text-200);
    margin: 0 0 12px 0;
  }

  .welcome-description {
    font-size: 14px;
    line-height: 1.6;
    color: var(--text-400);
    margin: 0;
    max-width: 280px;
  }
</style>
