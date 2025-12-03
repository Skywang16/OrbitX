<script setup lang="ts">
  import { ref } from 'vue'
  import ButtonGroup from '@/components/ui/ButtonGroup.vue'
  import TabBar from '@/components/ui/TabBar.vue'
  import type { AnyTabItem } from '@/types'
  import { getCurrentWindow } from '@tauri-apps/api/window'

  interface Props {
    tabs: AnyTabItem[]
    activeTabId: number | null
  }

  interface Emits {
    (e: 'switch', tabId: number): void
    (e: 'close', tabId: number): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()
  const DOUBLE_CLICK_TIME = 500
  const DRAG_THRESHOLD = 2

  const lastClickTime = ref(0)
  const lastClickPos = ref({ x: 0, y: 0 })
  const mouseDownPos = ref({ x: 0, y: 0 })
  const isDragging = ref(false)
  const canDrag = ref(false)

  const startDrag = async () => {
    await getCurrentWindow().startDragging()
  }

  const isInteractiveElement = (target: HTMLElement): boolean => {
    return (
      target.tagName === 'BUTTON' ||
      target.closest('button') !== null ||
      target.classList.contains('tab') ||
      target.closest('.tab') !== null ||
      target.closest('.window-controls-container') !== null
    )
  }

  const handleMouseDown = (event: MouseEvent) => {
    const target = event.target as HTMLElement
    canDrag.value = !isInteractiveElement(target)
    mouseDownPos.value = { x: event.clientX, y: event.clientY }
    isDragging.value = false
  }

  const handleMouseMove = (event: MouseEvent) => {
    if (canDrag.value && !isDragging.value) {
      const distance = Math.sqrt(
        Math.pow(event.clientX - mouseDownPos.value.x, 2) + Math.pow(event.clientY - mouseDownPos.value.y, 2)
      )

      if (distance > DRAG_THRESHOLD) {
        isDragging.value = true
        startDrag()
      }
    }
  }

  const handleMouseUp = async (event: MouseEvent) => {
    if (isDragging.value) {
      isDragging.value = false
      return
    }

    const target = event.target as HTMLElement
    if (isInteractiveElement(target)) return

    const currentTime = performance.now()
    const currentPos = { x: event.clientX, y: event.clientY }
    const timeDiff = currentTime - lastClickTime.value

    const distanceFromLastClick = Math.sqrt(
      Math.pow(currentPos.x - lastClickPos.value.x, 2) + Math.pow(currentPos.y - lastClickPos.value.y, 2)
    )
    const distanceFromMouseDown = Math.sqrt(
      Math.pow(currentPos.x - mouseDownPos.value.x, 2) + Math.pow(currentPos.y - mouseDownPos.value.y, 2)
    )

    if (
      timeDiff < DOUBLE_CLICK_TIME &&
      distanceFromLastClick < DRAG_THRESHOLD &&
      distanceFromMouseDown < DRAG_THRESHOLD
    ) {
      const window = getCurrentWindow()
      const isMaximized = await window.isMaximized()
      isMaximized ? await window.unmaximize() : await window.maximize()

      lastClickTime.value = 0
      lastClickPos.value = { x: 0, y: 0 }
    } else {
      lastClickTime.value = currentTime
      lastClickPos.value = currentPos
    }
  }
</script>

<template>
  <div
    class="title-bar"
    data-tauri-drag-region
    @mousedown="handleMouseDown"
    @mousemove="handleMouseMove"
    @mouseup="handleMouseUp"
  >
    <div class="left-buttons-space"></div>

    <div class="tab-bar-container" data-tauri-drag-region="false">
      <TabBar
        :tabs="props.tabs"
        :activeTabId="activeTabId"
        @switch="emit('switch', $event)"
        @close="emit('close', $event)"
      />
    </div>

    <div class="window-controls-container">
      <ButtonGroup />
    </div>
  </div>
</template>

<style scoped>
  .title-bar {
    display: flex;
    align-items: center;
    height: var(--titlebar-height);
    background-color: var(--bg-200);
    cursor: default;
    border-bottom: 1px solid var(--border-200);
    user-select: none;
    -webkit-user-select: none;
  }

  .left-buttons-space {
    width: 80px;
    height: var(--titlebar-element-height);
    cursor: default;
  }

  .tab-bar-container {
    flex: 1;
    min-width: 0;
  }

  .window-controls-container {
    flex-shrink: 0;
    margin-left: auto;
    margin-right: var(--spacing-sm);
  }
</style>
