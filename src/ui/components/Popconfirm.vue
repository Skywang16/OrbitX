<template>
  <div class="popconfirm-wrapper">
    <div ref="triggerRef" class="popconfirm-trigger" @click="handleTriggerClick">
      <slot name="trigger">
        <x-button v-bind="triggerProps">
          <slot name="trigger-content">
            {{ triggerText }}
          </slot>
        </x-button>
      </slot>
    </div>

    <teleport to="body">
      <transition name="popconfirm-fade">
        <div v-if="visible" ref="popoverRef" class="popconfirm-popover" :style="popoverStyle" @click.stop>
          <div class="popconfirm-content" :data-type="type">
            <div class="popconfirm-header">
              <div class="popconfirm-icon">
                <component v-if="iconComponent" :is="iconComponent" />
                <svg v-else-if="type === 'danger'" class="popconfirm-icon-svg" viewBox="0 0 20 20" fill="currentColor">
                  <path
                    fill-rule="evenodd"
                    d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
                    clip-rule="evenodd"
                  />
                </svg>
                <svg v-else-if="type === 'warning'" class="popconfirm-icon-svg" viewBox="0 0 20 20" fill="currentColor">
                  <path
                    fill-rule="evenodd"
                    d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
                    clip-rule="evenodd"
                  />
                </svg>
                <svg v-else class="popconfirm-icon-svg" viewBox="0 0 20 20" fill="currentColor">
                  <path
                    fill-rule="evenodd"
                    d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z"
                    clip-rule="evenodd"
                  />
                </svg>
              </div>
              <div class="popconfirm-message">
                {{ title }}
              </div>
            </div>

            <div class="popconfirm-description" v-if="description">
              {{ description }}
            </div>

            <div class="popconfirm-actions">
              <x-button size="small" variant="secondary" @click="handleCancel" :disabled="loading">
                {{ cancelText }}
              </x-button>
              <x-button size="small" :variant="confirmButtonVariant" @click="handleConfirm" :loading="loading">
                {{ confirmText }}
              </x-button>
            </div>
          </div>

          <div class="popconfirm-arrow" :data-placement="actualPlacement"></div>
        </div>
      </transition>
    </teleport>

    <teleport to="body">
      <div v-if="visible && closeOnClickOutside" class="popconfirm-mask" @click="handleMaskClick"></div>
    </teleport>
  </div>
</template>

<script setup lang="ts">
  import { computed, ref, nextTick, onMounted, watch } from 'vue'
  import type { PopconfirmProps } from '../types/index'
  import type { Placement } from '../types/index'

  const props = withDefaults(defineProps<PopconfirmProps>(), {
    title: '确定要执行此操作吗？',
    description: '',
    confirmText: '确定',
    cancelText: '取消',
    type: 'warning',
    placement: 'top',
    trigger: 'click',
    disabled: false,
    loading: false,
    closeOnClickOutside: true,
    offset: 8,
    triggerText: '',
  })

  const emit = defineEmits<{
    (e: 'confirm'): void
    (e: 'cancel'): void
    (e: 'update:visible', value: boolean): void
  }>()

  const visible = ref(false)
  const triggerRef = ref<HTMLElement>()
  const popoverRef = ref<HTMLElement>()
  const actualPlacement = ref<Placement>(props.placement)

  const confirmButtonVariant = computed(() => {
    switch (props.type) {
      case 'danger':
        return 'danger'
      case 'warning':
        return 'primary'
      case 'info':
        return 'primary'
      default:
        return 'primary'
    }
  })

  const iconComponent = computed(() => {
    if (typeof props.icon === 'object') {
      return props.icon
    }
    return null
  })

  const triggerProps = computed(() => {
    return {
      variant: props.triggerButtonVariant || (props.type === 'danger' ? 'danger' : 'secondary'),
      size: props.triggerButtonSize || 'medium',
      disabled: props.disabled,
      ...props.triggerButtonProps,
    }
  })

  const popoverStyle = ref({})

  const calculatePosition = async () => {
    if (!triggerRef.value) return

    const triggerRect = triggerRef.value.getBoundingClientRect()
    const viewportWidth = window.innerWidth
    const viewportHeight = window.innerHeight

    // 获取实际弹窗尺寸，如果不存在则使用预估值
    let popoverWidth = 250
    let popoverHeight = 120
    if (popoverRef.value) {
      const popoverRect = popoverRef.value.getBoundingClientRect()
      if (popoverRect.width > 0 && popoverRect.height > 0) {
        popoverWidth = popoverRect.width
        popoverHeight = popoverRect.height
      }
    }

    let top = 0
    let left = 0
    actualPlacement.value = props.placement

    // 计算基础位置
    switch (props.placement) {
      case 'top':
        top = triggerRect.top - popoverHeight - props.offset
        left = triggerRect.left + (triggerRect.width - popoverWidth) / 2
        break
      case 'bottom':
        top = triggerRect.bottom + props.offset
        left = triggerRect.left + (triggerRect.width - popoverWidth) / 2
        break
      case 'left':
        top = triggerRect.top + (triggerRect.height - popoverHeight) / 2
        left = triggerRect.left - popoverWidth - props.offset
        break
      case 'right':
        top = triggerRect.top + (triggerRect.height - popoverHeight) / 2
        left = triggerRect.right + props.offset
        break
    }

    // 边界检查和自适应调整
    const margin = 8
    // 检查是否超出视窗边界，如果超出则调整placement
    if (props.placement === 'top' && top < margin) {
      // 上方空间不足，改为下方
      top = triggerRect.bottom + props.offset
      actualPlacement.value = 'bottom'
    } else if (props.placement === 'bottom' && top + popoverHeight > viewportHeight - margin) {
      // 下方空间不足，改为上方
      top = triggerRect.top - popoverHeight - props.offset
      actualPlacement.value = 'top'
    } else if (props.placement === 'left' && left < margin) {
      // 左侧空间不足，改为右侧
      left = triggerRect.right + props.offset
      actualPlacement.value = 'right'
    } else if (props.placement === 'right' && left + popoverWidth > viewportWidth - margin) {
      // 右侧空间不足，改为左侧
      left = triggerRect.left - popoverWidth - props.offset
      actualPlacement.value = 'left'
    }

    // 水平位置边界检查
    if (left < margin) left = margin
    if (left + popoverWidth > viewportWidth - margin) {
      left = viewportWidth - popoverWidth - margin
    }
    // 垂直位置边界检查
    if (top < margin) top = margin
    if (top + popoverHeight > viewportHeight - margin) {
      top = viewportHeight - popoverHeight - margin
    }

    popoverStyle.value = {
      position: 'fixed',
      top: `${top}px`,
      left: `${left}px`,
      zIndex: 1000,
    }
  }

  const handleTriggerClick = async () => {
    if (props.disabled || props.trigger !== 'click') return

    visible.value = !visible.value
  }

  // 监听弹窗显示状态，在显示时计算位置
  watch(visible, async newVisible => {
    if (newVisible) {
      await nextTick()
      calculatePosition()
      // 延迟一帧再次计算，确保获取到正确的弹窗尺寸
      await nextTick()
      setTimeout(calculatePosition, 0)
    }
  })

  const handleConfirm = async () => {
    emit('confirm')
    if (!props.loading) {
      visible.value = false
    }
  }

  const handleCancel = () => {
    emit('cancel')
    visible.value = false
  }

  const handleMaskClick = () => {
    if (props.closeOnClickOutside) {
      visible.value = false
    }
  }

  onMounted(() => {
    if (visible.value) {
      calculatePosition()
    }
  })
</script>

<style scoped>
  .popconfirm-wrapper {
    display: inline-block;
  }

  .popconfirm-trigger {
    display: inline-block;
  }

  .popconfirm-mask {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 999;
    background: transparent;
  }

  .popconfirm-popover {
    background: var(--bg-100);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-md);
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.15);
    position: fixed;
    max-width: 300px;
    min-width: 200px;
  }

  .popconfirm-content {
    padding: var(--spacing-md);
  }

  .popconfirm-header {
    display: flex;
    align-items: flex-start;
    gap: var(--spacing-sm);
    margin-bottom: var(--spacing-sm);
  }

  .popconfirm-icon {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    margin-top: 2px;
  }

  .popconfirm-icon-svg {
    width: 16px;
    height: 16px;
    fill: currentColor;
  }

  .popconfirm-message {
    flex: 1;
    font-size: var(--font-size-md);
    font-weight: 500;
    line-height: 1.4;
    color: var(--text-100);
  }

  .popconfirm-description {
    font-size: var(--font-size-sm);
    line-height: 1.4;
    color: var(--text-300);
    margin-bottom: var(--spacing-md);
    margin-left: calc(16px + var(--spacing-sm));
  }

  .popconfirm-actions {
    display: flex;
    justify-content: space-between;
    gap: var(--spacing-sm);
  }

  .popconfirm-icon {
    color: var(--color-primary);
  }

  .popconfirm-content[data-type='danger'] .popconfirm-icon {
    color: var(--color-error);
  }

  .popconfirm-content[data-type='warning'] .popconfirm-icon {
    color: var(--color-warning, #f59e0b);
  }

  .popconfirm-arrow {
    position: absolute;
    width: 8px;
    height: 8px;
    background: var(--bg-100);
    border: 1px solid var(--border-200);
    transform: rotate(45deg);
  }

  /* 根据不同方向的箭头位置和样式 */
  .popconfirm-popover {
    position: relative;
  }

  /* 默认朝上（top placement 时箭头在下方） */
  .popconfirm-arrow[data-placement='top'] {
    bottom: -5px;
    left: 50%;
    margin-left: -4px;
    border-top: none;
    border-left: none;
  }

  /* 朝下（bottom placement 时箭头在上方） */
  .popconfirm-arrow[data-placement='bottom'] {
    top: -5px;
    left: 50%;
    margin-left: -4px;
    border-bottom: none;
    border-right: none;
  }

  /* 朝右（left placement 时箭头在右侧） */
  .popconfirm-arrow[data-placement='left'] {
    right: -5px;
    top: 50%;
    margin-top: -4px;
    border-top: none;
    border-right: none;
  }

  /* 朝左（right placement 时箭头在左侧） */
  .popconfirm-arrow[data-placement='right'] {
    left: -5px;
    top: 50%;
    margin-top: -4px;
    border-bottom: none;
    border-left: none;
  }

  .popconfirm-fade-enter-active,
  .popconfirm-fade-leave-active {
    transition: opacity 0.2s ease;
  }

  .popconfirm-fade-enter-from,
  .popconfirm-fade-leave-to {
    opacity: 0;
  }
</style>
