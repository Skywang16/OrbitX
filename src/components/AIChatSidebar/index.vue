<script setup lang="ts">
  import { computed, onMounted, onBeforeUnmount, ref } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useAIChatStore } from './store'
  import { useAISettingsStore } from '@/components/settings/components/AI'
  import { useSessionStore } from '@/stores/session'
  import { windowApi } from '@/api/window'
  import { agentApi } from '@/api/agent'

  import ChatHeader from './components/layout/ChatHeader.vue'
  import MessageList from './components/messages/MessageList.vue'
  import ChatInput from './components/input/ChatInput.vue'
  import ResizeHandle from './components/layout/ResizeHandle.vue'
  import ImageLightbox from './components/input/ImageLightbox.vue'
  import RollbackConfirmDialog from './components/messages/RollbackConfirmDialog.vue'
  import { useCheckpoint } from '@/composables/useCheckpoint'

  const aiChatStore = useAIChatStore()
  const aiSettingsStore = useAISettingsStore()
  const sessionStore = useSessionStore()
  const workspacePath = ref('')

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

  const selectSession = (sessionId: number) => {
    aiChatStore.switchToConversation(sessionId)
  }

  const deleteSession = (sessionId: number) => {
    aiChatStore.deleteConversation(sessionId)
  }

  const refreshSessions = async () => {
    await aiChatStore.refreshConversations()
  }

  const createNewSession = () => {
    aiChatStore.createConversation()
  }

  const handleSwitchMode = async (mode: 'chat' | 'agent') => {
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
    if (aiChatStore.isLoading) {
      if (aiChatStore.cancelFunction) {
        aiChatStore.cancelFunction()
        aiChatStore.cancelFunction = null
      }
      aiChatStore.isLoading = false
    }
  }

  const { refreshCheckpoints } = useCheckpoint()

  // 处理消息回滚
  const handleRollbackToMessage = async (messageId: number, content: string) => {
    if (!aiChatStore.currentConversationId) return

    try {
      // 调用后端删除消息
      await agentApi.deleteMessagesFrom(aiChatStore.currentConversationId, messageId)

      // 重新加载消息列表
      await aiChatStore.loadConversation(aiChatStore.currentConversationId)

      // 将消息内容填充到输入框
      messageInput.value = content
    } catch (error) {
      console.error('Failed to rollback messages:', error)
    }
  }

  // 处理文件回滚结果
  const handleRollbackResult = async (result: { success: boolean; message: string; messageId: number }) => {
    if (result.success) {
      // 找到被回滚的消息
      const message = aiChatStore.messageList.find(m => m.id === result.messageId)
      const content = message?.content || ''
      const images = message?.images || []

      // 执行消息回滚
      await handleRollbackToMessage(result.messageId, content)

      // 恢复图片到输入框
      if (images.length > 0) {
        chatInputRef.value?.setImages(images)
      }

      // 刷新 checkpoints
      if (aiChatStore.currentConversationId) {
        await refreshCheckpoints(aiChatStore.currentConversationId)
      }
    }
    console.warn('Checkpoint rollback:', result.message)
  }

  onMounted(async () => {
    await aiChatStore.initialize()

    // 获取当前工作目录
    try {
      workspacePath.value = await windowApi.getCurrentDirectory()
    } catch (e) {
      console.warn('Failed to get workspace path:', e)
    }

    // 从 sessionStore 恢复选中的模型（改用公开的计算属性）
    const savedModelId = sessionStore.aiState?.selectedModelId || null
    if (savedModelId) {
      selectedModelId.value = savedModelId
    } else if (modelOptions.value.length > 0) {
      selectedModelId.value = String(modelOptions.value[0].value)
    }

    await handleModelChange(selectedModelId.value)
  })

  // 在组件卸载前保存状态
  onBeforeUnmount(() => {
    // Task state is now managed by TaskManager, no need to save manually
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
        :sessions="aiChatStore.conversations"
        :current-session-id="aiChatStore.currentConversationId"
        :is-loading="aiChatStore.isLoading"
        @select-session="selectSession"
        @create-new-session="createNewSession"
        @delete-session="deleteSession"
        @refresh-sessions="refreshSessions"
      />
      <div class="messages-and-tasks">
        <MessageList
          :messages="aiChatStore.messageList"
          :is-loading="aiChatStore.isLoading"
          :chat-mode="aiChatStore.chatMode"
          :conversation-id="aiChatStore.currentConversationId"
          :workspace-path="workspacePath"
        />

        <!--  <TaskList /> -->
      </div>

      <ChatInput
        ref="chatInputRef"
        v-model="messageInput"
        :placeholder="t('chat.input_placeholder')"
        :loading="aiChatStore.isLoading"
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
