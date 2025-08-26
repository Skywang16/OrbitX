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

  // 状态管理
  const aiChatStore = useAIChatStore() // AI聊天状态管理
  const aiSettingsStore = useAISettingsStore() // AI设置状态管理
  const sessionStore = useSessionStore() // 会话状态管理
  const { t } = useI18n()

  // 本地状态
  const messageInput = ref('')
  const chatInputRef = ref<InstanceType<typeof ChatInput>>()

  // 拖拽调整功能状态
  const isDragging = ref(false)
  const isHovering = ref(false)

  // 计算属性
  const canSend = computed(() => {
    return messageInput.value.trim().length > 0 && aiChatStore.canSendMessage
  })

  // 方法
  const sendMessage = async () => {
    if (!canSend.value) return

    // 获取包含终端上下文的完整消息
    const fullMessage = chatInputRef.value?.getMessageWithTerminalContext() || messageInput.value.trim()
    messageInput.value = ''

    try {
      // 普通聊天模式
      await aiChatStore.sendMessage(fullMessage)
    } catch (error) {
      // silent error
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
      // ignore
    }
  }

  const createNewSession = () => {
    aiChatStore.createConversation()
  }

  const handleSwitchMode = async (mode: 'chat' | 'agent') => {
    aiChatStore.chatMode = mode
    // 无论哪种模式都通过eko处理，确保eko已初始化
    await aiChatStore.initializeEko()
    // 同步模式到 Eko，确保工具权限生效
    await aiChatStore.ekoInstance?.setMode(mode)
    // 状态变化会自动触发保存（通过 watch 监听器）
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
    aiChatStore.setSidebarWidth(250) // 重置为默认宽度
  }

  // ===== 模型选择器相关 =====
  const selectedModelId = ref<string | null>(null)

  // 计算模型选项
  const modelOptions = computed(() => {
    return aiSettingsStore.enabledModels.map(model => ({
      label: model.name,
      value: model.id,
    }))
  })

  // 处理模型切换
  const handleModelChange = (modelId: string | null) => {
    selectedModelId.value = modelId
    // 保存到会话状态中，利用现有的缓存系统
    sessionStore.updateAiState({ selectedModelId: modelId })
  }

  // ===== 响应式数据 =====

  // ===== 方法 =====
  /**
   * 停止流式消息接收
   */
  const stopMessage = () => {
    if (aiChatStore.isLoading) {
      if (aiChatStore.cancelFunction) {
        aiChatStore.cancelFunction()
        aiChatStore.cancelFunction = null
      }
      aiChatStore.isLoading = false
    }
  }

  // 监听模型列表变化，自动选择合适的模型
  watch(
    () => aiSettingsStore.enabledModels,
    newModels => {
      if (newModels.length === 0) return

      // 优先使用会话状态中保存的模型，如果不存在则使用第一个
      const targetModelId = sessionStore.sessionState.ai.selectedModelId
      const validModel = newModels.find(m => m.id === targetModelId) || newModels[0]

      if (selectedModelId.value !== validModel.id) {
        selectedModelId.value = validModel.id
        sessionStore.updateAiState({ selectedModelId: validModel.id })
      }
    },
    { immediate: true }
  )

  // 生命周期
  onMounted(async () => {
    // 确保AI设置已加载
    if (!aiSettingsStore.isInitialized) {
      await aiSettingsStore.loadSettings()
    }

    // 初始化 OrbitX Store（恢复状态）
    if (!aiChatStore.isInitialized) {
      await aiChatStore.initialize()
    }
  })

  onUnmounted(() => {
    // 新系统自动保存，不需要手动保存
  })
</script>

<template>
  <div v-if="aiChatStore.isVisible" class="ai-chat-sidebar" :style="{ width: `${aiChatStore.sidebarWidth}px` }">
    <!-- 拖拽调整区域 -->
    <ResizeHandle
      :is-dragging="isDragging"
      :is-hovering="isHovering"
      @mousedown="startDrag"
      @mouseenter="onMouseEnter"
      @mouseleave="onMouseLeave"
      @dblclick="onDoubleClick"
    />

    <!-- 头部 -->
    <ChatHeader
      :sessions="aiChatStore.conversations"
      :current-session-id="aiChatStore.currentConversationId"
      :is-loading="aiChatStore.isLoading"
      @select-session="selectSession"
      @create-new-session="createNewSession"
      @delete-session="deleteSession"
      @refresh-sessions="refreshSessions"
    />

    <!-- 消息区域 -->
    <MessageList :messages="aiChatStore.messageList" />

    <!-- 输入区域 -->
    <ChatInput
      ref="chatInputRef"
      v-model="messageInput"
      :loading="aiChatStore.isLoading"
      :can-send="canSend"
      :selected-model="selectedModelId"
      :model-options="modelOptions"
      :chat-mode="aiChatStore.chatMode"
      :placeholder="t('session.chat_placeholder')"
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

  /* 确保拖拽区域不影响内容布局 */
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
    border-radius: 4px;
    border: 1px solid var(--border-300);
    font-size: 12px;
    z-index: 100;
    white-space: nowrap;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  }
</style>
