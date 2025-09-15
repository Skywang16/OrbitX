<script setup lang="ts">
  import { computed, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useThemeStore } from '@/stores/theme'
  import { windowApi } from '@/api/window'
  import { configApi } from '@/api/config'
  import { XSelect } from '@/ui'
  import type { SelectOption } from '@/ui'
  import { useSessionStore } from '@/stores/session'
  import SettingsCard from '../../SettingsCard.vue'

  const themeStore = useThemeStore()
  const { t } = useI18n()
  const sessionStore = useSessionStore()

  // 初始化方法，供外部调用
  const init = async () => {
    await themeStore.initialize()
    const config = themeStore.themeConfig
    if (config) {
      if (config.lightTheme) {
        selectedLightTheme.value = config.lightTheme
      }
      if (config.darkTheme) {
        selectedDarkTheme.value = config.darkTheme
      }
    }
    await syncOpacityFromConfig()
  }

  // 暴露初始化方法给父组件
  defineExpose({
    init,
  })

  const selectedLightTheme = ref('light')
  const selectedDarkTheme = ref('dark')

  const opacity = ref(1.0)

  const currentMode = computed(() => {
    const isFollowing = themeStore.isFollowingSystem
    return isFollowing ? 'system' : 'manual'
  })

  // 缓存主题选项以避免重复计算
  const themeOptionsCache = computed(() => {
    return themeStore.themeOptions.map((option: any) => ({
      value: option.value,
      label: option.label,
      type: option.type,
      icon: getThemeIcon(option.type),
      isCurrent: option.isCurrent,
    }))
  })

  const manualThemeOptions = computed(() => themeOptionsCache.value)

  const lightThemeOptions = computed((): SelectOption[] => {
    return themeOptionsCache.value
      .filter(option => option.type === 'light' || option.type === 'auto')
      .map(option => ({
        label: option.label,
        value: option.value,
      }))
  })

  const darkThemeOptions = computed((): SelectOption[] => {
    return themeOptionsCache.value
      .filter(option => option.type === 'dark' || option.type === 'auto')
      .map(option => ({
        label: option.label,
        value: option.value,
      }))
  })

  watch(
    () => themeStore.themeConfig,
    config => {
      if (config) {
        if (config.lightTheme) {
          selectedLightTheme.value = config.lightTheme
        }
        if (config.darkTheme) {
          selectedDarkTheme.value = config.darkTheme
        }
      }
    },
    { immediate: true }
  )

  const getThemeIcon = (themeType: string) => {
    switch (themeType) {
      case 'light':
        return 'sun'
      case 'dark':
        return 'moon'
      case 'auto':
        return 'monitor'
      default:
        return 'palette'
    }
  }

  const handleModeChange = async (mode: 'manual' | 'system') => {
    if (currentMode.value === mode) {
      return
    }

    if (mode === 'system') {
      const lightTheme = selectedLightTheme.value || 'light'
      const darkTheme = selectedDarkTheme.value || 'dark'
      await themeStore.enableFollowSystem(lightTheme, darkTheme)
    } else {
      await themeStore.disableFollowSystem()
    }
  }

  const handleThemeSelect = async (themeName: string) => {
    await themeStore.switchToTheme(themeName)
  }

  const handleSystemThemeChange = async () => {
    if (currentMode.value === 'system') {
      try {
        await themeStore.setFollowSystem(true, selectedLightTheme.value, selectedDarkTheme.value)
      } catch (error) {
        console.error('Failed to set follow system theme:', error)
      }
    }
  }

  let opacityTimeout: NodeJS.Timeout | null = null

  const handleOpacityChange = async () => {
    if (opacityTimeout) {
      clearTimeout(opacityTimeout)
    }

    opacityTimeout = setTimeout(async () => {
      await windowApi.setWindowOpacity(opacity.value)
      await saveOpacityToConfig()
    }, 100)
  }

  const saveOpacityToConfig = async () => {
    try {
      const config = await configApi.getConfig()
      config.appearance.opacity = opacity.value
      await configApi.updateConfig(config)

      sessionStore.updateUiState({ opacity: opacity.value })
    } catch (error) {
      console.warn('Failed to save opacity config:', error)
    }
  }

  const syncOpacityFromConfig = async () => {
    try {
      const config = await configApi.getConfig()
      if (config.appearance.opacity !== undefined) {
        opacity.value = config.appearance.opacity
      } else {
        const currentOpacity = await windowApi.getWindowOpacity()
        opacity.value = currentOpacity
      }
    } catch (error) {
      console.warn('Failed to sync opacity config:', error)
    }
  }
</script>

<template>
  <div class="settings-group">
    <h2 class="settings-section-title">{{ t('settings.theme.title') }}</h2>

    <!-- 主题模式选择 -->
    <div class="settings-group">
      <h3 class="settings-group-title">{{ t('theme_settings.theme_mode') }}</h3>

      <SettingsCard>
        <div
          class="settings-item clickable"
          :class="{ active: currentMode === 'manual' }"
          @click="handleModeChange('manual')"
        >
          <div class="settings-item-header">
            <div class="settings-label">{{ t('theme_settings.manual_selection') }}</div>
            <div class="settings-description">{{ t('theme_settings.manual_description') }}</div>
          </div>
          <div class="settings-item-control">
            <input
              type="radio"
              value="manual"
              :checked="currentMode === 'manual'"
              class="settings-radio"
              @change="handleModeChange('manual')"
            />
          </div>
        </div>

        <div
          class="settings-item clickable"
          :class="{ active: currentMode === 'system' }"
          @click="handleModeChange('system')"
        >
          <div class="settings-item-header">
            <div class="settings-label">{{ t('theme_settings.follow_system') }}</div>
            <div class="settings-description">{{ t('theme_settings.follow_system_description') }}</div>
          </div>
          <div class="settings-item-control">
            <input
              type="radio"
              value="system"
              :checked="currentMode === 'system'"
              class="settings-radio"
              @change="handleModeChange('system')"
            />
          </div>
        </div>
      </SettingsCard>
    </div>

    <!-- 手动选择主题 -->
    <div v-if="currentMode === 'manual'" class="settings-group">
      <h3 class="settings-group-title">{{ t('theme_settings.select_theme') }}</h3>

      <SettingsCard>
        <div
          v-for="option in manualThemeOptions"
          :key="option.value"
          class="settings-item clickable"
          :class="{ active: option.isCurrent }"
          @click="handleThemeSelect(option.value)"
        >
          <div class="settings-item-header">
            <div class="settings-label">{{ option.label }}</div>
            <div class="settings-description">
              {{ option.type === 'light' ? t('theme_settings.light_theme') : t('theme_settings.dark_theme') }}
            </div>
          </div>
          <div class="settings-item-control">
            <input
              type="radio"
              :value="option.value"
              :checked="option.isCurrent"
              class="settings-radio"
              @change="handleThemeSelect(option.value)"
            />
          </div>
        </div>
      </SettingsCard>
    </div>

    <!-- 系统跟随主题选择 -->
    <div v-if="currentMode === 'system'" class="settings-group">
      <h3 class="settings-group-title">{{ t('theme_settings.select_theme') }}</h3>

      <SettingsCard>
        <div class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">{{ t('theme_settings.light_theme') }}</div>
            <div class="settings-description">{{ t('theme_settings.light_theme_description') }}</div>
          </div>
          <div class="settings-item-control">
            <XSelect
              v-model="selectedLightTheme"
              :options="lightThemeOptions"
              :placeholder="t('theme.select_light')"
              size="medium"
              @change="handleSystemThemeChange"
            />
          </div>
        </div>

        <div class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">{{ t('theme_settings.dark_theme') }}</div>
            <div class="settings-description">{{ t('theme_settings.dark_theme_description') }}</div>
          </div>
          <div class="settings-item-control">
            <XSelect
              v-model="selectedDarkTheme"
              :options="darkThemeOptions"
              :placeholder="t('theme.select_dark')"
              size="medium"
              @change="handleSystemThemeChange"
            />
          </div>
        </div>
      </SettingsCard>
    </div>

    <!-- 窗口透明度 -->
    <div class="settings-group">
      <h3 class="settings-group-title">{{ t('theme_settings.window_opacity') }}</h3>

      <SettingsCard>
        <div class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">{{ t('theme_settings.opacity') }}</div>
            <div class="settings-description">
              {{ t('theme_settings.opacity_description') }}
            </div>
          </div>
          <div class="settings-item-control">
            <input
              v-model.number="opacity"
              type="range"
              min="0.1"
              max="1.0"
              step="0.01"
              class="settings-slider"
              @input="handleOpacityChange"
            />
            <span class="settings-value">{{ Math.round(opacity * 100) }}%</span>
          </div>
        </div>
      </SettingsCard>
    </div>
  </div>
</template>

<style scoped>
  .theme-selection-list {
    display: flex;
    flex-direction: column;
    gap: var(--settings-spacing-compact);
  }

  @media (max-width: 480px) {
    .settings-item input[type='range'] {
      width: 100%;
      margin-bottom: 8px;
    }

    .settings-item-control {
      flex-direction: column;
      align-items: stretch;
      gap: 8px;
    }
  }

  @media (max-width: 480px) {
    .settings-item input[type='range'] {
      width: 100%;
    }
  }
</style>
