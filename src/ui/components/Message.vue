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
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 18px;
    margin-bottom: 10px;
    background: var(--bg-400);
    border-radius: var(--border-radius-xl);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    font-size: 14px;
    color: var(--text-200);
    max-width: 320px;
    min-width: 240px;
  }

  .x-message--success {
    background: #d1fae5;
    border: 1px solid #10b981;
  }

  .x-message--error {
    background: #fee2e2;
    border: 1px solid #ef4444;
  }

  .x-message--warning {
    background: #fef3c7;
    border: 1px solid #f59e0b;
  }

  .x-message--info {
    background: #dbeafe;
    border: 1px solid #3b82f6;
  }

  .x-message__content {
    flex: 1;
  }

  .x-message__text {
    margin: 0;
    color: var(--text-200);
  }

  .x-message__close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    background: rgba(255, 255, 255, 0.2);
    border: none;
    color: var(--text-400);
    cursor: pointer;
    border-radius: 50%;
    opacity: 0.7;
  }

  .x-message__close:hover {
    color: var(--text-200);
    background: rgba(255, 255, 255, 0.3);
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
