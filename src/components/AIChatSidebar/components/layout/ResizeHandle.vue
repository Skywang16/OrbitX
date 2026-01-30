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
</script>

<template>
  <div
    class="resize-handle"
    :class="{
      'resize-handle--active': isDragging || isHovering,
      'resize-handle--right': side === 'right',
    }"
    @mousedown.stop.prevent="emit('mousedown', $event)"
    @dragstart.prevent
    @mouseenter="emit('mouseenter')"
    @mouseleave="emit('mouseleave')"
    @dblclick.stop.prevent="emit('dblclick')"
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
    touch-action: none;
  }

  .resize-handle--right {
    left: auto;
    right: -2px;
  }

  .resize-handle--active {
    background: var(--color-primary);
  }
</style>
