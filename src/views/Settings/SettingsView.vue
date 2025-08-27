<script setup lang="ts">
  import { ref } from 'vue'
  import { useI18n } from 'vue-i18n'
  import AISettings from '@/components/settings/components/AI/AISettings.vue'
  import ThemeSettings from '@/components/settings/components/Theme/ThemeSettings.vue'
  import ShortcutSettings from '@/components/settings/components/Shortcuts/ShortcutSettings.vue'
  import { LanguageSettings } from '@/components/settings/components/Language'
  import SettingsNav from '@/components/settings/SettingsNav.vue'
  import { useSettingsStore } from '@/components/settings/store'
  import { createMessage, XButton } from '@/ui'
  import { configApi } from '@/api/config'
  import { onMounted } from 'vue'

  const { t } = useI18n()
  const settingsStore = useSettingsStore()
  const isOpeningFolder = ref(false)

  onMounted(async () => {
    settingsStore.setActiveSection('ai')
    await settingsStore.initializeSettings()
  })

  const handleNavigationChange = (section: string) => {
    settingsStore.setActiveSection(section)
  }

  const handleOpenConfigFolder = async () => {
    if (isOpeningFolder.value) return

    isOpeningFolder.value = true
    try {
      await configApi.openConfigFolder()
      createMessage.success(t('settings.general.config_folder_opened'))
    } catch (error) {
      console.error('Failed to open config folder:', error)
      createMessage.error(t('settings.general.config_folder_error'))
    } finally {
      isOpeningFolder.value = false
    }
  }
</script>

<template>
  <div class="settings-container">
    <div class="settings-content">
      <!-- 左侧导航 -->
      <div class="settings-sidebar">
        <SettingsNav :activeSection="settingsStore.activeSection" @change="handleNavigationChange" />

        <!-- 底部按钮区域 -->
        <div class="settings-sidebar-footer">
          <XButton :loading="isOpeningFolder" variant="primary" size="medium" @click="handleOpenConfigFolder">
            {{ t('settings.general.open_config_folder') }}
          </XButton>
        </div>
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

<style scoped>
  .settings-sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .settings-sidebar-footer {
    display: flex;
    justify-content: center;
    padding: var(--spacing-md);
  }

  /* 响应式设计 */
  @media (max-width: 480px) {
    .settings-sidebar-footer {
      padding: var(--spacing-sm);
    }
  }
</style>
