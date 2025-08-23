<template>
  <Transition
    name="x-message"
    @enter="onEnter"
    @after-enter="onAfterEnter"
    @leave="onLeave"
    @after-leave="onAfterLeave"
  >
    <div
      v-if="visible"
      :class="messageClasses"
      :style="messageStyle"
      role="alert"
      :aria-live="type === 'error' ? 'assertive' : 'polite'"
    >
      <!-- 图标 -->
      <div v-if="showIcon" class="x-message__icon">
        <slot name="icon">
          <!-- 成功图标 -->
          <svg v-if="type === 'success'" class="x-message__icon-svg" viewBox="0 0 20 20" fill="currentColor">
            <path
              fill-rule="evenodd"
              d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
              clip-rule="evenodd"
            />
          </svg>
          <!-- 错误图标 -->
          <svg v-else-if="type === 'error'" class="x-message__icon-svg" viewBox="0 0 20 20" fill="currentColor">
            <path
              fill-rule="evenodd"
              d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
              clip-rule="evenodd"
            />
          </svg>
          <!-- 警告图标 -->
          <svg v-else-if="type === 'warning'" class="x-message__icon-svg" viewBox="0 0 20 20" fill="currentColor">
            <path
              fill-rule="evenodd"
              d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
              clip-rule="evenodd"
            />
          </svg>
          <!-- 信息图标 -->
          <svg v-else class="x-message__icon-svg" viewBox="0 0 20 20" fill="currentColor">
            <path
              fill-rule="evenodd"
              d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z"
              clip-rule="evenodd"
            />
          </svg>
        </slot>
      </div>

      <!-- 消息内容 -->
      <div class="x-message__content">
        <div v-if="dangerouslyUseHTMLString" class="x-message__text" v-html="message"></div>
        <div v-else class="x-message__text">{{ message }}</div>
      </div>

      <!-- 关闭按钮 -->
      <button v-if="closable" class="x-message__close" type="button" :aria-label="closeButtonText" @click="handleClose">
        <svg class="x-message__close-icon" viewBox="0 0 24 24">
          <path d="M18 6L6 18M6 6l12 12" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
        </svg>
      </button>
    </div>
  </Transition>
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

  // 注入全局配置
  inject('xui-config', {})

  let timer: number | null = null

  // 计算消息类名
  const messageClasses = computed(() => [
    'x-message',
    `x-message--${props.type}`,
    {
      'x-message--closable': props.closable,
      'x-message--with-icon': props.showIcon,
    },
  ])

  // 计算消息样式
  const messageStyle = computed(() => ({}))

  // 关闭按钮文本
  const closeButtonText = computed(() => '关闭消息')

  // 处理关闭
  const handleClose = () => {
    clearTimer()
    emit('close')
  }

  // 清除定时器
  const clearTimer = () => {
    if (timer) {
      clearTimeout(timer)
      timer = null
    }
  }

  // 启动定时器
  const startTimer = () => {
    if (props.duration > 0) {
      timer = window.setTimeout(() => {
        handleClose()
      }, props.duration)
    }
  }

  // 进入动画处理
  const onEnter = (el: Element) => {
    const element = el as HTMLElement
    element.style.opacity = '0'
    element.style.transform = 'translateY(-20px) scale(0.8)'
  }

  const onAfterEnter = (el: Element) => {
    const element = el as HTMLElement
    element.style.opacity = '1'
    element.style.transform = 'translateY(0) scale(1)'

    // 启动自动关闭定时器
    startTimer()
  }

  // 离开动画处理
  const onLeave = (el: Element) => {
    const element = el as HTMLElement
    element.style.opacity = '0'
    element.style.transform = 'translateY(-20px) scale(0.8)'
  }

  const onAfterLeave = () => {
    // 动画完成后的清理工作
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
  /* 消息组件样式 */
  .x-message {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 18px;
    margin-bottom: 10px;
    background: var(--bg-400);
    border-radius: 14px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
    font-size: 14px;
    color: var(--text-200);
    max-width: 320px;
    min-width: 240px;
  }

  /* 消息类型样式 - 现代设计 */
  .x-message--success {
    background: linear-gradient(135deg, rgba(16, 185, 129, 0.1), rgba(16, 185, 129, 0.05));
    border: 1px solid rgba(16, 185, 129, 0.2);
  }

  .x-message--success .x-message__icon {
    color: #10b981;
    background: rgba(16, 185, 129, 0.15);
  }

  .x-message--error {
    background: linear-gradient(135deg, rgba(239, 68, 68, 0.1), rgba(239, 68, 68, 0.05));
    border: 1px solid rgba(239, 68, 68, 0.2);
  }

  .x-message--error .x-message__icon {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.15);
  }

  .x-message--warning {
    background: linear-gradient(135deg, rgba(245, 158, 11, 0.1), rgba(245, 158, 11, 0.05));
    border: 1px solid rgba(245, 158, 11, 0.2);
  }

  .x-message--warning .x-message__icon {
    color: #f59e0b;
    background: rgba(245, 158, 11, 0.15);
  }

  .x-message--info {
    background: linear-gradient(135deg, rgba(59, 130, 246, 0.1), rgba(59, 130, 246, 0.05));
    border: 1px solid rgba(59, 130, 246, 0.2);
  }

  .x-message--info .x-message__icon {
    color: #3b82f6;
    background: rgba(59, 130, 246, 0.15);
  }

  /* 图标样式 */
  .x-message__icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    padding: 6px;
    box-sizing: border-box;
  }

  .x-message__icon-svg {
    width: 12px;
    height: 12px;
    fill: currentColor;
  }

  /* 内容样式 */
  .x-message__content {
    flex: 1;
  }

  .x-message__text {
    margin: 0;
    color: var(--text-200);
  }

  /* 关闭按钮样式 */
  .x-message__close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    background: rgba(255, 255, 255, 0.1);
    border: none;
    color: var(--text-400);
    cursor: pointer;
    border-radius: 50%;
    opacity: 0.7;
  }

  .x-message__close:hover {
    color: var(--text-200);
    background: rgba(255, 255, 255, 0.2);
    opacity: 1;
  }

  .x-message__close-icon {
    width: 12px;
    height: 12px;
    stroke: currentColor;
    fill: none;
  }

  /* 动画效果 */
  .x-message-enter-active,
  .x-message-leave-active {
    transition: all 0.3s ease;
  }

  .x-message-enter-from,
  .x-message-leave-to {
    opacity: 0;
    transform: translateY(-20px);
  }

  /* 响应式设计 */
  @media (max-width: 768px) {
    .x-message {
      max-width: calc(100vw - 32px);
    }
  }
</style>
