<script setup lang="ts">
  interface Props {
    visible: boolean
  }

  interface Emits {
    (e: 'update:visible', value: boolean): void
  }

  defineProps<Props>()
  const emit = defineEmits<Emits>()

  const handleClose = () => {
    emit('update:visible', false)
  }

  const handleOverlayClick = (e: MouseEvent) => {
    if (e.target === e.currentTarget) {
      handleClose()
    }
  }
</script>

<template>
  <Transition name="slide-up" appear>
    <div v-if="visible" class="popover-wrapper">
      <div class="popover-content" @click.stop>
        <slot />
      </div>
      <div class="popover-overlay" @click="handleOverlayClick"></div>
    </div>
  </Transition>
</template>

<style scoped>
  .popover-wrapper {
    position: absolute;
    bottom: 100%;
    left: 0;
    right: 0;
    margin-bottom: 12px;
    z-index: 999;
  }

  .popover-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: transparent;
    z-index: -1;
  }

  .popover-content {
    max-height: 70vh;
    background: var(--bg-400);
    border-radius: var(--border-radius-md);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.4);
    overflow-y: auto;
    box-sizing: border-box;
  }

  .slide-up-enter-active,
  .slide-up-leave-active {
    transition:
      opacity 0.2s ease,
      transform 0.2s ease;
  }

  .slide-up-enter-from {
    opacity: 0;
    transform: translateY(8px);
  }

  .slide-up-leave-to {
    opacity: 0;
    transform: translateY(8px);
  }
</style>
