<script setup lang="ts">
  import ButtonGroup from '@/components/ui/ButtonGroup.vue'
  import TabBar from '@/components/ui/TabBar.vue'
  import { useTerminalStore } from '@/stores/Terminal'
  import type { RuntimeTerminalSession } from '@/stores/Terminal'
  import type { TabItem } from '@/types'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { computed } from 'vue'

  interface Props {
    terminals: RuntimeTerminalSession[]
    activeTerminalId: string | null
  }

  interface Emits {
    (e: 'switch', id: string): void
    (e: 'close', id: string): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()
  const terminalStore = useTerminalStore()

  // 转换终端数据为标签数据
  const tabs = computed<TabItem[]>(() =>
    props.terminals.map(terminal => ({
      id: terminal.id,
      title: terminal.title,
      isActive: terminal.id === props.activeTerminalId,
      closable: true,
    }))
  )

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
        :tabs="tabs"
        :activeTabId="activeTerminalId"
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
    background-color: var(--color-background);
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
