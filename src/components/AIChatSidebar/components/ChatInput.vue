<script setup lang="ts">
  import { computed, ref, nextTick, watch } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useTerminalSelection } from '@/composables/useTerminalSelection'
  import { useVectorIndexStatus } from '@/composables/useVectorIndexStatus'
  import { useVectorIndexBuild } from '@/composables/useVectorIndexBuild'
  import TerminalSelectionTag from './TerminalSelectionTag.vue'
  import TerminalTabTag from './TerminalTabTag.vue'

  // Props定义
  interface Props {
    modelValue: string
    placeholder?: string
    loading?: boolean

    canSend?: boolean
    selectedModel?: string | null
    modelOptions?: Array<{ label: string; value: string | number }>
    chatMode?: 'chat' | 'agent'
  }

  // Emits定义
  interface Emits {
    (e: 'update:modelValue', value: string): void
    (e: 'send'): void
    (e: 'stop'): void
    (e: 'update:selectedModel', value: string | null): void
    (e: 'model-change', value: string | null): void
    (e: 'mode-change', mode: 'chat' | 'agent'): void
  }

  const props = withDefaults(defineProps<Props>(), {
    placeholder: '',
    loading: false,

    canSend: false,
    selectedModel: null,
    modelOptions: () => [],
    chatMode: 'chat',
  })

  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  // 响应式引用
  const inputTextarea = ref<HTMLTextAreaElement>()

  // 终端选择管理
  const terminalSelection = useTerminalSelection()

  // 向量索引状态管理
  const vectorIndexStatus = useVectorIndexStatus()

  // 构建索引状态（集中管理）
  const { isBuilding, progress } = useVectorIndexBuild()

  // 计算属性
  const inputValue = computed({
    get: () => props.modelValue,
    set: (value: string) => emit('update:modelValue', value),
  })

  // 模式选项数据
  const modeOptions = computed(() => [
    {
      label: 'Chat',
      value: 'chat',
    },
    {
      label: 'Agent',
      value: 'agent',
    },
  ])

  // 方法
  /**
   * 处理键盘事件
   */
  const handleKeydown = (event: KeyboardEvent) => {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault()
      handleButtonClick()
    }
  }

  /**
   * 调整输入框高度
   */
  const adjustTextareaHeight = () => {
    if (!inputTextarea.value) return

    const textarea = inputTextarea.value
    textarea.style.height = 'auto'

    const scrollHeight = textarea.scrollHeight
    const maxHeight = 100
    const minHeight = 32
    const newHeight = Math.max(minHeight, Math.min(scrollHeight, maxHeight))

    textarea.style.height = newHeight + 'px'
    textarea.style.overflowY = scrollHeight > maxHeight ? 'auto' : 'hidden'
  }

  /**
   * 处理发送/停止按钮点击
   */
  const handleButtonClick = () => {
    if (props.loading) {
      emit('stop')
    } else if (props.canSend) {
      // 发送包含终端上下文的完整消息
      emit('send')
    }
  }

  /**
   * 处理模型选择变化
   */
  const handleModelChange = (value: string | number | null) => {
    const modelId = typeof value === 'string' ? value : null
    emit('update:selectedModel', modelId)
    emit('model-change', modelId)
  }

  /**
   * 处理模式切换
   */
  const handleModeChange = (value: string | number | null) => {
    const mode = value as 'chat' | 'agent'
    if (mode === 'chat' || mode === 'agent') {
      emit('mode-change', mode)
    }
  }

  /**
   * 处理插入选定文本 - 优化逻辑
   */
  const handleInsertSelectedText = () => {
    const selectedText = terminalSelection.getSelectedText()
    if (!selectedText.trim()) return

    // 智能拼接：有内容时添加空格分隔
    const newValue = props.modelValue ? `${props.modelValue} ${selectedText}` : selectedText

    emit('update:modelValue', newValue)

    // 异步聚焦和调整高度
    nextTick(() => {
      inputTextarea.value?.focus()
      adjustTextareaHeight()
    })
  }

  /**
   * 获取标签上下文信息（用于传递给后端）
   */
  const getTagContextInfo = () => {
    return terminalSelection.getTagContextInfo()
  }

  // 当终端标签改变时检查向量索引状态
  watch(
    () => vectorIndexStatus.currentDirectory.value,
    () => {
      vectorIndexStatus.checkCurrentDirectoryIndex()
    },
    { immediate: true }
  )

  // 暴露方法给父组件
  defineExpose({
    adjustTextareaHeight,
    focus: () => inputTextarea.value?.focus(),
    getTagContextInfo,
  })

  // 去除本组件的事件监听，统一在可复用的 composable 内管理
</script>

<template>
  <div class="chat-input">
    <!-- 索引构建进度条，显示在输入框上方 -->
    <div v-if="isBuilding" class="progress-container">
      <div class="progress-bar">
        <div class="progress-fill" :style="{ width: progress + '%' }"></div>
      </div>
      <div class="progress-text">{{ progress }}%</div>
    </div>
    <!-- 终端标签页标签 -->
    <TerminalTabTag
      :visible="terminalSelection.hasTerminalTab.value"
      :terminal-id="terminalSelection.currentTerminalTab.value?.terminalId"
      :shell="terminalSelection.currentTerminalTab.value?.shell"
      :cwd="terminalSelection.currentTerminalTab.value?.cwd"
      :display-path="terminalSelection.currentTerminalTab.value?.displayPath"
    />

    <!-- 终端选择标签 -->
    <TerminalSelectionTag
      :visible="terminalSelection.hasSelection.value"
      :selected-text="terminalSelection.selectedText.value"
      :selection-info="terminalSelection.selectionInfo.value"
      @clear="terminalSelection.clearSelection"
      @insert="handleInsertSelectedText"
    />
    <!-- 移除错误的自定义向量索引标签，统一由进度与功能开关控制显示 -->

    <!-- 主输入区域 -->
    <div class="input-main">
      <div class="input-content">
        <textarea
          ref="inputTextarea"
          v-model="inputValue"
          class="message-input"
          :placeholder="placeholder || t('chat.input_placeholder')"
          rows="1"
          @keydown="handleKeydown"
          @input="adjustTextareaHeight"
        />
      </div>
      <div class="button-container">
        <x-button
          :variant="loading ? 'danger' : 'primary'"
          size="small"
          circle
          class="send-button"
          :class="{ 'stop-button': loading }"
          :disabled="!loading && !canSend"
          @click="handleButtonClick"
        >
          <template #icon>
            <svg v-if="loading" width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <rect x="6" y="6" width="12" height="12" rx="2" />
            </svg>
            <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="22" y1="2" x2="11" y2="13" />
              <polygon points="22,2 15,22 11,13 2,9" />
            </svg>
          </template>
        </x-button>
      </div>
    </div>

    <!-- 模型选择器和模式切换 -->
    <div class="input-bottom">
      <div class="bottom-left">
        <x-select
          class="mode-selector"
          :model-value="chatMode"
          :options="modeOptions"
          :placeholder="t('ai.select_mode')"
          size="small"
          borderless
          @update:model-value="handleModeChange"
        />
        <x-select
          class="model-selector"
          :model-value="selectedModel"
          :options="modelOptions"
          :placeholder="t('ai.select_model')"
          size="small"
          borderless
          @update:model-value="handleModelChange"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
  .chat-input {
    padding: 10px;
    margin: auto;
    width: 90%;
    margin-bottom: 10px;
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius-lg);
    background-color: var(--bg-400);
    transition: border-color 0.1s ease;
  }

  .progress-container {
    margin-bottom: 8px;
  }
  .progress-bar {
    width: 100%;
    height: 6px;
    background: var(--bg-500);
    border-radius: var(--border-radius-sm);
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--color-primary);
    transition: width 0.2s ease;
  }

  .progress-text {
    margin-top: 4px;
    font-size: 12px;
    color: var(--text-400);
    text-align: right;
  }

  .chat-input:hover {
    border-color: var(--color-primary);
  }

  .input-main {
    display: flex;
    align-items: flex-end;
    gap: 2px; /* 输入框和按钮之间的间距 */
  }

  .input-content {
    flex: 1;
    min-height: 32px;
  }
  .message-input {
    width: 100%;
    min-height: 32px;
    max-height: 100px;
    border: none;
    background: transparent;
    color: var(--text-300);
    font-size: 14px;
    outline: none;
    resize: none;
  }

  .button-container {
    display: flex;
    align-items: center;
    padding-bottom: 8px;
  }

  .message-input::-webkit-scrollbar {
    display: none;
  }

  .message-input::placeholder {
    color: var(--text-400);
    opacity: 0.6;
  }

  .send-button {
    width: 24px;
    height: 24px;
    flex-shrink: 0; /* 防止按钮被压缩 */
  }

  .stop-button {
    background-color: var(--color-error) !important;
  }

  .stop-button:hover:not(:disabled) {
    background-color: var(--color-error) !important;
    opacity: 0.8;
  }

  .input-bottom {
    margin-top: 8px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }

  .bottom-left {
    flex: 1;
    display: flex;
    gap: 8px;
  }

  .bottom-right {
    flex-shrink: 0;
  }

  .mode-selector {
    width: 100px;
  }

  .model-selector {
    width: 110px;
  }
</style>
