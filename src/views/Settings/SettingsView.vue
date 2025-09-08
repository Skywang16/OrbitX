<script setup lang="ts">
  import { useI18n } from 'vue-i18n'
  import AISettings from '@/components/settings/components/AI/AISettings.vue'
  import ThemeSettings from '@/components/settings/components/Theme/ThemeSettings.vue'
  import ShortcutSettings from '@/components/settings/components/Shortcuts/ShortcutSettings.vue'
  import { LanguageSettings } from '@/components/settings/components/Language'
  import { GeneralSettings } from '@/components/settings/components/General'
  import { VectorIndexSettings } from '@/components/settings/components/VectorIndex'
  import SettingsNav from '@/components/settings/SettingsNav.vue'
  import { useSettingsStore } from '@/components/settings/store'
  import { createMessage, XButton } from '@/ui'
  import { handleErrorWithMessage } from '@/utils/errorHandler'
  import { configApi } from '@/api/config'
  import { onMounted } from 'vue'
  import { debounce } from 'lodash-es'

  const { t } = useI18n()
  const settingsStore = useSettingsStore()

  onMounted(async () => {
    settingsStore.setActiveSection('general')
    await settingsStore.initializeSettings()
  })

  const handleNavigationChange = (section: string) => {
    settingsStore.setActiveSection(section)
  }

  const openConfigFolder = async () => {
    try {
      await configApi.openConfigFolder()
      createMessage.success(t('settings.general.config_folder_opened'))
    } catch (error) {
      console.error('Failed to open config folder:', error)
      handleErrorWithMessage(error, t('settings.general.config_folder_error'))
    }
  }

  // 创建防抖版本的函数，防止用户快速点击导致重复调用
  const handleOpenConfigFolder = debounce(openConfigFolder, 500)
</script>

<template>
  <div class="settings-container">
    <div class="settings-content">
      <!-- 左侧导航 -->
      <div class="settings-sidebar">
        <SettingsNav :activeSection="settingsStore.activeSection" @change="handleNavigationChange" />

        <!-- 底部按钮区域 -->
        <div class="settings-sidebar-footer">
          <XButton variant="primary" size="medium" @click="handleOpenConfigFolder">
            {{ t('settings.general.open_config_folder') }}
          </XButton>
        </div>
      </div>

      <!-- 右侧内容区域 -->
      <div class="settings-main">
        <div class="settings-panel">
          <GeneralSettings v-if="settingsStore.activeSection === 'general'" />
          <AISettings v-else-if="settingsStore.activeSection === 'ai'" />
          <VectorIndexSettings v-else-if="settingsStore.activeSection === 'vectorIndex'" />
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
