<template>
  <label
    class="base-switch"
    :class="{ 'is-disabled': disabled || loading, 'is-checked': modelValue }"
    role="switch"
    :aria-checked="modelValue"
    :aria-disabled="disabled || loading"
  >
    <input type="checkbox" :checked="modelValue" :disabled="disabled || loading" @change="handleChange" />
    <span class="slider">
      <span v-if="loading" class="loading-spinner"></span>
    </span>
  </label>
</template>

<script setup lang="ts">
  import { defineEmits, defineProps, withDefaults } from 'vue'

  const props = withDefaults(
    defineProps<{
      modelValue: boolean
      disabled?: boolean
      loading?: boolean
    }>(),
    {
      disabled: false,
      loading: false,
    }
  )

  const emit = defineEmits(['update:modelValue'])

  const handleChange = (event: Event) => {
    if (props.disabled || props.loading) return
    const target = event.target as HTMLInputElement
    emit('update:modelValue', target.checked)
  }
</script>

<style scoped>
  /* 开关组件样式 - 使用全局主题变量 */
  .base-switch {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 22px;
    cursor: pointer;
    font-family: var(--font-family);
  }

  .base-switch.is-disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  .base-switch input {
    opacity: 0;
    width: 0;
    height: 0;
    position: absolute;
  }

  /* 滑块背景 */
  .slider {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: var(--color-background-secondary);
    border: 1px solid var(--border-color);
    transition: all 0.3s cubic-bezier(0.645, 0.045, 0.355, 1);
    border-radius: 22px;
    display: flex;
    align-items: center;
  }

  /* 滑块按钮 */
  .slider:before {
    position: absolute;
    content: '';
    height: 18px;
    width: 18px;
    left: 2px;
    background-color: var(--color-background);
    border: 1px solid var(--border-color);
    transition: all 0.3s cubic-bezier(0.645, 0.045, 0.355, 1);
    border-radius: 50%;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  /* 选中状态 */
  input:checked + .slider {
    background-color: var(--color-primary);
    border-color: var(--color-primary);
  }

  input:checked + .slider:before {
    transform: translateX(22px);
    background-color: #fff;
    border-color: var(--color-primary);
  }

  /* 悬停状态 */
  .base-switch:hover:not(.is-disabled) .slider {
    border-color: var(--border-color-hover);
  }

  .base-switch:hover:not(.is-disabled) input:checked + .slider {
    background-color: var(--color-primary);
    opacity: 0.8;
  }

  /* 焦点状态 */
  .base-switch input:focus + .slider {
    box-shadow: 0 0 0 2px var(--color-primary-alpha);
  }

  /* 加载状态 */
  .loading-spinner {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 12px;
    height: 12px;
    border: 2px solid var(--text-muted);
    border-top-color: var(--text-secondary);
    border-radius: 50%;
    animation: x-switch-spin 1s linear infinite;
  }

  .is-checked .loading-spinner {
    border-color: rgba(255, 255, 255, 0.3);
    border-top-color: white;
  }

  /* 尺寸变体 */
  .base-switch--small {
    width: 32px;
    height: 16px;
  }

  .base-switch--small .slider:before {
    height: 12px;
    width: 12px;
    left: 2px;
  }

  .base-switch--small input:checked + .slider:before {
    transform: translateX(16px);
  }

  .base-switch--large {
    width: 56px;
    height: 28px;
  }

  .base-switch--large .slider {
    border-radius: 28px;
  }

  .base-switch--large .slider:before {
    height: 24px;
    width: 24px;
    left: 2px;
  }

  .base-switch--large input:checked + .slider:before {
    transform: translateX(28px);
  }

  /* 文本标签 */
  .switch-text {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    font-size: var(--font-size-xs);
    font-weight: 500;
    color: var(--text-primary);
    pointer-events: none;
    transition: opacity 0.3s ease;
  }

  .switch-text--checked {
    left: var(--spacing-sm);
    opacity: 0;
  }

  .switch-text--unchecked {
    right: var(--spacing-sm);
    opacity: 1;
  }

  input:checked ~ .switch-text--checked {
    opacity: 1;
    color: white;
  }

  input:checked ~ .switch-text--unchecked {
    opacity: 0;
  }

  /* 动画 */
  @keyframes x-switch-spin {
    0% {
      transform: translate(-50%, -50%) rotate(0deg);
    }
    100% {
      transform: translate(-50%, -50%) rotate(360deg);
    }
  }

  /* 减少动画模式支持 */
  @media (prefers-reduced-motion: reduce) {
    .slider,
    .slider:before {
      transition: none;
    }

    .loading-spinner {
      animation: none;
    }
  }

  /* 高对比度模式支持 */
  @media (prefers-contrast: high) {
    .slider {
      border-width: 2px;
    }

    .slider:before {
      border-width: 2px;
    }
  }
</style>
