<script setup lang="ts">
  import AISettings from '@/components/settings/components/AI/AISettings.vue'
  import ConfigSettings from '@/components/settings/components/Config/ConfigSettings.vue'
  import ThemeSettings from '@/components/settings/components/Theme/ThemeSettings.vue'
  import SettingsNav from '@/components/settings/SettingsNav.vue'
  import { useSettingsStore } from '@/components/settings/store'
  import { onMounted } from 'vue'

  const settingsStore = useSettingsStore()

  // 组件挂载时设置设置页面为打开状态并初始化设置
  onMounted(async () => {
    settingsStore.openSettings()
    // 初始化所有设置
    await settingsStore.initializeSettings()
  })

  // 处理导航项切换
  const handleNavigationChange = (section: string) => {
    settingsStore.setActiveSection(section)
  }
</script>

<template>
  <div class="settings-view">
    <!-- 设置页面主体 -->
    <div class="settings-content">
      <!-- 左侧导航 -->
      <div class="settings-sidebar">
        <SettingsNav :activeSection="settingsStore.activeSection" @change="handleNavigationChange" />
      </div>

      <!-- 右侧内容区域 -->
      <div class="settings-main">
        <div class="settings-panel">
          <!-- 根据当前选中的设置项显示对应组件 -->
          <ThemeSettings v-if="settingsStore.activeSection === 'theme'" />
          <ConfigSettings v-if="settingsStore.activeSection === 'config'" />
          <AISettings v-if="settingsStore.activeSection === 'ai'" />

          <!-- 默认显示主题设置 -->
          <ThemeSettings v-if="!settingsStore.activeSection" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .settings-view {
    height: 100vh;
    background-color: var(--color-background);
    overflow: hidden;
  }

  .settings-content {
    display: flex;
    height: 100%;
  }

  .settings-sidebar {
    width: 280px;
    background-color: var(--color-background-secondary);
    border-right: 1px solid var(--color-border);
    overflow-y: auto;
  }

  .settings-main {
    flex: 1;
    overflow-y: auto;
    background-color: var(--color-background);
  }

  .settings-panel {
    height: 100%;
  }
</style>
