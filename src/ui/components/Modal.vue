<template>
  <Teleport to="body">
    <div v-if="visible" class="modal-overlay" @mousedown="handleOverlayMouseDown" @mouseup="handleOverlayMouseUp">
      <div ref="modalRef" class="modal-container" :class="sizeClass" role="dialog" aria-modal="true">
        <div v-if="showHeader" class="modal-header">
          <div class="modal-title-section">
            <h3 v-if="title" class="modal-title">{{ title }}</h3>
            <slot name="title"></slot>
          </div>
          <button
            v-if="closable"
            class="modal-close-button"
            @click="handleClose"
            :title="closeButtonTitle || t('dialog.close')"
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>
        <div class="modal-body" :class="{ 'no-padding': noPadding }">
          <slot></slot>
        </div>

        <div v-if="showFooter" class="modal-footer">
          <slot name="footer">
            <div class="modal-actions">
              <XButton
                v-if="showCancelButton"
                variant="secondary"
                size="small"
                @click="handleCancel"
                :disabled="loading"
              >
                {{ cancelText || t('dialog.cancel') }}
              </XButton>
              <XButton
                v-if="showConfirmButton"
                :variant="confirmButtonClass === 'danger' ? 'danger' : 'primary'"
                size="small"
                @click="handleConfirm"
                :loading="loading"
                :disabled="loading"
              >
                {{ confirmText || t('dialog.confirm') }}
              </XButton>
            </div>
          </slot>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
  import { computed, onMounted, onUnmounted, watch, ref } from 'vue'
  import { useI18n } from 'vue-i18n'
  import XButton from './Button.vue'

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
    confirmButtonClass?: string
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
    cancelText: '',
    confirmText: '',
    loadingText: '',
    closeButtonTitle: '',
    loading: false,
    noPadding: false,
    zIndex: 1000,
    confirmButtonClass: '',
  })

  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const isMouseDownOnOverlay = ref(false)

  const sizeClass = computed(() => `modal-${props.size}`)

  const handleOverlayMouseDown = (event: MouseEvent) => {
    if (event.target === event.currentTarget) {
      isMouseDownOnOverlay.value = true
    }
  }

  const handleOverlayMouseUp = (event: MouseEvent) => {
    if (isMouseDownOnOverlay.value && event.target === event.currentTarget && props.maskClosable) {
      handleClose()
    }
    isMouseDownOnOverlay.value = false
  }

  const handleClose = () => {
    emit('update:visible', false)
    emit('close')
  }

  const handleCancel = () => {
    emit('cancel')
    if (!props.loading) {
      handleClose()
    }
  }

  const handleConfirm = () => {
    emit('confirm')
  }

  const handleKeydown = (event: KeyboardEvent) => {
    if (event.key === 'Escape' && props.visible && props.closable) {
      handleClose()
    }
  }

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

  onMounted(() => {
    document.addEventListener('keydown', handleKeydown)
  })

  onUnmounted(() => {
    document.removeEventListener('keydown', handleKeydown)
    document.body.style.overflow = ''
  })
</script>

<style scoped>
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: v-bind(zIndex);
    padding: 16px;
    backdrop-filter: blur(2px);
    animation: fadeIn 0.15s ease-out;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  .modal-container {
    background-color: var(--bg-100);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius-xl);
    box-shadow: var(--x-shadow-xl);
    max-height: 90vh;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: modalSlideIn var(--x-duration-normal) var(--x-ease-out);
    font-family: var(--font-family);
  }

  @keyframes modalSlideIn {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .modal-small {
    width: 100%;
    max-width: min(480px, calc(100vw - 32px));
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
    padding: var(--spacing-xl) var(--spacing-xl) var(--spacing-lg) var(--spacing-xl);
    border-bottom: 1px solid var(--border-300);
    flex-shrink: 0;
  }

  .modal-title-section {
    flex: 1;
    min-width: 0;
  }

  .modal-title {
    font-size: var(--font-size-lg);
    font-weight: 600;
    color: var(--text-100);
    margin: 0;
    line-height: 1.3;
  }

  .modal-close-button {
    background: none;
    border: none;
    color: var(--text-300);
    cursor: pointer;
    padding: 4px;
    border-radius: var(--border-radius-xs);
    transition: all 0.1s ease;
    flex-shrink: 0;
    margin-left: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal-close-button:hover {
    background-color: var(--bg-300);
    color: var(--text-100);
  }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 0 var(--spacing-xl) var(--spacing-xl) var(--spacing-xl);
  }

  .modal-body.no-padding {
    padding: 0;
  }

  .modal-footer {
    flex-shrink: 0;
    padding: var(--spacing-md) var(--spacing-xl) var(--spacing-xl) var(--spacing-xl);
    background-color: var(--bg-100);
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--spacing-md);
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

  @media (max-width: 480px) {
    .modal-overlay {
      padding: 12px;
    }

    .modal-container {
      max-height: 95vh;
    }

    .modal-small,
    .modal-medium,
    .modal-large {
      width: 100%;
      max-width: calc(100vw - 24px);
    }

    .modal-header {
      padding: 16px 16px 12px 16px;
    }

    .modal-body {
      padding: 0 16px 16px 16px;
    }

    .modal-footer {
      padding: 8px 16px 16px 16px;
    }

    .modal-actions {
      flex-direction: column-reverse;
    }

    .modal-actions :deep(.x-button) {
      width: 100%;
    }
  }
</style>
