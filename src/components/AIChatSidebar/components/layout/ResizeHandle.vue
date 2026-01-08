<script setup lang="ts">
  interface Props {
    isDragging?: boolean
    isHovering?: boolean
    side?: 'left' | 'right'
  }

  interface Emits {
    (e: 'mousedown', event: MouseEvent): void
    (e: 'mouseenter'): void
    (e: 'mouseleave'): void
    (e: 'dblclick'): void
  }

  withDefaults(defineProps<Props>(), {
    isDragging: false,
    isHovering: false,
    side: 'left',
  })

  const emit = defineEmits<Emits>()

  const handleMouseDown = (event: MouseEvent) => {
    emit('mousedown', event)
  }

  const handleMouseEnter = () => {
    emit('mouseenter')
  }

  const handleMouseLeave = () => {
    emit('mouseleave')
  }

  const handleDoubleClick = () => {
    emit('dblclick')
  }
</script>

<template>
  <div
    class="resize-handle"
    :class="{
      'resize-handle--dragging': isDragging,
      'resize-handle--hovering': isHovering,
      'resize-handle--right': side === 'right',
    }"
    @mousedown="handleMouseDown"
    @mouseenter="handleMouseEnter"
    @mouseleave="handleMouseLeave"
    @dblclick="handleDoubleClick"
  />
</template>

<style scoped>
  .resize-handle {
    position: absolute;
    left: -2px;
    top: 0;
    width: 4px;
    height: 100%;
    cursor: col-resize;
    z-index: 10;
    background: transparent;
  }

  .resize-handle--right {
    left: auto;
    right: -2px;
  }

  .resize-handle:hover {
    background: var(--color-primary-alpha);
  }

  .resize-handle--dragging {
    background: var(--color-primary-alpha);
    opacity: 2;
  }

  .resize-handle__indicator {
    display: none;
  }

  .resize-handle--dragging {
    position: fixed !important;
    left: 0 !important;
    right: 0 !important;
    top: 0 !important;
    bottom: 0 !important;
    width: 100vw !important;
    height: 100vh !important;
    background: transparent !important;
    cursor: col-resize !important;
    z-index: 9999 !important;
  }
</style>
