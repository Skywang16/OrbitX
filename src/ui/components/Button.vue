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

<style scoped>
  .x-button {
    --x-button-font-weight: 400;
    --x-button-border-width: 1px;
    --x-button-border-style: solid;
    --x-button-border-radius: var(--border-radius);
    --x-button-transition: all var(--x-duration-normal) var(--x-ease-in-out);

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
    background: var(--bg-400);
    border: var(--x-button-border-width) var(--x-button-border-style) var(--border-300);
    border-radius: var(--x-button-border-radius);
    color: var(--text-200);
    cursor: pointer;
    transition: var(--x-button-transition);
    user-select: none;
    touch-action: manipulation;
    outline: none;
  }

  .x-button:hover {
    color: var(--color-primary);
    border-color: var(--color-primary);
    background: var(--color-hover);
  }

  .x-button:focus {
    color: var(--color-primary);
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px var(--color-primary-alpha);
  }

  .x-button:active {
    color: var(--color-primary);
    border-color: var(--color-primary);
    background: var(--color-hover);
  }

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
  .x-button--primary {
    color: var(--text-100);
    background: var(--color-primary);
    border-color: var(--color-primary);
  }

  .x-button--primary:hover {
    color: var(--text-100);
    background: var(--color-primary);
    border-color: var(--color-primary);
    opacity: 0.8;
  }

  .x-button--primary:focus {
    color: var(--text-100);
    background: var(--color-primary);
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px var(--color-primary-alpha);
  }

  .x-button--primary:active {
    color: var(--text-100);
    background: var(--color-primary);
    border-color: var(--color-primary);
    opacity: 0.9;
  }

  .x-button--secondary {
    color: var(--text-200);
    background: var(--bg-500);
    border-color: var(--border-300);
  }

  .x-button--secondary:hover {
    background: var(--color-hover);
    border-color: var(--border-400);
  }
  .x-button--danger {
    color: var(--text-100);
    background: var(--color-error);
    border-color: var(--color-error);
  }

  .x-button--danger:hover {
    color: var(--text-100);
    background: var(--color-error);
    border-color: var(--color-error);
    opacity: 0.8;
  }

  .x-button--danger:focus {
    color: var(--text-100);
    background: var(--color-error);
    border-color: var(--color-error);
    box-shadow: 0 0 0 2px var(--color-primary-alpha);
  }

  .x-button--danger:active {
    color: var(--text-100);
    background: var(--color-error);
    border-color: var(--color-error);
    opacity: 0.9;
  }

  .x-button--ghost {
    color: var(--text-200);
    background: transparent;
    border-color: var(--border-300);
  }

  .x-button--ghost:hover {
    color: var(--color-primary);
    border-color: var(--color-primary);
    background: var(--color-primary-alpha);
  }
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

  .x-button--disabled,
  .x-button:disabled {
    color: var(--text-500) !important;
    background: var(--bg-500) !important;
    border-color: var(--border-300) !important;
    cursor: not-allowed !important;
    box-shadow: none !important;
    opacity: 0.6 !important;
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
  }

  .x-button--loading .x-button__content {
    opacity: 0.6;
  }

  .x-button--block {
    width: 100%;
  }
  .x-button--round {
    border-radius: 32px;
  }

  .x-button--round.x-button--small {
    border-radius: 24px;
  }

  .x-button--round.x-button--large {
    border-radius: 40px;
  }

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

  .x-button__content {
    display: inline-flex;
    align-items: center;
    justify-content: center;
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
