<script setup lang="ts">
  import AISettings from '@/components/settings/components/AI/AISettings.vue'
  import ThemeSettings from '@/components/settings/components/Theme/ThemeSettings.vue'
  import ShortcutSettings from '@/components/settings/components/Shortcuts/ShortcutSettings.vue'
  import { LanguageSettings } from '@/components/settings/components/Language'
  import SettingsNav from '@/components/settings/SettingsNav.vue'
  import { useSettingsStore } from '@/components/settings/store'
  import { onMounted } from 'vue'

  const settingsStore = useSettingsStore()

  onMounted(async () => {
    settingsStore.setActiveSection('ai')
    await settingsStore.initializeSettings()
  })

  const handleNavigationChange = (section: string) => {
    settingsStore.setActiveSection(section)
  }
</script>

<template>
  <div class="settings-container">
    <div class="settings-content">
      <!-- 左侧导航 -->
      <div class="settings-sidebar">
        <SettingsNav :activeSection="settingsStore.activeSection" @change="handleNavigationChange" />
      </div>

      <!-- 右侧内容区域 -->
      <div class="settings-main">
        <div class="settings-panel">
          <AISettings v-if="settingsStore.activeSection === 'ai'" />
          <ThemeSettings v-else-if="settingsStore.activeSection === 'theme'" />
          <ShortcutSettings v-else-if="settingsStore.activeSection === 'shortcuts'" />
          <LanguageSettings v-else-if="settingsStore.activeSection === 'language'" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped></style>
