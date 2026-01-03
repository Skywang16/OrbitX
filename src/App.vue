<script setup lang="ts">
  import TerminalView from '@/views/Terminal/TerminalView.vue'
  import { OnboardingView } from '@/views/Onboarding'
  import { useShortcutListener } from '@/shortcuts'
  import { useWindowOpacity } from '@/composables/useWindowOpacity'
  import { useMenuEvents } from '@/composables/useMenuEvents'
  import { appApi, workspaceApi } from '@/api'
  import { useTabManagerStore } from '@/stores/TabManager'
  import { useSessionStore } from '@/stores/session'
  import { storeToRefs } from 'pinia'
  import { computed, onMounted, onUnmounted } from 'vue'
  import type { UnlistenFn } from '@tauri-apps/api/event'

  const { reloadConfig } = useShortcutListener()
  const tabManager = useTabManagerStore()
  const sessionStore = useSessionStore()
  const { uiState } = storeToRefs(sessionStore)

  // 初始化透明度管理
  useWindowOpacity()

  // 初始化菜单事件监听
  useMenuEvents()

  const showOnboarding = computed(() => uiState.value.onboardingCompleted !== true)

  const handleOnboardingComplete = () => {
    sessionStore.updateUiState({ onboardingCompleted: true })
  }

  // 测试按钮：重新打开引导页面
  const showOnboardingForTesting = () => {
    sessionStore.updateUiState({ onboardingCompleted: false })
  }

  // 开发环境下暴露到全局
  if (import.meta.env.DEV) {
    ;(window as typeof window & { showOnboarding?: () => void }).showOnboarding = showOnboardingForTesting
  }

  let unlistenClearTabs: UnlistenFn | undefined

  onMounted(async () => {
    ;(window as typeof window & { reloadShortcuts?: () => void }).reloadShortcuts = reloadConfig

    // 后台维护工作区数据
    workspaceApi.maintainWorkspaces()

    // 监听清空所有标签页的事件（macOS 窗口关闭时触发）
    unlistenClearTabs = await appApi.onClearAllTabs(async () => {
      await tabManager.closeAllTabs()
    })
  })

  onUnmounted(() => {
    if (unlistenClearTabs) {
      unlistenClearTabs()
    }
  })
</script>

<template>
  <div class="app-layout">
    <Transition name="fade-scale" mode="out-in">
      <OnboardingView v-if="showOnboarding" key="onboarding" @complete="handleOnboardingComplete" />
      <TerminalView v-else key="terminal" />
    </Transition>
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

  /* Onboarding → Terminal 过渡动画 */
  .fade-scale-enter-active,
  .fade-scale-leave-active {
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .fade-scale-enter-from {
    opacity: 0;
    transform: scale(0.98);
  }

  .fade-scale-leave-to {
    opacity: 0;
    transform: scale(1.02);
  }
</style>
