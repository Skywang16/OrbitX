<script setup lang="ts">
  import { onBeforeUnmount, onMounted, ref } from 'vue'
  import ButtonGroup from '@/components/ui/ButtonGroup.vue'
  import EditorHeader from '@/components/editor/EditorHeader.vue'
  import { getCurrentWindow } from '@tauri-apps/api/window'

  const DOUBLE_CLICK_TIME = 500
  const DRAG_THRESHOLD = 2

  const lastClickTime = ref(0)
  const lastClickPos = ref({ x: 0, y: 0 })
  const mouseDownPos = ref({ x: 0, y: 0 })
  const isDragging = ref(false)
  const canDrag = ref(false)

  const controlsRef = ref<HTMLElement | null>(null)
  const rightGutterWidth = ref(0)
  let resizeObserver: ResizeObserver | null = null

  const LEFT_GUTTER_WIDTH = 80

  const updateRightGutterWidth = () => {
    const el = controlsRef.value
    if (!el) return
    rightGutterWidth.value = Math.ceil(el.getBoundingClientRect().width) + 12
  }

  onMounted(() => {
    updateRightGutterWidth()
    const el = controlsRef.value
    if (!el) return
    resizeObserver = new ResizeObserver(() => updateRightGutterWidth())
    resizeObserver.observe(el)
  })

  onBeforeUnmount(() => {
    resizeObserver?.disconnect()
    resizeObserver = null
  })

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
    <div
      class="title-bar__tabs"
      :style="{
        '--orbitx-titlebar-left-gutter': `${LEFT_GUTTER_WIDTH}px`,
        '--orbitx-titlebar-right-gutter': `${rightGutterWidth}px`,
      }"
    >
      <EditorHeader />
    </div>

    <div ref="controlsRef" class="window-controls-container">
      <ButtonGroup />
    </div>
  </div>
</template>

<style scoped>
  .title-bar {
    display: flex;
    align-items: center;
    position: relative;
    align-items: stretch;
    background-color: var(--bg-200);
    cursor: default;
    border-bottom: 1px solid var(--border-200);
    user-select: none;
    -webkit-user-select: none;
  }

  .title-bar__tabs {
    flex: 1;
    min-width: 0;
    height: 100%;
    overflow: hidden;
  }

  .window-controls-container {
    position: absolute;
    top: 0;
    right: var(--spacing-sm);
    height: var(--titlebar-height);
    display: flex;
    align-items: center;
  }
</style>
