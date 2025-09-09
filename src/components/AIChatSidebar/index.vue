<script setup lang="ts">
  import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
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

  onMounted(async () => {
    if (!aiSettingsStore.isInitialized) {
      await aiSettingsStore.loadSettings()
    }

    if (!aiChatStore.isInitialized) {
      await aiChatStore.initialize()
    }
  })

  onUnmounted(() => {})
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
</style>
