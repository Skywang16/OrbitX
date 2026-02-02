<script setup lang="ts">
  import { computed, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useThemeStore } from '@/stores/theme'
  import { getWindowOpacity, setWindowOpacity } from '@/api/window/opacity'
  import { XSelect } from '@/ui'
  import type { SelectOption } from '@/ui'
  import type { ThemeOption } from '@/types/domain/theme'
  import SettingsCard from '../../SettingsCard.vue'

  const themeStore = useThemeStore()
  const { t } = useI18n()

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
    return themeStore.themeOptions.map((option: ThemeOption) => ({
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
      await setWindowOpacity(opacity.value)
    }, 100)
  }

  const syncOpacityFromConfig = async () => {
    try {
      opacity.value = await getWindowOpacity()
    } catch (error) {
      console.warn('Failed to sync opacity config:', error)
    }
  }
</script>

<template>
  <div class="theme-settings">
    <!-- Theme Mode Section -->
    <div class="settings-section">
      <h3 class="settings-section-title">{{ t('theme_settings.theme_mode') }}</h3>

      <SettingsCard>
        <div
          class="settings-item clickable"
          :class="{ active: currentMode === 'manual' }"
          @click="handleModeChange('manual')"
        >
          <div class="settings-item-header">
            <div class="settings-label">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="item-icon">
                <circle cx="12" cy="12" r="5" />
                <line x1="12" y1="1" x2="12" y2="3" />
                <line x1="12" y1="21" x2="12" y2="23" />
                <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
                <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
                <line x1="1" y1="12" x2="3" y2="12" />
                <line x1="21" y1="12" x2="23" y2="12" />
                <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
                <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
              </svg>
              {{ t('theme_settings.manual_selection') }}
            </div>
            <div class="settings-description">{{ t('theme_settings.manual_description') }}</div>
          </div>
          <div class="settings-item-control">
            <div class="radio-indicator" :class="{ checked: currentMode === 'manual' }"></div>
          </div>
        </div>

        <div
          class="settings-item clickable"
          :class="{ active: currentMode === 'system' }"
          @click="handleModeChange('system')"
        >
          <div class="settings-item-header">
            <div class="settings-label">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="item-icon">
                <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
                <line x1="8" y1="21" x2="16" y2="21" />
                <line x1="12" y1="17" x2="12" y2="21" />
              </svg>
              {{ t('theme_settings.follow_system') }}
            </div>
            <div class="settings-description">{{ t('theme_settings.follow_system_description') }}</div>
          </div>
          <div class="settings-item-control">
            <div class="radio-indicator" :class="{ checked: currentMode === 'system' }"></div>
          </div>
        </div>
      </SettingsCard>
    </div>

    <!-- Manual Theme Selection -->
    <div v-if="currentMode === 'manual'" class="settings-section">
      <h3 class="settings-section-title">{{ t('theme_settings.select_theme') }}</h3>

      <div class="theme-grid">
        <div
          v-for="option in manualThemeOptions"
          :key="option.value"
          class="theme-card"
          :class="{ selected: option.isCurrent, light: option.type === 'light', dark: option.type === 'dark' }"
          @click="handleThemeSelect(option.value)"
        >
          <div class="theme-preview">
            <div class="preview-header">
              <div class="preview-dots">
                <span></span>
                <span></span>
                <span></span>
              </div>
            </div>
            <div class="preview-content">
              <div class="preview-sidebar"></div>
              <div class="preview-main">
                <div class="preview-line short"></div>
                <div class="preview-line"></div>
                <div class="preview-line medium"></div>
              </div>
            </div>
          </div>
          <div class="theme-info">
            <span class="theme-name">{{ option.label }}</span>
            <svg
              v-if="option.isCurrent"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="3"
              class="check-icon"
            >
              <polyline points="20 6 9 17 4 12" />
            </svg>
          </div>
        </div>
      </div>
    </div>

    <!-- System Theme Selection -->
    <div v-if="currentMode === 'system'" class="settings-section">
      <h3 class="settings-section-title">{{ t('theme_settings.system_themes') || 'System Theme Mapping' }}</h3>

      <SettingsCard>
        <div class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="item-icon">
                <circle cx="12" cy="12" r="5" />
                <line x1="12" y1="1" x2="12" y2="3" />
                <line x1="12" y1="21" x2="12" y2="23" />
                <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
                <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
                <line x1="1" y1="12" x2="3" y2="12" />
                <line x1="21" y1="12" x2="23" y2="12" />
                <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
                <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
              </svg>
              {{ t('theme_settings.light_theme') }}
            </div>
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
            <div class="settings-label">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="item-icon">
                <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
              </svg>
              {{ t('theme_settings.dark_theme') }}
            </div>
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

    <!-- Window Opacity Section -->
    <div class="settings-section">
      <h3 class="settings-section-title">{{ t('theme_settings.window_opacity') }}</h3>

      <SettingsCard>
        <div class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">{{ t('theme_settings.opacity') }}</div>
            <div class="settings-description">
              {{ t('theme_settings.opacity_description') }}
            </div>
          </div>
          <div class="settings-item-control slider-control">
            <input
              v-model.number="opacity"
              type="range"
              min="0.05"
              max="1"
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
  .theme-settings {
    display: flex;
    flex-direction: column;
    gap: 32px;
  }

  .settings-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  /* Item Icons */
  .item-icon {
    width: 16px;
    height: 16px;
    margin-right: 8px;
    vertical-align: middle;
    color: var(--text-400);
  }

  .settings-item.active .item-icon {
    color: var(--color-primary);
  }

  /* Radio Indicator */
  .radio-indicator {
    width: 18px;
    height: 18px;
    border: 2px solid var(--border-300);
    border-radius: 50%;
    position: relative;
    transition: all 0.15s ease;
  }

  .radio-indicator.checked {
    border-color: var(--color-primary);
    background: var(--color-primary);
  }

  .radio-indicator.checked::after {
    content: '';
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: white;
  }

  /* Theme Grid */
  .theme-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 12px;
  }

  .theme-card {
    background: var(--bg-200);
    border: 2px solid var(--border-100);
    border-radius: 12px;
    overflow: hidden;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .theme-card:hover {
    border-color: var(--border-300);
    transform: translateY(-2px);
  }

  .theme-card.selected {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 20%, transparent);
  }

  /* Theme Preview */
  .theme-preview {
    aspect-ratio: 16 / 10;
    background: var(--bg-300);
    padding: 6px;
    display: flex;
    flex-direction: column;
  }

  .theme-card.light .theme-preview {
    background: #f5f5f5;
  }

  .theme-card.dark .theme-preview {
    background: #1a1a1a;
  }

  .preview-header {
    height: 10px;
    display: flex;
    align-items: center;
    padding: 0 4px;
    margin-bottom: 4px;
  }

  .preview-dots {
    display: flex;
    gap: 3px;
  }

  .preview-dots span {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--text-500);
    opacity: 0.5;
  }

  .theme-card.light .preview-dots span {
    background: #999;
  }

  .theme-card.dark .preview-dots span {
    background: #666;
  }

  .preview-content {
    flex: 1;
    display: flex;
    gap: 4px;
    border-radius: 4px;
    overflow: hidden;
  }

  .preview-sidebar {
    width: 25%;
    background: var(--bg-400);
    border-radius: 3px;
  }

  .theme-card.light .preview-sidebar {
    background: #e0e0e0;
  }

  .theme-card.dark .preview-sidebar {
    background: #2a2a2a;
  }

  .preview-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 4px;
    background: var(--bg-200);
    border-radius: 3px;
  }

  .theme-card.light .preview-main {
    background: #fff;
  }

  .theme-card.dark .preview-main {
    background: #222;
  }

  .preview-line {
    height: 4px;
    background: var(--bg-400);
    border-radius: 2px;
  }

  .preview-line.short {
    width: 40%;
  }

  .preview-line.medium {
    width: 70%;
  }

  .theme-card.light .preview-line {
    background: #ddd;
  }

  .theme-card.dark .preview-line {
    background: #3a3a3a;
  }

  /* Theme Info */
  .theme-info {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    border-top: 1px solid var(--border-100);
  }

  .theme-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-200);
  }

  .theme-card.selected .theme-name {
    color: var(--color-primary);
  }

  .check-icon {
    width: 16px;
    height: 16px;
    color: var(--color-primary);
  }

  /* Slider Control */
  .slider-control {
    gap: 12px;
  }
</style>
