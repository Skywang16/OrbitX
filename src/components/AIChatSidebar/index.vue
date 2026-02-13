<script setup lang="ts">
  import { computed, onMounted, ref } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useAIChatStore } from './store'
  import { useAISettingsStore } from '@/components/settings/components/AI'
  import { useSessionStore } from '@/stores/session'

  import ChatHeader from './components/layout/ChatHeader.vue'
  import MessageList from './components/messages/MessageList.vue'
  import ChatInput from './components/input/ChatInput.vue'
  import ResizeHandle from './components/layout/ResizeHandle.vue'
  import ImageLightbox from './components/input/ImageLightbox.vue'
  import RollbackConfirmDialog from './components/messages/RollbackConfirmDialog.vue'
  import ToolConfirmationDialog from './components/messages/ToolConfirmationDialog.vue'

  const aiChatStore = useAIChatStore()
  const aiSettingsStore = useAISettingsStore()
  const sessionStore = useSessionStore()

  const { t } = useI18n()

  const messageInput = ref('')
  const chatInputRef = ref<InstanceType<typeof ChatInput>>()

  const isDragging = ref(false)
  const isHovering = ref(false)

  const canSend = computed(() => {
    return messageInput.value.trim().length > 0 && aiChatStore.canSendMessage
  })

  const sendMessage = async (
    images?: Array<{ id: string; dataUrl: string; fileName: string; fileSize: number; mimeType: string }>
  ) => {
    if (!canSend.value && (!images || images.length === 0)) return

    const message = messageInput.value.trim()
    messageInput.value = ''

    // 重置输入框高度
    chatInputRef.value?.adjustTextareaHeight()

    // 清空图片
    if (images && images.length > 0) {
      chatInputRef.value?.clearImages()
    }

    await aiChatStore.sendMessage(message, images)
  }

  const handleSessionSelect = async (sessionId: number) => {
    await aiChatStore.switchSession(sessionId)
  }

  const handleCreateSession = async () => {
    await aiChatStore.createSession()
  }

  const handleRefreshSessions = async () => {
    await aiChatStore.refreshSessions()
  }

  const handleSwitchMode = (mode: 'chat' | 'agent') => {
    aiChatStore.chatMode = mode
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
      label: model.model,
      value: model.id,
    }))
  })

  const handleModelChange = async (modelId: string | null) => {
    selectedModelId.value = modelId
    sessionStore.updateAiState({ selectedModelId: modelId })
  }

  const stopMessage = () => {
    aiChatStore.stopCurrentTask()
  }

  const handleRollbackResult = async (result: { success: boolean; message: string; restoreContent?: string }) => {
    console.warn('Checkpoint rollback:', result.message)
    if (result.success) {
      await aiChatStore.refreshSessions()
      if (result.restoreContent && result.restoreContent.trim().length > 0) {
        messageInput.value = result.restoreContent
        chatInputRef.value?.adjustTextareaHeight()
        chatInputRef.value?.focus()
      }
    }
  }

  onMounted(async () => {
    await aiChatStore.initialize()

    const savedModelId = sessionStore.aiState?.selectedModelId || null
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
        :sessions="aiChatStore.sessions"
        :current-session-id="aiChatStore.currentSession?.id ?? null"
        :is-loading="aiChatStore.isSending"
        @select-session="handleSessionSelect"
        @create-new-session="handleCreateSession"
        @refresh-sessions="handleRefreshSessions"
      />
      <div class="messages-and-tasks">
        <MessageList
          :messages="aiChatStore.messageList"
          :is-loading="aiChatStore.isSending"
          :chat-mode="aiChatStore.chatMode"
          :session-id="aiChatStore.currentSession?.id ?? null"
          :workspace-path="aiChatStore.currentWorkspacePath ?? ''"
        />

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
        :chat-mode="aiChatStore.chatMode"
        @send="sendMessage"
        @stop="stopMessage"
        @update:selected-model="handleModelChange"
        @mode-change="handleSwitchMode"
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
    background: var(--bg-50);
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
  }
</style>
