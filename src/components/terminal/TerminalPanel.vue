<script setup lang="ts">
  import { useTerminalStore } from '@/stores/Terminal'
  import type { TerminalSession } from '@/types'
  import { computed } from 'vue'
  import Terminal from './Terminal.vue'

  interface Props {
    panelId: 'main' | 'left' | 'right'
    terminalIds: string[]
    activeTerminalId: string | null
  }

  const props = defineProps<Props>()

  const terminalStore = useTerminalStore()

  // 计算当前面板的终端列表
  const terminals = computed(() => {
    return props.terminalIds
      .map(id => terminalStore.terminals.find(t => t.id === id))
      .filter((terminal): terminal is TerminalSession => terminal !== undefined)
  })

  // --- 事件处理器 ---
  // 处理终端输入
  const handleTerminalInput = (id: string, data: string) => {
    terminalStore.writeToTerminal(id, data)
  }

  // 处理终端大小调整
  const handleTerminalResize = (id: string, rows: number, cols: number) => {
    terminalStore.resizeTerminal(id, rows, cols)
  }
</script>

<template>
  <div class="terminal-panel">
    <div class="panel-content">
      <Terminal
        v-for="terminal in terminals"
        v-show="terminal.id === props.activeTerminalId"
        :key="terminal.id"
        :terminalId="terminal.id"
        :backendId="terminal.backendId"
        :isActive="terminal.id === props.activeTerminalId"
        @input="(data: string) => handleTerminalInput(terminal.id, data)"
        @resize="(rows: number, cols: number) => handleTerminalResize(terminal.id, rows, cols)"
      />
    </div>
  </div>
</template>

<style scoped>
  .terminal-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background-color: var(--color-background);
  }

  .panel-content {
    flex: 1;
    min-height: 0;
    height: 100%;
  }
</style>
