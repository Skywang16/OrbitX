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
    const mode = value as 'chat' | 'agent'
    if (mode === 'chat' || mode === 'agent') {
      emit('mode-change', mode)
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
    border: 1px solid var(--border-300);
    border-radius: 8px;
    background-color: var(--bg-400);
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
