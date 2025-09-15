<script setup lang="ts">
  import TerminalView from '@/views/Terminal/TerminalView.vue'
  import { OnboardingView } from '@/components/Onboarding'
  import { useShortcutListener } from '@/shortcuts'
  import { createStorage } from '@/utils/storage'
  import { onMounted, ref } from 'vue'

  const { reloadConfig } = useShortcutListener()

  // 首次启动状态管理
  const onboardingStorage = createStorage<boolean>('orbitx-onboarding-completed')
  const showOnboarding = ref(!onboardingStorage.exists())

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
    window.showOnboarding = showOnboardingForTesting
  }

  onMounted(() => {
    window.reloadShortcuts = reloadConfig
  })
</script>

<template>
  <div class="app-layout">
    <OnboardingView v-if="showOnboarding" @complete="handleOnboardingComplete" />
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
