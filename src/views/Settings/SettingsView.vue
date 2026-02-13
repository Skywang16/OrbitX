<script setup lang="ts">
  import { useI18n } from 'vue-i18n'
  import AISettings from '@/components/settings/components/AI/AISettings.vue'
  import ThemeSettings from '@/components/settings/components/Theme/ThemeSettings.vue'
  import ShortcutSettings from '@/components/settings/components/Shortcuts/ShortcutSettings.vue'
  import { LanguageSettings } from '@/components/settings/components/Language'
  import { GeneralSettings } from '@/components/settings/components/General'
  import SettingsNav from '@/components/settings/SettingsNav.vue'

  import { configApi } from '@/api/config'
  import { onMounted, ref, nextTick } from 'vue'
  import { debounce } from 'lodash-es'
  import { useEditorStore } from '@/stores/Editor'

  const { t } = useI18n()
  const tabManagerStore = useEditorStore()

  const currentTabId = tabManagerStore.activeTabId
  const savedSection = currentTabId ? tabManagerStore.getSettingsTabSection(currentTabId) : undefined
  const activeSection = ref<string>(savedSection || 'general')

  const aiSettingsRef = ref()
  const themeSettingsRef = ref()
  const shortcutSettingsRef = ref()
  const generalSettingsRef = ref()

  onMounted(async () => {
    await initializeCurrentSection()
  })

  const initializeCurrentSection = async () => {
    const section = activeSection.value

    try {
      await nextTick()

      switch (section) {
        case 'general':
          if (generalSettingsRef.value?.init) {
            await generalSettingsRef.value.init()
          }
          break
        case 'ai':
          if (aiSettingsRef.value?.init) {
            await aiSettingsRef.value.init()
          }
          break
        case 'theme':
          if (themeSettingsRef.value?.init) {
            await themeSettingsRef.value.init()
          }
          break
        case 'shortcuts':
          if (shortcutSettingsRef.value?.init) {
            await shortcutSettingsRef.value.init()
          }
          break
        case 'language':
          break
        default:
          break
      }
    } catch (error) {
      console.error(`Failed to initialize ${section} settings:`, error)
    }
  }

  const handleNavigationChange = async (section: string) => {
    activeSection.value = section

    if (currentTabId !== null) {
      tabManagerStore.updateSettingsTabSection(currentTabId, section)
    }

    await initializeCurrentSection()
  }

  const openConfigFolder = async () => {
    await configApi.openConfigFolder()
  }

  const handleOpenConfigFolder = debounce(openConfigFolder, 500)
</script>

<template>
  <div class="settings-container">
    <div class="settings-content">
      <div class="settings-sidebar">
        <SettingsNav :activeSection="activeSection" @change="handleNavigationChange" />

        <div class="settings-sidebar-footer">
          <x-button variant="primary" size="medium" @click="handleOpenConfigFolder">
            {{ t('settings.general.config_open_folder') }}
          </x-button>
        </div>
      </div>

      <div class="settings-main">
        <div class="settings-panel">
          <GeneralSettings v-if="activeSection === 'general'" ref="generalSettingsRef" />
          <AISettings v-else-if="activeSection === 'ai'" ref="aiSettingsRef" />
          <ThemeSettings v-else-if="activeSection === 'theme'" ref="themeSettingsRef" />
          <ShortcutSettings v-else-if="activeSection === 'shortcuts'" ref="shortcutSettingsRef" />
          <LanguageSettings v-else-if="activeSection === 'language'" />
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
    background: var(--bg-200);
    border-top: 1px solid var(--border-200);
  }
</style>
