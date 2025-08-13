<template>
  <div class="search-input">
    <div class="search-icon">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="10" cy="10" r="6" />
        <path d="m20 20-6-6" />
      </svg>
    </div>
    <input
      ref="inputRef"
      v-model="inputValue"
      type="text"
      :placeholder="placeholder"
      class="search-field"
      @input="handleInput"
      @focus="handleFocus"
      @blur="handleBlur"
      @keydown.enter="handleEnter"
      @keydown.escape="handleEscape"
    />
    <button v-if="inputValue && clearable" class="clear-button" @click="handleClear" title="清除">
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="18" y1="6" x2="6" y2="18" />
        <line x1="6" y1="6" x2="18" y2="18" />
      </svg>
    </button>
  </div>
</template>

<script setup lang="ts">
  import { ref, watch } from 'vue'
  import { debounce } from 'lodash-es'

  interface Props {
    modelValue?: string
    placeholder?: string
    clearable?: boolean
    autofocus?: boolean
    debounce?: number
  }

  interface Emits {
    (e: 'update:modelValue', value: string): void
    (e: 'search', value: string): void
    (e: 'focus', event: FocusEvent): void
    (e: 'blur', event: FocusEvent): void
    (e: 'clear'): void
  }

  const props = withDefaults(defineProps<Props>(), {
    modelValue: '',
    placeholder: '搜索',
    clearable: true,
    autofocus: false,
    debounce: 300,
  })

  const emit = defineEmits<Emits>()

  const inputRef = ref<HTMLInputElement>()
  const inputValue = ref(props.modelValue)

  // 使用lodash防抖搜索
  const debouncedSearch = debounce((value: string) => {
    emit('search', value)
  }, props.debounce)

  // 监听外部值变化
  watch(
    () => props.modelValue,
    newValue => {
      inputValue.value = newValue
    }
  )

  // 处理输入
  const handleInput = () => {
    emit('update:modelValue', inputValue.value)
    debouncedSearch(inputValue.value)
  }

  // 处理焦点
  const handleFocus = (event: FocusEvent) => {
    emit('focus', event)
  }

  const handleBlur = (event: FocusEvent) => {
    emit('blur', event)
  }

  // 处理回车
  const handleEnter = () => {
    debouncedSearch.cancel() // 取消防抖，立即搜索
    emit('search', inputValue.value)
  }

  // 处理ESC
  const handleEscape = () => {
    if (inputValue.value) {
      handleClear()
    }
  }

  // 清除输入
  const handleClear = () => {
    debouncedSearch.cancel() // 取消防抖
    inputValue.value = ''
    emit('update:modelValue', '')
    emit('search', '')
    emit('clear')
    inputRef.value?.focus()
  }

  // 聚焦方法
  const focus = () => {
    inputRef.value?.focus()
  }

  // 暴露方法
  defineExpose({
    focus,
  })
</script>

<style scoped>
  /* 搜索输入框样式 - 使用全局主题变量 */
  .search-input {
    position: relative;
    display: flex;
    align-items: center;
    background-color: var(--bg-400);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius);
    transition: all 0.2s ease;
    height: 32px;
    font-family: var(--font-family);
  }

  .search-input:hover {
    border-color: var(--border-400);
  }

  /* 搜索图标 */
  .search-icon {
    position: relative;
    left: var(--spacing-sm);
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-400);
    flex-shrink: 0;
  }

  .search-icon svg {
    width: 14px;
    height: 14px;
  }

  /* 输入框 */
  .search-field {
    width: 100%;
    padding: 0 var(--spacing-xl) 0 var(--spacing-md);
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-200);
    font-size: var(--font-size-md);
    font-family: var(--font-family);
    line-height: 1.5;
  }

  .search-field::placeholder {
    color: var(--text-400);
  }

  /* 清除按钮 */
  .clear-button {
    position: absolute;
    right: var(--spacing-sm);
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-sm);
    color: var(--text-400);
    cursor: pointer;
    transition: all 0.2s ease;
    flex-shrink: 0;
  }

  .clear-button:hover {
    background-color: var(--color-hover);
    color: var(--text-200);
  }

  .clear-button svg {
    width: 12px;
    height: 12px;
  }

  /* 尺寸变体 */
  .search-input--small {
    height: 24px;
  }

  .search-input--small .search-field {
    font-size: var(--font-size-xs);
  }

  .search-input--large {
    height: 40px;
  }

  .search-input--large .search-field {
    font-size: var(--font-size-lg);
  }

  /* 禁用状态 */
  .search-input--disabled {
    background-color: var(--bg-500);
    border-color: var(--border-300);
    cursor: not-allowed;
    opacity: 0.6;
  }

  .search-input--disabled .search-field {
    cursor: not-allowed;
  }

  .search-input--disabled .clear-button {
    cursor: not-allowed;
    pointer-events: none;
  }

  /* 响应式设计 */
  @media (max-width: 768px) {
    .search-input {
      height: 36px;
    }

    .search-field {
      font-size: var(--font-size-md);
    }
  }

  /* 减少动画模式支持 */
  @media (prefers-reduced-motion: reduce) {
    .search-input,
    .clear-button {
      transition: none;
    }
  }
</style>
