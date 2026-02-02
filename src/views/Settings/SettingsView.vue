<script setup lang="ts">
  import { useI18n } from 'vue-i18n'
  import AISettings from '@/components/settings/components/AI/AISettings.vue'
  import ThemeSettings from '@/components/settings/components/Theme/ThemeSettings.vue'
  import ShortcutSettings from '@/components/settings/components/Shortcuts/ShortcutSettings.vue'
  import { LanguageSettings } from '@/components/settings/components/Language'
  import { GeneralSettings } from '@/components/settings/components/General'
  import SettingsNav from '@/components/settings/SettingsNav.vue'

  import { configApi } from '@/api/config'
  import { onMounted, ref, nextTick, computed } from 'vue'
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

  // Get section info for header
  const sectionInfo = computed(() => {
    const sections: Record<string, { title: string; description: string }> = {
      general: {
        title: t('settings.general.title'),
        description: t('settings.general.description'),
      },
      ai: {
        title: t('settings.ai.title'),
        description: t('settings.ai.description'),
      },
      theme: {
        title: t('settings.theme.title'),
        description: t('settings.theme.description'),
      },
      shortcuts: {
        title: t('settings.shortcuts.title'),
        description: t('settings.shortcuts.description'),
      },
      language: {
        title: t('settings.language.title'),
        description: t('settings.language.description'),
      },
    }
    return sections[activeSection.value] || sections.general
  })

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
          <x-button variant="ghost" size="small" @click="handleOpenConfigFolder" class="config-folder-btn">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="btn-icon">
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
            </svg>
            {{ t('settings.general.config_open_folder') }}
          </x-button>
        </div>
      </div>

      <div class="settings-main">
        <div class="settings-panel">
          <!-- Page Header -->
          <div class="settings-page-header">
            <h1 class="settings-page-title">{{ sectionInfo.title }}</h1>
            <p class="settings-page-description">{{ sectionInfo.description }}</p>
          </div>

          <!-- Content -->
          <Transition name="settings-fade" mode="out-in">
            <GeneralSettings v-if="activeSection === 'general'" ref="generalSettingsRef" :key="'general'" />
            <AISettings v-else-if="activeSection === 'ai'" ref="aiSettingsRef" :key="'ai'" />
            <ThemeSettings v-else-if="activeSection === 'theme'" ref="themeSettingsRef" :key="'theme'" />
            <ShortcutSettings v-else-if="activeSection === 'shortcuts'" ref="shortcutSettingsRef" :key="'shortcuts'" />
            <LanguageSettings v-else-if="activeSection === 'language'" :key="'language'" />
          </Transition>
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
    padding: 12px 12px 16px;
    border-top: 1px solid var(--border-100);
    background: var(--bg-200);
  }

  .config-folder-btn {
    width: 100%;
    justify-content: flex-start;
    gap: 8px;
    color: var(--text-300);
    font-size: 12px;
  }

  .config-folder-btn:hover {
    color: var(--text-100);
  }

  .config-folder-btn .btn-icon {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
  }

  /* Page transitions */
  .settings-fade-enter-active,
  .settings-fade-leave-active {
    transition: opacity 0.15s ease;
  }

  .settings-fade-enter-from,
  .settings-fade-leave-to {
    opacity: 0;
  }
</style>
