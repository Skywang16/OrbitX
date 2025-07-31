<template>
  <Teleport to="body">
    <div v-if="visible" class="modal-overlay" @click.self="handleOverlayClick">
      <div
        ref="modalRef"
        class="modal-container"
        :class="sizeClass"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="titleId"
      >
        <!-- 模态框头部 -->
        <div v-if="showHeader" class="modal-header">
          <div class="modal-title-section">
            <h3 v-if="title" :id="titleId" class="modal-title">{{ title }}</h3>
            <slot name="title"></slot>
          </div>
          <button v-if="closable" class="modal-close-button" @click="handleClose" :title="closeButtonTitle">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <!-- 模态框内容 -->
        <div class="modal-body" :class="{ 'no-padding': noPadding }">
          <slot></slot>
        </div>

        <!-- 模态框底部 -->
        <div v-if="showFooter" class="modal-footer">
          <slot name="footer">
            <div class="modal-actions">
              <button
                v-if="showCancelButton"
                type="button"
                class="modal-button modal-button-secondary"
                @click="handleCancel"
                :disabled="loading"
              >
                {{ cancelText }}
              </button>
              <button
                v-if="showConfirmButton"
                type="button"
                class="modal-button modal-button-primary"
                @click="handleConfirm"
                :disabled="loading"
              >
                <svg
                  v-if="loading"
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  class="spinning"
                >
                  <path d="M21 12a9 9 0 11-6.219-8.56" />
                </svg>
                {{ loading ? loadingText : confirmText }}
              </button>
            </div>
          </slot>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
  import { computed, onMounted, onUnmounted, watch } from 'vue'

  interface Props {
    visible?: boolean
    title?: string
    size?: 'small' | 'medium' | 'large' | 'full'
    closable?: boolean
    maskClosable?: boolean
    showHeader?: boolean
    showFooter?: boolean
    showCancelButton?: boolean
    showConfirmButton?: boolean
    cancelText?: string
    confirmText?: string
    loadingText?: string
    closeButtonTitle?: string
    loading?: boolean
    noPadding?: boolean
    zIndex?: number
  }

  interface Emits {
    (e: 'update:visible', visible: boolean): void
    (e: 'close'): void
    (e: 'cancel'): void
    (e: 'confirm'): void
    (e: 'opened'): void
    (e: 'closed'): void
  }

  const props = withDefaults(defineProps<Props>(), {
    visible: false,
    title: '',
    size: 'medium',
    closable: true,
    maskClosable: true,
    showHeader: true,
    showFooter: false,
    showCancelButton: true,
    showConfirmButton: true,
    cancelText: '取消',
    confirmText: '确定',
    loadingText: '处理中...',
    closeButtonTitle: '关闭',
    loading: false,
    noPadding: false,
    zIndex: 1000,
  })

  const emit = defineEmits<Emits>()

  // 计算尺寸类名
  const sizeClass = computed(() => `modal-${props.size}`)

  // 处理遮罩点击
  const handleOverlayClick = () => {
    if (props.maskClosable) {
      handleClose()
    }
  }

  // 处理关闭
  const handleClose = () => {
    emit('update:visible', false)
    emit('close')
  }

  // 处理取消
  const handleCancel = () => {
    emit('cancel')
    if (!props.loading) {
      handleClose()
    }
  }

  // 处理确认
  const handleConfirm = () => {
    emit('confirm')
  }

  // 处理ESC键
  const handleKeydown = (event: KeyboardEvent) => {
    if (event.key === 'Escape' && props.visible && props.closable) {
      handleClose()
    }
  }

  // 监听visible变化
  watch(
    () => props.visible,
    newVisible => {
      if (newVisible) {
        emit('opened')
        document.body.style.overflow = 'hidden'
      } else {
        emit('closed')
        document.body.style.overflow = ''
      }
    }
  )

  // 组件挂载时添加键盘监听
  onMounted(() => {
    document.addEventListener('keydown', handleKeydown)
  })

  // 组件卸载时清理
  onUnmounted(() => {
    document.removeEventListener('keydown', handleKeydown)
    document.body.style.overflow = ''
  })
</script>

<style scoped>
  /* 模态框样式 - 使用全局主题变量 */
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: v-bind(zIndex);
    padding: var(--spacing-lg);
    backdrop-filter: blur(2px);
  }

  .modal-container {
    background-color: var(--color-background);
    border: 1px solid var(--border-color);
    border-radius: var(--border-radius-lg);
    box-shadow: var(--shadow-sm);
    max-height: 90vh;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: modalSlideIn 0.2s ease-out;
    font-family: var(--font-family);
  }

  @keyframes modalSlideIn {
    from {
      opacity: 0;
      transform: translateY(-20px) scale(0.95);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  /* 尺寸变体 */
  .modal-small {
    width: 100%;
    max-width: 400px;
  }

  .modal-medium {
    width: 100%;
    max-width: 600px;
  }

  .modal-large {
    width: 100%;
    max-width: 800px;
  }

  .modal-full {
    width: 95vw;
    height: 95vh;
    max-width: none;
    max-height: none;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--spacing-lg);
    border-bottom: 1px solid var(--border-color);
    flex-shrink: 0;
  }

  .modal-title-section {
    flex: 1;
    min-width: 0;
  }

  .modal-title {
    font-size: var(--font-size-lg);
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .modal-close-button {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: var(--spacing-xs);
    border-radius: var(--border-radius);
    transition: all 0.2s ease;
    flex-shrink: 0;
    margin-left: var(--spacing-md);
  }

  .modal-close-button:hover {
    background-color: var(--color-background-hover);
    color: var(--text-primary);
  }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: var(--spacing-lg);
  }

  .modal-body.no-padding {
    padding: 0;
  }

  .modal-footer {
    flex-shrink: 0;
    padding: var(--spacing-lg);
    border-top: 1px solid var(--border-color);
    background-color: var(--color-background-secondary);
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--spacing-sm);
  }

  .modal-button {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    padding: var(--spacing-sm) var(--spacing-lg);
    border-radius: var(--border-radius);
    font-size: var(--font-size-sm);
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
    border: 1px solid;
    min-width: 80px;
    justify-content: center;
  }

  .modal-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .modal-button-secondary {
    background-color: transparent;
    border-color: var(--border-color);
    color: var(--text-primary);
  }

  .modal-button-secondary:hover:not(:disabled) {
    background-color: var(--color-background-hover);
  }

  .modal-button-primary {
    background-color: var(--color-primary);
    border-color: var(--color-primary);
    color: white;
  }

  .modal-button-primary:hover:not(:disabled) {
    background-color: var(--color-primary-hover);
  }

  .spinning {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  /* 响应式设计 */
  @media (max-width: 768px) {
    .modal-overlay {
      padding: var(--spacing-md);
    }

    .modal-container {
      max-height: 95vh;
    }

    .modal-small,
    .modal-medium,
    .modal-large {
      width: 100%;
      max-width: none;
    }

    .modal-header,
    .modal-body,
    .modal-footer {
      padding: var(--spacing-md);
    }

    .modal-actions {
      flex-direction: column-reverse;
    }

    .modal-button {
      width: 100%;
    }
  }
</style>
