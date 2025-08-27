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
  import { OnboardingView } from '@/components/Onboarding'
  import { useShortcutListener } from '@/shortcuts'
  import { createStorage } from '@/utils/storage'
  import { onMounted, ref } from 'vue'

  const { reloadConfig } = useShortcutListener()

  // 首次启动状态管理
  const showOnboarding = ref(false)
  const onboardingStorage = createStorage<boolean>('orbitx-onboarding-completed')

  const handleOnboardingComplete = () => {
    onboardingStorage.save(true)
    showOnboarding.value = false
  }

  // 测试按钮：重新打开引导页面
  const showOnboardingForTesting = () => {
    onboardingStorage.remove()
    showOnboarding.value = true
  }

  // 开发环境下暴露到全局
  if (import.meta.env.DEV) {
    ;(window as any).showOnboarding = showOnboardingForTesting
  }

  onMounted(() => {
    ;(window as any).reloadShortcuts = reloadConfig

    // 检查是否是首次启动
    const hasCompletedOnboarding = onboardingStorage.exists()
    showOnboarding.value = !hasCompletedOnboarding
  })
</script>

<template>
  <div class="app-layout">
    <!-- 引导页面 -->
    <OnboardingView v-if="showOnboarding" @complete="handleOnboardingComplete" />

    <!-- 主应用界面 -->
    <TerminalView v-else />
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
