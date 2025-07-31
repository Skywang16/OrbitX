<script setup lang="ts">
  import { computed } from 'vue'
  import { useRoute, useRouter } from 'vue-router'

  const route = useRoute()
  const router = useRouter()

  // 判断是否需要显示顶部安全区
  const needsSafeArea = computed(() => {
    return route.name === 'Settings'
  })

  // 获取当前页面标题
  const pageTitle = computed(() => {
    return route.meta?.title || '设置'
  })

  // 返回终端页面
  const goBack = () => {
    router.push('/')
  }
</script>

<template>
  <div class="app-layout">
    <!-- 顶部安全区 - 仅在设置页面显示 -->
    <div v-if="needsSafeArea" class="top-safe-area" data-tauri-drag-region>
      <button class="back-button" @click="goBack" data-tauri-drag-region="false" title="返回终端">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M19 12H5M12 19l-7-7 7-7" />
        </svg>
      </button>
      <div class="page-title">{{ pageTitle }}</div>
    </div>

    <!-- 路由视图 -->
    <div class="router-content" :class="{ 'with-safe-area': needsSafeArea }">
      <router-view v-slot="{ Component }">
        <keep-alive>
          <component :is="Component" />
        </keep-alive>
      </router-view>
    </div>
  </div>
</template>

<style>
  :root {
    font-family: var(--font-family);
    font-size: var(--font-size-lg);
    font-weight: 400;
    color: var(--text-secondary);
    background-color: var(--color-background);

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

  .top-safe-area {
    height: var(--titlebar-height);
    background-color: transparent;
    flex-shrink: 0;
    cursor: default;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    padding: 0 var(--spacing-md);
  }

  .back-button {
    position: absolute;
    left: 80px;
    display: flex;
    align-items: center;
    padding: var(--spacing-xs) var(--spacing-sm);
    background: transparent;
    border: 1px solid var(--border-color);
    border-radius: var(--border-radius);
    color: var(--text-primary);
    cursor: pointer;
    font-size: var(--font-size-sm);
    transition: all 0.2s ease;
  }

  .back-button:hover {
    background-color: var(--color-background-hover);
    border-color: var(--border-color-hover);
  }

  .page-title {
    font-size: var(--font-size-md);
    font-weight: 600;
    color: var(--text-primary);
    text-align: center;
    pointer-events: none;
  }

  .router-content {
    flex: 1;
    min-height: 0;
  }

  .router-content.with-safe-area {
    height: calc(100vh - var(--titlebar-height));
  }
</style>
