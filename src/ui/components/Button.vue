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
    <span v-if="loading" class="x-button__loading">
      <svg class="x-button__loading-icon" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2" fill="none" opacity="0.25" />
        <path d="M12 2a10 10 0 0 1 10 10" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" />
      </svg>
    </span>

    <span v-if="showLeftIcon" class="x-button__icon x-button__icon--left">
      <slot name="icon">
        <svg v-if="icon" class="x-button__icon-svg" viewBox="0 0 24 24">
          <use :href="`#${icon}`"></use>
        </svg>
      </slot>
    </span>

    <span v-if="!circle && $slots.default" class="x-button__content">
      <slot></slot>
    </span>

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

  const slots = useSlots()
  inject('xui-config', {})

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

  const showLeftIcon = computed(() => {
    return !props.loading && (props.icon || slots.icon) && props.iconPosition === 'left'
  })

  const showRightIcon = computed(() => {
    return !props.loading && (props.icon || slots.icon) && props.iconPosition === 'right'
  })

  const ariaLabel = computed(() => {
    if (props.loading) {
      return '加载中'
    }
    return undefined
  })

  const handleClick = (event: MouseEvent) => {
    if (props.disabled || props.loading) {
      event.preventDefault()
      return
    }
    emit('click', event)
  }

  const handleFocus = (_event: FocusEvent) => {}

  const handleBlur = (_event: FocusEvent) => {}
</script>

<style>
  .x-button {
    --x-button-font-weight: 500;
    --x-button-border-width: 1px;
    --x-button-border-style: solid;
    --x-button-border-radius: var(--border-radius-sm);
    --x-button-transition: all 0.12s ease-out;
    --x-button-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
    --x-button-shadow-hover: 0 2px 6px 0 rgba(0, 0, 0, 0.12);
    --x-button-shadow-active: 0 1px 2px 0 rgba(0, 0, 0, 0.08);

    --x-button-padding-small: 4px 6px;
    --x-button-font-size-small: 12px;

    --x-button-padding-medium: 6px 8px;
    --x-button-font-size-medium: 14px;

    --x-button-padding-large: 8px 10px;
    --x-button-font-size-large: 16px;
  }

  .x-button {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: var(--x-button-padding-medium);
    font-size: var(--x-button-font-size-medium);
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
    font-weight: var(--x-button-font-weight);
    line-height: 1.4;
    white-space: nowrap;
    text-align: center;
    background: var(--bg-300);
    border: var(--x-button-border-width) var(--x-button-border-style) var(--border-300);
    border-radius: var(--x-button-border-radius);
    color: var(--text-200);
    cursor: pointer;
    user-select: none;
    touch-action: manipulation;
    outline: none;
    box-shadow: var(--x-button-shadow);
    overflow: hidden;
  }

  .x-button::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(255, 255, 255, 0.1);
    opacity: 0;
    pointer-events: none;
  }

  .x-button:hover {
    color: var(--text-200);
    border-color: var(--border-400);
    background: var(--bg-400);
    box-shadow: var(--x-button-shadow-hover);
  }

  .x-button:hover::before {
    opacity: 1;
  }

  .x-button:focus {
    color: var(--text-200);
    border-color: var(--color-primary);
    box-shadow:
      0 0 0 3px var(--color-primary-alpha),
      var(--x-button-shadow);
    outline: none;
  }

  .x-button:active {
    color: var(--text-200);
    border-color: var(--border-300);
    background: var(--bg-200);
    box-shadow: var(--x-button-shadow-active);
  }

  .x-button:active::before {
    opacity: 0;
  }

  .x-button--small {
    padding: var(--x-button-padding-small);
    font-size: var(--x-button-font-size-small);
    border-radius: var(--x-button-border-radius);
  }

  .x-button--large {
    padding: var(--x-button-padding-large);
    font-size: var(--x-button-font-size-large);
    border-radius: calc(var(--x-button-border-radius) + 2px);
  }
  .x-button--primary {
    color: var(--bg-100);
    background: var(--color-primary);
    border-color: var(--color-primary);
    box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.1);
    font-weight: 600;
  }

  .x-button--primary::before {
    background: rgba(0, 0, 0, 0.05);
  }

  .x-button--primary:hover {
    color: var(--bg-100);
    background: var(--color-primary-hover);
    border-color: var(--color-primary-hover);
    box-shadow: 0 2px 4px 0 rgba(0, 0, 0, 0.15);
  }

  .x-button--primary:focus {
    color: var(--bg-100);
    background: var(--color-primary);
    border-color: var(--color-primary);
    box-shadow:
      0 0 0 3px var(--color-primary-alpha),
      0 1px 2px 0 rgba(0, 0, 0, 0.1);
  }

  .x-button--primary:active {
    color: var(--bg-100);
    background: var(--color-primary-hover);
    border-color: var(--color-primary-hover);
    box-shadow: inset 0 1px 2px 0 rgba(0, 0, 0, 0.1);
  }

  .x-button--secondary {
    color: var(--text-200);
    background: var(--bg-400);
    border-color: var(--border-300);
    box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
  }

  .x-button--secondary:hover {
    color: var(--text-100);
    background: var(--bg-500);
    border-color: var(--border-400);
    box-shadow: 0 2px 4px 0 rgba(0, 0, 0, 0.1);
  }

  .x-button--secondary:active {
    color: var(--text-100);
    background: var(--bg-300);
    border-color: var(--border-400);
    box-shadow: inset 0 1px 2px 0 rgba(0, 0, 0, 0.05);
  }
  .x-button--danger {
    color: var(--bg-100);
    background: var(--color-error);
    border-color: var(--color-error);
    box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.1);
    font-weight: 600;
  }

  .x-button--danger::before {
    background: rgba(0, 0, 0, 0.05);
  }

  .x-button--danger:hover {
    color: var(--bg-100);
    background: var(--ansi-red);
    border-color: var(--ansi-red);
    box-shadow: 0 2px 4px 0 rgba(0, 0, 0, 0.15);
  }

  .x-button--danger:focus {
    color: var(--bg-100);
    background: var(--color-error);
    border-color: var(--color-error);
    box-shadow:
      0 0 0 3px rgba(244, 71, 71, 0.2),
      0 1px 2px 0 rgba(0, 0, 0, 0.1);
  }

  .x-button--danger:active {
    color: var(--bg-100);
    background: var(--ansi-red);
    border-color: var(--ansi-red);
    box-shadow: inset 0 1px 2px 0 rgba(0, 0, 0, 0.1);
  }

  .x-button--ghost {
    color: var(--text-200);
    background: transparent;
    border-color: var(--border-300);
    box-shadow: none;
  }

  .x-button--ghost::before {
    background: var(--color-primary-alpha);
  }

  .x-button--ghost:hover {
    color: var(--color-primary);
    border-color: var(--color-primary);
    background: var(--color-primary-alpha);
    box-shadow: 0 2px 8px 0 var(--color-primary-alpha);
  }

  .x-button--ghost:active {
    color: var(--color-primary-hover);
    border-color: var(--color-primary-hover);
    background: var(--color-primary-alpha);
    box-shadow: none;
  }
  .x-button--link {
    color: var(--color-primary);
    background: transparent;
    border-color: transparent;
    box-shadow: none;
    padding: 0 4px;
  }

  .x-button--link::before {
    background: var(--color-primary-alpha);
  }

  .x-button--link:hover {
    color: var(--color-primary-hover);
    background: var(--color-primary-alpha);
    border-color: transparent;
    box-shadow: none;
  }

  .x-button--link:focus {
    color: var(--color-primary-hover);
    background: transparent;
    border-color: transparent;
    box-shadow: 0 0 0 2px var(--color-primary-alpha);
    border-radius: calc(var(--x-button-border-radius) - 2px);
  }

  .x-button--link:active {
    color: #0284c7;
    background: var(--color-primary-alpha);
  }

  .x-button--disabled,
  .x-button:disabled {
    color: var(--text-500) !important;
    background: var(--bg-200) !important;
    border-color: var(--border-200) !important;
    cursor: not-allowed !important;
    box-shadow: none !important;
    opacity: 0.6 !important;
  }

  .x-button--disabled::before,
  .x-button:disabled::before {
    display: none;
  }

  .x-button--link.x-button--disabled,
  .x-button--link:disabled {
    color: var(--text-500) !important;
    background: transparent !important;
    border-color: transparent !important;
  }
  .x-button--loading {
    position: relative;
    pointer-events: none;
    cursor: default;
  }

  .x-button--loading .x-button__content {
    opacity: 0.7;
  }

  .x-button--loading:hover {
    transform: none !important;
  }

  .x-button--block {
    width: 100%;
  }
  .x-button--round {
    border-radius: calc(var(--x-button-border-radius) * 3);
  }

  .x-button--round.x-button--small {
    border-radius: calc(var(--x-button-border-radius) * 2.5);
  }

  .x-button--round.x-button--large {
    border-radius: calc(var(--x-button-border-radius) * 3.5);
  }

  .x-button--circle {
    width: 36px;
    height: 36px;
    min-width: 36px;
    padding: 0;
    border-radius: 50%;
  }

  .x-button--circle.x-button--small {
    width: 28px;
    height: 28px;
    min-width: 28px;
  }

  .x-button--circle.x-button--large {
    width: 44px;
    height: 44px;
    min-width: 44px;
  }
  .x-button--icon-only {
    padding: 6px;
    min-width: 36px;
  }

  .x-button--icon-only.x-button--small {
    padding: 4px;
    min-width: 28px;
  }

  .x-button--icon-only.x-button--large {
    padding: 8px;
    min-width: 44px;
  }

  .x-button__loading {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: inherit;
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
  .x-button__icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: inherit;
  }

  /* 让通过 #icon 插槽传入的 SVG 图标按按钮尺寸规范显示（不覆盖其 fill/stroke） */
  .x-button__icon :slotted(svg) {
    width: 14px;
    height: 14px;
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

  .x-button--small .x-button__icon :slotted(svg) {
    width: 12px;
    height: 12px;
  }

  .x-button--large .x-button__icon-svg {
    width: 16px;
    height: 16px;
  }

  .x-button--large .x-button__icon :slotted(svg) {
    width: 16px;
    height: 16px;
  }

  .x-button__content {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: inherit;
  }
  @keyframes x-button-spin {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 768px) {
    .x-button--block {
      width: 100%;
    }
  }

  @media (prefers-contrast: high) {
    .x-button {
      border-width: 2px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .x-button {
      transition: none;
    }

    .x-button__loading-icon {
      animation: none;
    }
  }
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
