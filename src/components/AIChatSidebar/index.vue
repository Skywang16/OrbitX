<script setup lang="ts">
  import { computed, onMounted, onBeforeUnmount, ref } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useAIChatStore } from './store'
  import { useAISettingsStore } from '@/components/settings/components/AI'
  import { useSessionStore } from '@/stores/session'

  import ChatHeader from './components/layout/ChatHeader.vue'
  import MessageList from './components/messages/MessageList.vue'
  import ChatInput from './components/input/ChatInput.vue'
  import ResizeHandle from './components/layout/ResizeHandle.vue'
  //  import TaskList from './components/TaskList.vue'

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

  const sendMessage = async () => {
    if (!canSend.value) return

    const message = messageInput.value.trim()
    messageInput.value = ''

    // 重置输入框高度
    chatInputRef.value?.adjustTextareaHeight()

    await aiChatStore.sendMessage(message)
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

  onMounted(async () => {
    await aiChatStore.initialize()

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
