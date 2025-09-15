<script setup lang="ts">
  import { ref, watch, nextTick, onMounted } from 'vue'
  interface Props {
    visible: boolean
    targetRef?: HTMLElement | null
  }

  interface Emits {
    (e: 'update:visible', value: boolean): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()

  const top = ref(0)
  const left = ref(0)
  const offset = 12
  const contentWidth = ref(0)

  const handleClose = () => {
    emit('update:visible', false)
  }

  const handleOverlayClick = (e: MouseEvent) => {
    if (e.target === e.currentTarget) {
      handleClose()
    }
  }

  const updatePosition = () => {
    if (!props.visible || !props.targetRef) return
    const rect = props.targetRef.getBoundingClientRect()
    left.value = rect.left + rect.width / 2
    top.value = rect.top - offset
    contentWidth.value = rect.width
  }

  watch(
    () => props.visible,
    async val => {
      if (val) {
        await nextTick()
        updatePosition()
      }
    }
  )

  onMounted(() => {
    if (props.visible) {
      updatePosition()
    }
  })
</script>

<template>
  <Teleport to="body">
    <Transition name="slide-up" appear>
      <div
        v-if="visible"
        class="popover-content"
        :style="{ top: top + 'px', left: left + 'px', width: contentWidth + 'px' }"
        @click.stop
      >
        <slot />
      </div>
    </Transition>
    <div v-if="visible" class="popover-overlay" @click="handleOverlayClick"></div>
  </Teleport>
</template>

<style scoped>
  .popover-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: transparent;
    z-index: 998;
  }

  .popover-content {
    position: absolute;
    transform: translate(-50%, -100%);
    max-height: 400px;
    background: var(--bg-200);
    border-radius: var(--border-radius-md);
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.08);
    border: 1px solid var(--border-200);
    overflow: hidden;
    box-sizing: border-box;
    z-index: 999;
  }

  .slide-up-enter-active,
  .slide-up-leave-active {
    transition:
      opacity 0.2s ease,
      transform 0.2s ease;
  }

  .slide-up-enter-from {
    opacity: 0;
    transform: translate(-50%, -100%) translateY(8px);
  }

  .slide-up-leave-to {
    opacity: 0;
    transform: translate(-50%, -100%) translateY(8px);
  }
</style>
