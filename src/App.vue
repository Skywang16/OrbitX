<!--
Copyright (C) 2025 OrbitX Contributors

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.
-->

<script setup lang="ts">
  import TerminalView from '@/views/Terminal/TerminalView.vue'
  import { useShortcutListener } from '@/shortcuts'
  import { onMounted } from 'vue'

  // 初始化快捷键监听器
  const { reloadConfig } = useShortcutListener()

  onMounted(() => {
    // 暴露重新加载配置功能到控制台
    ;(window as any).reloadShortcuts = reloadConfig
  })
</script>

<template>
  <div class="app-layout">
    <!-- 直接显示终端视图，所有内容都通过标签系统管理 -->
    <TerminalView />
  </div>
</template>

<style>
  :root {
    font-family: var(--font-family);
    font-size: var(--font-size-lg);
    font-weight: 400;
    color: var(--text-300);
    background-color: var(--bg-200);

    font-synthesis: none;
    text-rendering: optimizeLegibility;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    -webkit-text-size-adjust: 100%;
  }

  * {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  body,
  html {
    height: 100%;
    overflow: hidden;
  }

  #app {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }

  .app-layout {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
</style>
