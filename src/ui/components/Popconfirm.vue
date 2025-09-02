<template>
  <Popover
    v-model="visible"
    :placement="placement"
    :trigger="trigger"
    :disabled="disabled"
    :close-on-click-outside="closeOnClickOutside"
    :close-on-click-inside="false"
    :offset="offset"
    :class="['popconfirm', attrs.class]"
  >
    <template #trigger>
      <slot name="trigger">
        <x-button v-bind="triggerProps">
          <slot name="trigger-content">
            {{ triggerText }}
          </slot>
        </x-button>
      </slot>
    </template>

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
  </Popover>
</template>

<script setup lang="ts">
  import { computed, ref, useAttrs } from 'vue'
  import Popover from './Popover.vue'
  import type { PopconfirmProps } from '../types/index'

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

  // 禁用自动属性继承
  defineOptions({
    inheritAttrs: false,
  })

  // 获取传入的属性
  const attrs = useAttrs()

  const emit = defineEmits<{
    (e: 'confirm'): void
    (e: 'cancel'): void
    (e: 'update:visible', value: boolean): void
  }>()

  const visible = ref(false)

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
</script>

<style scoped>
  .popconfirm :deep(.popover) {
    max-width: 300px;
    min-width: 200px;
  }

  .popconfirm-content {
    padding: 4px;
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
    font-size: 14px;
    font-weight: 500;
    line-height: 1.4;
    color: var(--text-100);
  }

  .popconfirm-description {
    font-size: 13px;
    line-height: 1.4;
    color: var(--text-300);
    margin-bottom: var(--spacing-md);
    margin-left: 24px;
  }

  .popconfirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--spacing-sm);
    margin-top: var(--spacing-md);
  }

  .popconfirm-actions :deep(.x-button) {
    min-width: 60px;
  }

  /* 不同类型的图标颜色 */
  .popconfirm-icon {
    color: var(--color-primary);
  }

  .popconfirm-content[data-type='danger'] .popconfirm-icon {
    color: var(--color-error);
  }

  .popconfirm-content[data-type='warning'] .popconfirm-icon {
    color: var(--color-warning, #f59e0b);
  }

  /* 响应式设计 */
  @media (max-width: 480px) {
    .popconfirm-actions {
      flex-direction: column-reverse;
    }

    .popconfirm-actions :deep(.x-button) {
      width: 100%;
      min-width: unset;
    }
  }

  /* 减少动画模式支持 */
  @media (prefers-reduced-motion: reduce) {
    .popconfirm-content {
      transition: none;
    }
  }
</style>
