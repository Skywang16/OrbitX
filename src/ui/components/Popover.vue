<template>
  <!-- 触发器插槽 - 手动坐标模式也渲染触发器，只是不参与定位计算 -->
  <div ref="triggerRef" class="popover-trigger" @click="handleTriggerClick">
    <slot name="trigger">
      <button class="default-trigger">{{ triggerText }}</button>
    </slot>
  </div>

  <!-- 弹出内容 -->
  <Teleport to="body">
    <div
      v-if="internalVisible"
      ref="popoverRef"
      class="popover"
      :class="[`popover--${placement}`]"
      :style="popoverStyle"
      @click="handlePopoverClick"
    >
      <!-- 内容区域 -->
      <div class="popover-content">
        <slot>
          <div class="popover-menu">
            <div
              v-for="(item, index) in menuItems"
              :key="index"
              class="popover-menu-item"
              :class="{ 'popover-menu-item--disabled': item.disabled }"
              @click="handleMenuItemClick(item)"
            >
              <!-- 支持字符串表情/文本图标 与 组件图标 -->
              <span v-if="typeof item.icon === 'string'" class="menu-item-icon">{{ item.icon }}</span>
              <component v-else-if="item.icon" :is="item.icon" class="menu-item-icon" />
              <span class="menu-item-text">{{ item.label }}</span>
            </div>
          </div>
        </slot>
      </div>
    </div>

    <!-- 遮罩层 -->
    <div v-if="internalVisible && mask" class="popover-mask" @click="handleMaskClick"></div>
  </Teleport>
</template>

<script setup lang="ts">
  import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'

  interface MenuItem {
    label: string
    value?: unknown
    icon?: string | object
    disabled?: boolean
    onClick?: () => void
  }

  interface Props {
    // 显示控制
    modelValue?: boolean
    visible?: boolean

    // 触发方式
    trigger?: 'click' | 'hover' | 'manual'
    triggerText?: string

    // 位置控制
    placement?: 'top' | 'top-start' | 'top-end' | 'bottom' | 'bottom-start' | 'bottom-end' | 'left' | 'right'
    offset?: number
    x?: number // 手动指定 x 坐标
    y?: number // 手动指定 y 坐标

    // 内容
    content?: string
    menuItems?: MenuItem[]

    // 样式
    width?: string | number
    maxWidth?: string | number

    // 行为控制
    disabled?: boolean
    mask?: boolean
    closeOnClickOutside?: boolean
    closeOnClickInside?: boolean

    // 兼容旧版本
    actionText?: string
  }

  const props = withDefaults(defineProps<Props>(), {
    modelValue: false,
    visible: false,
    trigger: 'click',
    placement: 'bottom',
    offset: 8,
    mask: false,
    closeOnClickOutside: true,
    closeOnClickInside: true,
    disabled: false,
    menuItems: () => [],
  })

  const emit = defineEmits<{
    'update:modelValue': [value: boolean]
    'update:visible': [value: boolean]
    action: []
    close: []
    show: []
    hide: []
    'menu-item-click': [item: MenuItem]
  }>()

  // 引用
  const triggerRef = ref<HTMLElement>()
  const popoverRef = ref<HTMLElement>()

  // 内部状态
  const internalVisible = ref(false)
  const position = ref({ x: 0, y: 0 })

  // 计算显示状态
  watch(
    [() => props.modelValue, () => props.visible],
    ([modelValue, visible]) => {
      internalVisible.value = modelValue || visible
    },
    { immediate: true }
  )

  // 计算弹出框样式
  const popoverStyle = computed(() => {
    // 如果传入了手动坐标，直接使用
    if (props.x !== undefined && props.y !== undefined) {
      return {
        left: `${props.x}px`,
        top: `${props.y}px`,
        width: typeof props.width === 'number' ? `${props.width}px` : props.width,
        maxWidth: typeof props.maxWidth === 'number' ? `${props.maxWidth}px` : props.maxWidth,
      }
    }

    // 否则使用计算的位置
    return {
      left: `${position.value.x}px`,
      top: `${position.value.y}px`,
      width: typeof props.width === 'number' ? `${props.width}px` : props.width,
      maxWidth: typeof props.maxWidth === 'number' ? `${props.maxWidth}px` : props.maxWidth,
    }
  })

  // 处理触发器点击
  const handleTriggerClick = () => {
    if (props.disabled || props.trigger !== 'click') return

    if (internalVisible.value) {
      hide()
    } else {
      show()
    }
  }

  // 显示弹出框
  const show = async () => {
    if (props.disabled) return

    internalVisible.value = true
    emit('update:modelValue', true)
    emit('update:visible', true)
    emit('show')

    await nextTick()
    updatePosition()
  }

  // 隐藏弹出框
  const hide = () => {
    internalVisible.value = false
    emit('update:modelValue', false)
    emit('update:visible', false)
    emit('close')
    emit('hide')
  }

  // 更新位置
  const updatePosition = () => {
    if (!triggerRef.value || !popoverRef.value) return

    const triggerRect = triggerRef.value.getBoundingClientRect()
    const popoverRect = popoverRef.value.getBoundingClientRect()

    let x = 0
    let y = 0

    // 根据 placement 计算位置
    switch (props.placement) {
      case 'bottom':
        x = triggerRect.left + triggerRect.width / 2 - popoverRect.width / 2
        y = triggerRect.bottom + props.offset
        break
      case 'bottom-start':
        x = triggerRect.left
        y = triggerRect.bottom + props.offset
        break
      case 'bottom-end':
        x = triggerRect.right - popoverRect.width
        y = triggerRect.bottom + props.offset
        break
      case 'top':
        x = triggerRect.left + triggerRect.width / 2 - popoverRect.width / 2
        y = triggerRect.top - popoverRect.height - props.offset
        break
      case 'top-start':
        x = triggerRect.left
        y = triggerRect.top - popoverRect.height - props.offset
        break
      case 'top-end':
        x = triggerRect.right - popoverRect.width
        y = triggerRect.top - popoverRect.height - props.offset
        break
      case 'left':
        x = triggerRect.left - popoverRect.width - props.offset
        y = triggerRect.top + triggerRect.height / 2 - popoverRect.height / 2
        break
      case 'right':
        x = triggerRect.right + props.offset
        y = triggerRect.top + triggerRect.height / 2 - popoverRect.height / 2
        break
    }

    // 边界检查
    const padding = 8
    x = Math.max(padding, Math.min(x, window.innerWidth - popoverRect.width - padding))
    y = Math.max(padding, Math.min(y, window.innerHeight - popoverRect.height - padding))

    position.value = { x, y }
  }

  // 处理弹出框点击
  const handlePopoverClick = (e: Event) => {
    e.stopPropagation()
    if (props.closeOnClickInside) {
      hide()
    }
  }

  // 处理遮罩点击
  const handleMaskClick = () => {
    if (props.closeOnClickOutside) {
      hide()
    }
  }

  // 处理菜单项点击
  const handleMenuItemClick = (item: MenuItem) => {
    if (item.disabled) return

    emit('menu-item-click', item)
    if (item.onClick) {
      item.onClick()
    }

    // 兼容旧版本
    if (item.label === props.actionText) {
      emit('action')
    }

    if (props.closeOnClickInside) {
      hide()
    }
  }

  // 处理外部点击
  const handleClickOutside = (e: Event) => {
    if (!props.closeOnClickOutside || !internalVisible.value) return

    const target = e.target as Node
    if (triggerRef.value?.contains(target) || popoverRef.value?.contains(target)) {
      return
    }

    hide()
  }

  // 生命周期
  onMounted(() => {
    document.addEventListener('click', handleClickOutside)
    window.addEventListener('resize', updatePosition)
    window.addEventListener('scroll', updatePosition)
  })

  onUnmounted(() => {
    document.removeEventListener('click', handleClickOutside)
    window.removeEventListener('resize', updatePosition)
    window.removeEventListener('scroll', updatePosition)
  })

  // 监听位置变化
  watch(internalVisible, visible => {
    if (visible) {
      nextTick(updatePosition)
    }
  })

  // 兼容旧版本 - 如果传入了 actionText，自动生成菜单项
  const computedMenuItems = computed(() => {
    if (props.actionText && props.menuItems.length === 0) {
      return [{ label: props.actionText, value: 'action' }]
    }
    return props.menuItems
  })

  // 使用计算后的菜单项
  const menuItems = computedMenuItems
</script>

<style scoped>
  /* 弹出框组件样式 - 使用全局主题变量 */
  .popover-trigger {
    display: inline-block;
    cursor: pointer;
  }

  .default-trigger {
    padding: var(--spacing-sm) var(--spacing-lg);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius);
    background: var(--bg-400);
    color: var(--text-200);
    font-family: var(--font-family);
    font-size: var(--font-size-md);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .default-trigger:hover {
    border-color: var(--color-primary);
    color: var(--color-primary);
    background: var(--color-hover);
  }

  .default-trigger:focus {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px var(--color-primary-alpha);
    outline: none;
  }

  .popover {
    position: fixed;
    z-index: 1000;
    background: var(--bg-400);
    border: 1px solid var(--border-300);
    border-radius: 6px;
    box-shadow: var(--shadow-sm);
    padding: 4px 6px; /* 上下更紧凑，左右留少量内边距 */
    min-width: 100px;
    max-width: 260px;
    font-family: var(--font-family);
    font-size: 14px;
    line-height: 1.5;
    color: var(--text-200);
    -webkit-app-region: no-drag; /* 防止标题栏拖拽区域拦截交互 */
  }

  .popover-content {
    position: relative;
    z-index: 1;
  }

  .popover-menu {
    padding: 0; /* 简化：去掉额外上下留白，让分割线与项目紧贴 */
  }

  .popover-menu-item {
    display: flex;
    align-items: center;
    padding: 4px 6px; /* 更紧凑，和容器左右内边距对齐 */
    cursor: pointer;
    border-radius: 0; /* 去圆角以保证分割线简洁笔直 */
    margin: 0; /* 去掉左右额外留白 */
    transition: all 0.2s ease;
    color: var(--text-200);
  }

  /* 分割线：相邻菜单项之间添加上边框 */
  .popover-menu-item + .popover-menu-item {
    border-top: 1px solid var(--border-300); /* 最简单的分割线 */
  }

  .popover-menu-item:hover:not(.popover-menu-item--disabled) {
    background-color: var(--color-hover);
    color: var(--text-200);
  }

  .popover-menu-item:focus {
    background-color: var(--color-hover);
    outline: none;
  }

  .popover-menu-item--disabled {
    opacity: 0.5;
    cursor: not-allowed;
    color: var(--text-500);
  }

  .menu-item-icon {
    width: 16px;
    height: 16px;
    margin-right: var(--spacing-sm);
    flex-shrink: 0;
    color: var(--text-400);
  }

  .menu-item-text {
    flex: 1;
    color: inherit;
    font-size: inherit;
  }

  .popover-mask {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 999;
    background: transparent;
  }

  /* 响应式设计 */
  @media (max-width: 768px) {
    .popover {
      max-width: calc(100vw - 32px);
    }
  }

  /* 减少动画模式支持 */
  @media (prefers-reduced-motion: reduce) {
    .default-trigger,
    .popover-menu-item {
      transition: none;
    }
  }

  /* 高对比度模式支持 */
  @media (prefers-contrast: high) {
    .popover {
      border-width: 2px;
    }

    .default-trigger {
      border-width: 2px;
    }
  }
</style>
