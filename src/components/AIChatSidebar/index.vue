<script setup lang="ts">
  import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
  import { useAIChatStore } from './store'
  import { useAISettingsStore } from '@/components/settings/components/AI'
  import ChatHeader from './components/ChatHeader.vue'

  import MessageList from './components/MessageList.vue'
  import ChatInput from './components/ChatInput.vue'
  import ResizeHandle from './components/ResizeHandle.vue'

  // 状态管理
  const aiChatStore = useAIChatStore() // AI聊天状态管理
  const aiSettingsStore = useAISettingsStore() // AI设置状态管理

  // 本地状态
  const messageInput = ref('')

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

    const message = messageInput.value.trim()
    messageInput.value = ''

    try {
      await aiChatStore.sendMessage(message)
    } catch (error) {
      console.error('发送消息失败:', error)
    }
  }

  const selectSession = (sessionId: string) => {
    aiChatStore.loadSession(sessionId)
  }

  const deleteSession = (sessionId: string) => {
    aiChatStore.deleteSession(sessionId)
  }

  const refreshSessions = async () => {
    try {
      await aiChatStore.refreshSessions()
    } catch (error) {
      console.error('刷新会话列表失败:', error)
    }
  }

  const createNewSession = () => {
    aiChatStore.createNewSession()
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
        aiChatStore.hideSidebar()
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
  const selectedModelId = ref<string | null>(aiSettingsStore.defaultModel?.id || null)

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
    if (modelId) {
      aiSettingsStore.setDefaultModel(modelId)
    }
  }

  // ===== 响应式数据 =====
  const showMessage = ref(false) // 是否显示消息提示
  const messageText = ref('') // 消息提示内容
  const messageType = ref<'success' | 'error' | 'warning' | 'info'>('success') // 消息提示类型
  const messageListRef = ref() // MessageList组件引用

  // ===== 方法 =====
  /**
   * 停止流式消息接收
   */
  const stopMessage = () => {
    aiChatStore.stopStreaming()
  }

  // 滚动到底部
  const scrollToBottom = () => {
    if (messageListRef.value) {
      messageListRef.value.scrollToBottom()
    }
  }

  // 监听消息变化，自动滚动到底部
  watch(
    () => aiChatStore.messages,
    () => {
      // 使用 nextTick 确保 DOM 更新后再滚动
      nextTick(() => {
        scrollToBottom()
      })
    },
    { deep: true }
  )

  // 监听流式状态变化，确保流式过程中也能滚动
  watch(
    () => aiChatStore.isStreaming,
    isStreaming => {
      if (isStreaming) {
        // 流式开始时滚动到底部
        nextTick(() => {
          scrollToBottom()
        })
      }
    }
  )

  // 监听流式内容变化，实时滚动
  watch(
    () => aiChatStore.streamingContent,
    () => {
      if (aiChatStore.isStreaming) {
        // 流式过程中实时滚动
        nextTick(() => {
          scrollToBottom()
        })
      }
    }
  )

  // 监听默认模型变化，同步选中状态
  watch(
    () => aiSettingsStore.defaultModel?.id,
    newModelId => {
      selectedModelId.value = newModelId || null
    }
  )

  // 生命周期
  onMounted(async () => {
    // 确保AI设置已加载
    if (!aiSettingsStore.isInitialized) {
      await aiSettingsStore.loadSettings()
    }

    aiChatStore.initialize()
    scrollToBottom()
  })

  onUnmounted(() => {
    aiChatStore.saveCurrentSession()
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
      :sessions="aiChatStore.sessions"
      :current-session-id="aiChatStore.currentSessionId"
      :is-loading="aiChatStore.isLoading"
      @select-session="selectSession"
      @create-new-session="createNewSession"
      @delete-session="deleteSession"
      @refresh-sessions="refreshSessions"
    />

    <!-- 消息列表区域 -->
    <MessageList
      ref="messageListRef"
      :messages="aiChatStore.messages"
      :has-messages="aiChatStore.hasMessages"
      :is-streaming="aiChatStore.isStreaming"
      :empty-state-description="
        aiSettingsStore.defaultModel ? `使用 ${aiSettingsStore.defaultModel.name} 模型` : '请先配置AI模型'
      "
    />

    <!-- 输入区域 -->
    <ChatInput
      v-model="messageInput"
      :loading="aiChatStore.isLoading"
      :is-streaming="aiChatStore.isStreaming"
      :can-send="canSend"
      :selected-model="selectedModelId"
      :model-options="modelOptions"
      @send="sendMessage"
      @stop="stopMessage"
      @model-change="handleModelChange"
    />

    <!-- 消息提示 -->
    <x-message :visible="showMessage" :message="messageText" :type="messageType" @close="showMessage = false" />
  </div>
</template>

<style scoped>
  .ai-chat-sidebar {
    height: 100%;
    background-color: var(--color-ai-sidebar-background);
    border-left: 1px solid var(--color-border);
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
    background: var(--color-background);
    color: var(--text-primary);
    padding: 8px 16px;
    border-radius: 4px;
    border: 1px solid var(--color-border);
    font-size: 12px;
    z-index: 100;
    white-space: nowrap;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  }
</style>
