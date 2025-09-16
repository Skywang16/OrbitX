<script setup lang="ts">
  import { computed, onMounted, ref } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useAIChatStore } from './store'
  import { useAISettingsStore } from '@/components/settings/components/AI'
  import { useSessionStore } from '@/stores/session'

  import ChatHeader from './components/ChatHeader.vue'
  import MessageList from './components/MessageList.vue'
  import ChatInput from './components/ChatInput.vue'
  import ResizeHandle from './components/ResizeHandle.vue'
  import TaskList from './components/TaskList.vue'

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
</script>

<template>
  <div class="ai-chat-sidebar" :style="{ width: aiChatStore.sidebarWidth + 'px' }">
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

      <TaskList v-if="aiChatStore.chatMode === 'agent'" :task-nodes="[]" />

      <MessageList
        :messages="aiChatStore.messageList"
        :is-loading="aiChatStore.isLoading"
        :chat-mode="aiChatStore.chatMode"
      />

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
    height: 100%;
    background: var(--bg-50);
    border-left: 1px solid var(--border-200);
    display: flex;
    flex-direction: column;
    min-width: 100px;
  }

  .ai-chat-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }
</style>
