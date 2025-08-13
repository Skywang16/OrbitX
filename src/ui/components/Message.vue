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
          <svg class="x-message__icon-svg" viewBox="0 0 24 24">
            <use :href="`#${iconName}`"></use>
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

  // 计算图标名称
  const iconName = computed(() => {
    const iconMap = {
      success: 'check-circle',
      error: 'x-circle',
      warning: 'alert-triangle',
      info: 'info-circle',
    }
    return iconMap[props.type] || 'info-circle'
  })

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
  /* 消息组件样式 - 使用全局主题变量 */
  .x-message {
    position: relative;
    display: flex;
    align-items: flex-start;
    gap: var(--spacing-sm);
    padding: var(--spacing-md) var(--spacing-lg);
    margin-bottom: var(--spacing-sm);
    background: var(--bg-400);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius);
    box-shadow: var(--shadow-sm);
    font-family: var(--font-family);
    font-size: var(--font-size-md);
    line-height: 1.5;
    color: var(--text-200);
    transition: all 0.3s cubic-bezier(0.645, 0.045, 0.355, 1);
    max-width: 400px;
    word-wrap: break-word;
  }

  /* 消息类型样式 */
  .x-message--success {
    border-color: var(--color-success);
    background: var(--bg-400);
  }

  .x-message--success .x-message__icon {
    color: var(--color-success);
  }

  .x-message--error {
    border-color: var(--color-error);
    background: var(--bg-400);
  }

  .x-message--error .x-message__icon {
    color: var(--color-error);
  }

  .x-message--warning {
    border-color: var(--color-warning);
    background: var(--bg-400);
  }

  .x-message--warning .x-message__icon {
    color: var(--color-warning);
  }

  .x-message--info {
    border-color: var(--color-info);
    background: var(--bg-400);
  }

  .x-message--info .x-message__icon {
    color: var(--color-info);
  }

  /* 图标样式 */
  .x-message__icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    margin-top: 2px;
  }

  .x-message__icon-svg {
    width: 100%;
    height: 100%;
    fill: currentColor;
  }

  /* 内容样式 */
  .x-message__content {
    flex: 1;
    min-width: 0;
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
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    margin-top: 2px;
    padding: 0;
    background: transparent;
    border: none;
    color: var(--text-400);
    cursor: pointer;
    border-radius: var(--border-radius-sm);
    transition: all 0.2s ease;
  }

  .x-message__close:hover {
    color: var(--text-200);
    background: var(--color-hover);
  }

  .x-message__close-icon {
    width: 12px;
    height: 12px;
    stroke: currentColor;
    fill: none;
  }

  /* 可关闭消息的样式调整 */
  .x-message--closable {
    padding-right: var(--spacing-md);
  }

  /* 带图标消息的样式调整 */
  .x-message--with-icon .x-message__content {
    margin-left: 0;
  }

  /* 动画效果 */
  .x-message-enter-active,
  .x-message-leave-active {
    transition: all 0.3s cubic-bezier(0.645, 0.045, 0.355, 1);
  }

  .x-message-enter-from {
    opacity: 0;
    transform: translateY(-20px) scale(0.8);
  }

  .x-message-enter-to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }

  .x-message-leave-from {
    opacity: 1;
    transform: translateY(0) scale(1);
  }

  .x-message-leave-to {
    opacity: 0;
    transform: translateY(-20px) scale(0.8);
  }

  /* 响应式设计 */
  @media (max-width: 768px) {
    .x-message {
      max-width: calc(100vw - 32px);
      margin: 0 var(--spacing-lg);
    }
  }

  /* 减少动画模式支持 */
  @media (prefers-reduced-motion: reduce) {
    .x-message,
    .x-message-enter-active,
    .x-message-leave-active {
      transition: none;
    }
  }
</style>
