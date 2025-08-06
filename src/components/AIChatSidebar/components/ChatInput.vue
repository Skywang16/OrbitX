<script setup lang="ts">
  import { computed, ref } from 'vue'

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
    placeholder: '输入消息...',
    loading: false,

    canSend: false,
    selectedModel: null,
    modelOptions: () => [],
    chatMode: 'chat',
  })

  const emit = defineEmits<Emits>()

  // 响应式引用
  const inputTextarea = ref<HTMLTextAreaElement>()

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
      description: '普通聊天模式',
    },
    {
      label: 'Agent',
      value: 'agent',
      description: 'Agent智能助手模式',
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
    const maxHeight = 120
    const minHeight = 44
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
    console.log('🔄 [ChatInput] 模式切换事件触发:', value)
    const mode = value as 'chat' | 'agent'
    if (mode && (mode === 'chat' || mode === 'agent')) {
      console.log('✅ [ChatInput] 发送模式切换事件:', mode)
      emit('mode-change', mode)
    } else {
      console.log('❌ [ChatInput] 无效的模式值:', value)
    }
  }

  // 暴露方法给父组件
  defineExpose({
    adjustTextareaHeight,
    focus: () => inputTextarea.value?.focus(),
  })
</script>

<template>
  <div class="chat-input">
    <!-- 主输入区域 -->
    <div class="input-main">
      <div class="input-content">
        <textarea
          ref="inputTextarea"
          v-model="inputValue"
          class="message-input"
          :placeholder="placeholder"
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
          :disabled="loading ? false : !canSend"
          :loading="loading"
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
          placeholder="选择模式"
          size="small"
          borderless
          @update:model-value="handleModeChange"
        />
        <x-select
          class="model-selector"
          :model-value="selectedModel"
          :options="modelOptions"
          placeholder="选择AI模型"
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
    border: 1px solid var(--color-border);
    border-radius: 8px;
    background-color: var(--color-background);
    transition: border-color 0.1s ease;
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
    min-height: 44px;
  }
  .message-input {
    width: 100%;
    min-height: 44px;
    max-height: 150px;
    border: none;
    background: transparent;
    color: var(--color-text);
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
    color: var(--color-text-secondary);
    opacity: 0.6;
  }

  .send-button {
    width: 24px;
    height: 24px;
    flex-shrink: 0; /* 防止按钮被压缩 */
  }

  .stop-button {
    background-color: #ff4d4f !important;
  }

  .stop-button:hover:not(:disabled) {
    background-color: #ff7875 !important;
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
    width: 80px;
  }

  .model-selector {
    width: 110px;
  }

  /* 智能体模式开关样式 */
  .agent-mode-toggle {
    display: flex;
    align-items: center;
  }

  .toggle-label {
    display: flex;
    align-items: center;
    cursor: pointer;
    user-select: none;
    font-size: 12px;
    color: var(--color-text-secondary);
    gap: 6px;
  }

  .toggle-checkbox {
    display: none;
  }

  .toggle-slider {
    position: relative;
    width: 32px;
    height: 18px;
    background-color: var(--color-border);
    border-radius: 9px;
    transition: background-color 0.2s ease;
  }

  .toggle-slider::before {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    background-color: white;
    border-radius: 50%;
    transition: transform 0.2s ease;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
  }

  .toggle-checkbox:checked + .toggle-slider {
    background-color: var(--color-primary);
  }

  .toggle-checkbox:checked + .toggle-slider::before {
    transform: translateX(14px);
  }

  .toggle-label:hover .toggle-slider {
    background-color: var(--color-primary-hover);
  }

  .toggle-checkbox:checked + .toggle-slider:hover {
    background-color: var(--color-primary-active);
  }

  .toggle-text {
    font-weight: 500;
    white-space: nowrap;
  }

  .toggle-checkbox:checked ~ .toggle-text {
    color: var(--color-primary);
  }
</style>
