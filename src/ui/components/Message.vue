<template>
  <div v-if="visible" :class="messageClasses" role="alert" :aria-live="type === 'error' ? 'assertive' : 'polite'">
    <div class="x-message__content">
      <div v-if="dangerouslyUseHTMLString" class="x-message__text" v-html="message"></div>
      <div v-else class="x-message__text">{{ message }}</div>
    </div>
    <button v-if="closable" class="x-message__close" type="button" aria-label="关闭消息" @click="handleClose">
      <svg class="x-message__close-icon" viewBox="0 0 24 24">
        <path d="M18 6L6 18M6 6l12 12" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
      </svg>
    </button>
  </div>
</template>

<script setup lang="ts">
  import { computed, inject, onMounted, onUnmounted } from 'vue'
  import type { MessageProps } from '../types/index'

  const props = withDefaults(defineProps<MessageProps>(), {
    type: 'info',
    duration: 3000,
    closable: false,
    showIcon: true,
    dangerouslyUseHTMLString: false,
  })

  const emit = defineEmits<{
    close: []
  }>()

  inject('xui-config', {})

  let timer: number | null = null

  const messageClasses = computed(() => [
    'x-message',
    `x-message--${props.type}`,
    {
      'x-message--closable': props.closable,
    },
  ])

  const handleClose = () => {
    clearTimer()
    emit('close')
  }

  const clearTimer = () => {
    if (timer) {
      clearTimeout(timer)
      timer = null
    }
  }

  const startTimer = () => {
    if (props.duration > 0) {
      timer = window.setTimeout(() => {
        handleClose()
      }, props.duration)
    }
  }

  onMounted(() => {
    if (props.visible) {
      startTimer()
    }
  })

  onUnmounted(() => {
    clearTimer()
  })
</script>

<style scoped>
  .x-message {
    --x-message-bg: var(--bg-400);
    --x-message-border: var(--border-300);
    --x-message-fg: var(--text-100);

    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 18px;
    margin-bottom: 10px;
    background: var(--x-message-bg);
    border: 1px solid var(--x-message-border);
    border-radius: var(--border-radius-xl);
    box-shadow: var(--x-shadow-md);
    font-size: 14px;
    color: var(--x-message-fg);
    max-width: 320px;
    min-width: 240px;
  }

  .x-message--success {
    --x-message-bg: color-mix(in srgb, var(--color-success) 18%, var(--bg-400));
    --x-message-border: color-mix(in srgb, var(--color-success) 55%, var(--border-300));
  }

  .x-message--error {
    --x-message-bg: color-mix(in srgb, var(--color-error) 18%, var(--bg-400));
    --x-message-border: color-mix(in srgb, var(--color-error) 55%, var(--border-300));
  }

  .x-message--warning {
    --x-message-bg: color-mix(in srgb, var(--color-warning) 18%, var(--bg-400));
    --x-message-border: color-mix(in srgb, var(--color-warning) 55%, var(--border-300));
  }

  .x-message--info {
    --x-message-bg: color-mix(in srgb, var(--color-info) 18%, var(--bg-400));
    --x-message-border: color-mix(in srgb, var(--color-info) 55%, var(--border-300));
  }

  .x-message__content {
    flex: 1;
  }

  .x-message__text {
    margin: 0;
    color: inherit;
  }

  .x-message__close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    background: color-mix(in srgb, var(--x-message-fg) 12%, transparent);
    border: none;
    color: color-mix(in srgb, var(--x-message-fg) 75%, transparent);
    cursor: pointer;
    border-radius: 50%;
    opacity: 0.7;
  }

  .x-message__close:hover {
    color: var(--x-message-fg);
    background: color-mix(in srgb, var(--x-message-fg) 18%, transparent);
    opacity: 1;
  }

  .x-message__close-icon {
    width: 12px;
    height: 12px;
    stroke: currentColor;
    fill: none;
  }

  @media (max-width: 768px) {
    .x-message {
      max-width: calc(100vw - 32px);
    }
  }
</style>
