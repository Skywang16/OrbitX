<script setup lang="ts">
  import { ref } from 'vue'
  import ButtonGroup from '@/components/ui/ButtonGroup.vue'
  import TabBar from '@/components/ui/TabBar.vue'
  import type { TabItem } from '@/types'
  import { getCurrentWindow } from '@tauri-apps/api/window'

  interface Props {
    tabs: TabItem[]
    activeTabId: number | string | null
  }

  interface Emits {
    (e: 'switch', id: number | string): void
    (e: 'close', id: number | string): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()

  // 开始拖拽窗口
  const startDrag = async () => {
    await getCurrentWindow().startDragging()
  }

  // 双击检测状态
  const DOUBLE_CLICK_TIME = 500 // 双击时间窗口（毫秒）
  const DRAG_THRESHOLD = 2 // 拖拽判断阈值（像素）

  const lastClickTime = ref(0)
  const lastClickPos = ref({ x: 0, y: 0 })
  const mouseDownPos = ref({ x: 0, y: 0 })
  const isDragging = ref(false)

  // 处理 mousedown 事件
  const handleMouseDown = (event: MouseEvent) => {
    mouseDownPos.value = { x: event.clientX, y: event.clientY }
    isDragging.value = false
  }

  // 处理 mousemove 事件，检测是否开始拖拽
  const handleMouseMove = (event: MouseEvent) => {
    if (!isDragging.value) {
      const distanceFromMouseDown = Math.sqrt(
        Math.pow(event.clientX - mouseDownPos.value.x, 2) + Math.pow(event.clientY - mouseDownPos.value.y, 2)
      )

      // 如果移动超过阈值，开始拖拽
      if (distanceFromMouseDown > DRAG_THRESHOLD) {
        isDragging.value = true
        startDrag()
      }
    }
  }

  // 处理 mouseup 事件，检测是否为有效双击
  const handleMouseUp = async (event: MouseEvent) => {
    // 如果正在拖拽，不处理双击
    if (isDragging.value) {
      isDragging.value = false
      return
    }

    const currentTime = performance.now()
    const currentPos = { x: event.clientX, y: event.clientY }

    // 计算时间差和位置偏移
    const timeDiff = currentTime - lastClickTime.value
    const distanceFromLastClick = Math.sqrt(
      Math.pow(currentPos.x - lastClickPos.value.x, 2) + Math.pow(currentPos.y - lastClickPos.value.y, 2)
    )
    const distanceFromMouseDown = Math.sqrt(
      Math.pow(currentPos.x - mouseDownPos.value.x, 2) + Math.pow(currentPos.y - mouseDownPos.value.y, 2)
    )

    // 判断是否为有效双击：
    // 1. 时间间隔在双击时间窗口内
    // 2. 两次点击位置接近（没有移动太多）
    // 3. mouseup 位置接近 mousedown 位置（没有拖拽）
    if (
      timeDiff < DOUBLE_CLICK_TIME &&
      distanceFromLastClick < DRAG_THRESHOLD &&
      distanceFromMouseDown < DRAG_THRESHOLD
    ) {
      // 有效双击，触发全屏切换
      const window = getCurrentWindow()
      const isMaximized = await window.isMaximized()

      if (isMaximized) {
        await window.unmaximize()
      } else {
        await window.maximize()
      }

      // 重置状态，避免三击触发
      lastClickTime.value = 0
      lastClickPos.value = { x: 0, y: 0 }
    } else {
      // 不是双击，更新最后点击时间和位置
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
