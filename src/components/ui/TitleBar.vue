<script setup lang="ts">
  import ButtonGroup from '@/components/ui/ButtonGroup.vue'
  import TabBar from '@/components/ui/TabBar.vue'
  import type { TabItem } from '@/types'
  import { getCurrentWindow } from '@tauri-apps/api/window'

  interface Props {
    tabs: TabItem[]
    activeTabId: string | null
  }

  interface Emits {
    (e: 'switch', id: string): void
    (e: 'close', id: string): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()

  // 开始拖拽窗口
  const startDrag = async () => {
    await getCurrentWindow().startDragging()
  }
</script>

<template>
  <div class="title-bar" data-tauri-drag-region @mousedown="startDrag">
    <!-- 左侧按钮区域预留空间 -->
    <div class="left-buttons-space"></div>

    <!-- 中间标签栏区域 -->
    <div class="tab-bar-container" data-tauri-drag-region="false">
      <TabBar
        :tabs="props.tabs"
        :activeTabId="activeTabId"
        @switch="emit('switch', $event)"
        @close="emit('close', $event)"
      />
    </div>

    <!-- 右侧窗口控制按钮 -->
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
