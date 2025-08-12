<template>
  <button
    :class="buttonClasses"
    :disabled="disabled || loading"
    :aria-disabled="disabled || loading"
    :aria-label="ariaLabel"
    :type="type"
    @click="handleClick"
    @focus="handleFocus"
    @blur="handleBlur"
  >
    <!-- 加载状态图标 -->
    <span v-if="loading" class="x-button__loading">
      <svg class="x-button__loading-icon" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2" fill="none" opacity="0.25" />
        <path d="M12 2a10 10 0 0 1 10 10" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" />
      </svg>
    </span>

    <!-- 左侧图标 -->
    <span v-if="showLeftIcon" class="x-button__icon x-button__icon--left">
      <slot name="icon">
        <svg v-if="icon" class="x-button__icon-svg" viewBox="0 0 24 24">
          <use :href="`#${icon}`"></use>
        </svg>
      </slot>
    </span>

    <!-- 按钮内容 -->
    <span v-if="!circle && $slots.default" class="x-button__content">
      <slot></slot>
    </span>

    <!-- 右侧图标 -->
    <span v-if="showRightIcon" class="x-button__icon x-button__icon--right">
      <slot name="icon">
        <svg v-if="icon" class="x-button__icon-svg" viewBox="0 0 24 24">
          <use :href="`#${icon}`"></use>
        </svg>
      </slot>
    </span>
  </button>
</template>

<script setup lang="ts">
  import { computed, inject, useSlots } from 'vue'
  import type { ButtonProps } from '../types/index'

  const props = withDefaults(defineProps<ButtonProps>(), {
    variant: 'primary',
    size: 'medium',
    disabled: false,
    loading: false,
    type: 'button',
    iconPosition: 'left',
    block: false,
    round: false,
    circle: false,
  })

  const emit = defineEmits<{
    click: [event: MouseEvent]
  }>()

  // 获取插槽
  const slots = useSlots()

  // 注入全局配置
  inject('xui-config', {})

  // 计算按钮类名
  const buttonClasses = computed(() => [
    'x-button',
    `x-button--${props.variant}`,
    `x-button--${props.size}`,
    {
      'x-button--loading': props.loading,
      'x-button--disabled': props.disabled,
      'x-button--block': props.block,
      'x-button--round': props.round,
      'x-button--circle': props.circle,
      'x-button--icon-only': props.circle || (!slots.default && (props.icon || slots.icon)),
    },
  ])

  // 计算是否显示左侧图标
  const showLeftIcon = computed(() => {
    return !props.loading && (props.icon || slots.icon) && props.iconPosition === 'left'
  })

  // 计算是否显示右侧图标
  const showRightIcon = computed(() => {
    return !props.loading && (props.icon || slots.icon) && props.iconPosition === 'right'
  })

  // 计算aria-label
  const ariaLabel = computed(() => {
    if (props.loading) {
      return '加载中'
    }
    return undefined
  })

  // 处理点击事件
  const handleClick = (event: MouseEvent) => {
    if (props.disabled || props.loading) {
      event.preventDefault()
      return
    }
    emit('click', event)
  }

  // 处理焦点事件
  const handleFocus = (_event: FocusEvent) => {
    // 可以在这里添加焦点处理逻辑
  }

  const handleBlur = (_event: FocusEvent) => {
    // 可以在这里添加失焦处理逻辑
  }
</script>

<style scoped>
  /* 基础按钮样式 - 使用全局主题变量 */
  .x-button {
    /* 基础变量 */
    --x-button-font-weight: 400;
    --x-button-border-width: 1px;
    --x-button-border-style: solid;
    --x-button-border-radius: var(--border-radius);
    --x-button-transition: all 0.2s cubic-bezier(0.645, 0.045, 0.355, 1);

    /* 尺寸变量 */
    --x-button-height-small: 24px;
    --x-button-padding-small: 0 var(--spacing-sm);
    --x-button-font-size-small: var(--font-size-xs);

    --x-button-height-medium: 32px;
    --x-button-padding-medium: 0 var(--spacing-lg);
    --x-button-font-size-medium: var(--font-size-md);

    --x-button-height-large: 40px;
    --x-button-padding-large: 0 var(--spacing-xl);
    --x-button-font-size-large: var(--font-size-lg);
  }

  /* 基础按钮样式 */
  .x-button {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--spacing-xs);
    height: var(--x-button-height-medium);
    padding: var(--x-button-padding-medium);
    font-size: var(--x-button-font-size-medium);
    font-family: var(--font-family);
    font-weight: var(--x-button-font-weight);
    line-height: 1.5715;
    white-space: nowrap;
    text-align: center;
    background: var(--color-background);
    border: var(--x-button-border-width) var(--x-button-border-style) var(--border-color);
    border-radius: var(--x-button-border-radius);
    color: var(--text-primary);
    cursor: pointer;
    transition: var(--x-button-transition);
    user-select: none;
    touch-action: manipulation;
    outline: none;
  }

  .x-button:hover {
    color: var(--color-primary);
    border-color: var(--color-primary);
    background: var(--color-background-hover);
  }

  .x-button:focus {
    color: var(--color-primary);
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px var(--color-primary-alpha);
  }

  .x-button:active {
    color: var(--color-primary);
    border-color: var(--color-primary);
    background: var(--color-background-hover);
  }

  /* 尺寸变体 */
  .x-button--small {
    height: var(--x-button-height-small);
    padding: var(--x-button-padding-small);
    font-size: var(--x-button-font-size-small);
    border-radius: var(--border-radius-sm);
  }

  .x-button--large {
    height: var(--x-button-height-large);
    padding: var(--x-button-padding-large);
    font-size: var(--x-button-font-size-large);
    border-radius: var(--border-radius-lg);
  }

  /* 主要按钮 */
  .x-button--primary {
    color: #fff;
    background: var(--color-primary);
    border-color: var(--color-primary);
  }

  .x-button--primary:hover {
    color: #fff;
    background: var(--color-primary);
    border-color: var(--color-primary);
    opacity: 0.8;
  }

  .x-button--primary:focus {
    color: #fff;
    background: var(--color-primary);
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px var(--color-primary-alpha);
  }

  .x-button--primary:active {
    color: #fff;
    background: var(--color-primary);
    border-color: var(--color-primary);
    opacity: 0.9;
  }

  /* 次要按钮 */
  .x-button--secondary {
    color: var(--text-primary);
    background: var(--color-background-secondary);
    border-color: var(--border-color);
  }

  .x-button--secondary:hover {
    background: var(--color-background-hover);
    border-color: var(--border-color-hover);
  }

  /* 危险按钮 */
  .x-button--danger {
    color: #fff;
    background: var(--color-red);
    border-color: var(--color-red);
  }

  .x-button--danger:hover {
    color: #fff;
    background: var(--color-red);
    border-color: var(--color-red);
    opacity: 0.8;
  }

  .x-button--danger:focus {
    color: #fff;
    background: var(--color-red);
    border-color: var(--color-red);
    box-shadow: 0 0 0 2px rgba(244, 71, 71, 0.2);
  }

  .x-button--danger:active {
    color: #fff;
    background: var(--color-red);
    border-color: var(--color-red);
    opacity: 0.9;
  }

  /* 幽灵按钮 */
  .x-button--ghost {
    color: var(--text-primary);
    background: transparent;
    border-color: var(--border-color);
  }

  .x-button--ghost:hover {
    color: var(--color-primary);
    border-color: var(--color-primary);
    background: var(--color-primary-alpha);
  }

  /* 链接按钮 */
  .x-button--link {
    color: var(--color-primary);
    background: transparent;
    border-color: transparent;
    box-shadow: none;
  }

  .x-button--link:hover {
    color: var(--color-primary);
    background: transparent;
    border-color: transparent;
    opacity: 0.8;
  }

  .x-button--link:focus {
    color: var(--color-primary);
    background: transparent;
    border-color: transparent;
    box-shadow: none;
  }

  .x-button--link:active {
    color: var(--color-primary);
    opacity: 0.9;
  }

  /* 禁用状态 */
  .x-button--disabled,
  .x-button:disabled {
    color: var(--text-muted) !important;
    background: var(--color-background-secondary) !important;
    border-color: var(--border-color) !important;
    cursor: not-allowed !important;
    box-shadow: none !important;
    opacity: 0.6 !important;
  }

  .x-button--link.x-button--disabled,
  .x-button--link:disabled {
    color: var(--text-muted) !important;
    background: transparent !important;
    border-color: transparent !important;
  }

  /* 加载状态 */
  .x-button--loading {
    position: relative;
    pointer-events: none;
  }

  .x-button--loading .x-button__content {
    opacity: 0.6;
  }

  /* 块级按钮 */
  .x-button--block {
    width: 100%;
  }

  /* 圆角按钮 */
  .x-button--round {
    border-radius: 32px;
  }

  .x-button--round.x-button--small {
    border-radius: 24px;
  }

  .x-button--round.x-button--large {
    border-radius: 40px;
  }

  /* 圆形按钮 */
  .x-button--circle {
    min-width: var(--x-button-height-medium);
    padding: 0;
    border-radius: 50%;
  }

  .x-button--circle.x-button--small {
    min-width: var(--x-button-height-small);
  }

  .x-button--circle.x-button--large {
    min-width: var(--x-button-height-large);
  }

  /* 仅图标按钮 */
  .x-button--icon-only {
    padding: 0;
    min-width: var(--x-button-height-medium);
  }

  .x-button--icon-only.x-button--small {
    min-width: var(--x-button-height-small);
  }

  .x-button--icon-only.x-button--large {
    min-width: var(--x-button-height-large);
  }

  /* 加载图标 */
  .x-button__loading {
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .x-button__loading-icon {
    width: 14px;
    height: 14px;
    animation: x-button-spin 1s linear infinite;
  }

  .x-button--small .x-button__loading-icon {
    width: 12px;
    height: 12px;
  }

  .x-button--large .x-button__loading-icon {
    width: 16px;
    height: 16px;
  }

  /* 图标 */
  .x-button__icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .x-button__icon-svg {
    width: 14px;
    height: 14px;
    fill: currentColor;
  }

  .x-button--small .x-button__icon-svg {
    width: 12px;
    height: 12px;
  }

  .x-button--large .x-button__icon-svg {
    width: 16px;
    height: 16px;
  }

  /* 内容 */
  .x-button__content {
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  /* 动画 */
  @keyframes x-button-spin {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }

  /* 响应式设计 */
  @media (max-width: 768px) {
    .x-button--block {
      width: 100%;
    }
  }

  /* 高对比度模式支持 */
  @media (prefers-contrast: high) {
    .x-button {
      border-width: 2px;
    }
  }

  /* 减少动画模式支持 */
  @media (prefers-reduced-motion: reduce) {
    .x-button {
      transition: none;
    }

    .x-button__loading-icon {
      animation: none;
    }
  }

  /* 深色模式支持 */
  @media (prefers-color-scheme: dark) {
    .x-button {
      --x-color-text: rgba(255, 255, 255, 0.85);
      --x-color-text-secondary: rgba(255, 255, 255, 0.45);
      --x-color-border: #434343;
      --x-color-background: #141414;
      --x-color-background-hover: #262626;
      --x-color-disabled: #262626;
      --x-color-disabled-text: rgba(255, 255, 255, 0.25);
    }
  }
</style>
