<script setup lang="ts">
  import { computed, onMounted, ref, watch } from 'vue'
  import { useTheme } from '../../../../composables/useTheme'
  import { XSelect } from '@/ui'
  import type { SelectOption } from '@/ui'

  const theme = useTheme()

  // 组件挂载时初始化主题系统
  onMounted(async () => {
    try {
      await theme.initialize()
      // 确保组件状态与配置同步
      const config = theme.themeConfig.value
      if (config) {
        if (config.lightTheme) {
          selectedLightTheme.value = config.lightTheme
        }
        if (config.darkTheme) {
          selectedDarkTheme.value = config.darkTheme
        }
      }
    } catch (error) {
      console.error('主题系统初始化失败:', error)
    }
  })

  // 本地状态 - 设置默认值
  const selectedLightTheme = ref('light')
  const selectedDarkTheme = ref('dark')

  // 简化模式计算 - 直接使用配置状态，移除复杂的本地状态逻辑
  const currentMode = computed(() => {
    const isFollowing = theme.isFollowingSystem.value
    return isFollowing ? 'system' : 'manual'
  })

  const currentThemeName = computed(() => theme.currentThemeName.value)

  const systemStatus = computed(() => {
    const isDark = theme.isSystemDark.value
    if (isDark === null) return '未知'
    return isDark ? '深色模式' : '浅色模式'
  })

  // 主题选项 - 用于手动模式
  const manualThemeOptions = computed(() => {
    return theme.themeOptions.value.map((option: any) => ({
      value: option.value,
      label: option.label,
      type: option.type,
      icon: getThemeIcon(option.type),
      isCurrent: option.isCurrent,
    }))
  })

  // 系统主题选项
  const lightThemeOptions = computed((): SelectOption[] => {
    return theme.themeOptions.value
      .filter((option: any) => option.type === 'light' || option.type === 'auto')
      .map((option: any) => ({
        label: option.label,
        value: option.value,
      }))
  })

  const darkThemeOptions = computed((): SelectOption[] => {
    return theme.themeOptions.value
      .filter((option: any) => option.type === 'dark' || option.type === 'auto')
      .map((option: any) => ({
        label: option.label,
        value: option.value,
      }))
  })

  // 监听主题配置变化，更新本地选择
  watch(
    () => theme.themeConfig.value,
    config => {
      if (config) {
        // 只有在配置中有值时才更新，否则保持默认值
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

  // 获取主题图标
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

  // 获取图标SVG
  const getIconSvg = (iconName: string) => {
    const icons: Record<string, string> = {
      sun: `<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <circle cx="12" cy="12" r="5"/>
      <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/>
    </svg>`,
      moon: `<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
    </svg>`,
      monitor: `<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <rect x="2" y="3" width="20" height="14" rx="2" ry="2"/>
      <line x1="8" y1="21" x2="16" y2="21"/>
      <line x1="12" y1="17" x2="12" y2="21"/>
    </svg>`,
      palette: `<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <circle cx="13.5" cy="6.5" r=".5"/>
      <circle cx="17.5" cy="10.5" r=".5"/>
      <circle cx="8.5" cy="7.5" r=".5"/>
      <circle cx="6.5" cy="12.5" r=".5"/>
      <path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z"/>
    </svg>`,
    }
    return icons[iconName] || ''
  }

  // 事件处理
  const handleModeChange = async (mode: 'manual' | 'system') => {
    // 防止重复切换
    if (currentMode.value === mode) {
      return
    }

    try {
      if (mode === 'system') {
        // 确保有默认的浅色和深色主题
        const lightTheme = selectedLightTheme.value || 'light'
        const darkTheme = selectedDarkTheme.value || 'dark'

        await theme.enableFollowSystem(lightTheme, darkTheme)
      } else {
        await theme.disableFollowSystem()
      }
    } catch (error) {
      console.error('切换主题模式失败:', error)
    }
  }

  const handleThemeSelect = async (themeName: string) => {
    try {
      await theme.switchToTheme(themeName)
    } catch (error) {
      console.error('切换主题失败:', error)
    }
  }

  const handleSystemThemeChange = async () => {
    if (currentMode.value === 'system') {
      try {
        await theme.setFollowSystem(true, selectedLightTheme.value, selectedDarkTheme.value)
      } catch (error) {
        console.error('设置跟随系统主题失败:', error)
      }
    }
  }
</script>

<template>
  <div class="theme-settings">
    <!-- 模式选择 -->
    <div class="settings-card">
      <h3 class="section-title">主题模式</h3>
      <div class="mode-selector">
        <label class="mode-option">
          <input type="radio" value="manual" :checked="currentMode === 'manual'" @change="handleModeChange('manual')" />
          <div class="mode-content">
            <div class="mode-left">
              <div class="mode-icon" v-html="getIconSvg('palette')"></div>
              <div class="mode-info">
                <div class="mode-label">手动选择</div>
                <div class="mode-description">手动选择一个固定主题</div>
              </div>
            </div>
            <div class="option-radio">
              <div class="radio-button" :class="{ checked: currentMode === 'manual' }">
                <div class="radio-dot"></div>
              </div>
            </div>
          </div>
        </label>

        <label class="mode-option">
          <input type="radio" value="system" :checked="currentMode === 'system'" @change="handleModeChange('system')" />
          <div class="mode-content">
            <div class="mode-left">
              <div class="mode-icon" v-html="getIconSvg('monitor')"></div>
              <div class="mode-info">
                <div class="mode-label">跟随系统</div>
                <div class="mode-description">根据系统设置自动切换主题</div>
              </div>
            </div>
            <div class="option-radio">
              <div class="radio-button" :class="{ checked: currentMode === 'system' }">
                <div class="radio-dot"></div>
              </div>
            </div>
          </div>
        </label>
      </div>
    </div>

    <!-- 手动模式设置 -->
    <div v-if="currentMode === 'manual'" class="settings-card">
      <h3 class="section-title">选择主题</h3>
      <div class="theme-grid">
        <div
          v-for="option in manualThemeOptions"
          :key="option.value"
          class="theme-card"
          :class="{ active: option.isCurrent }"
          @click="handleThemeSelect(option.value)"
        >
          <div class="theme-left">
            <div class="theme-icon" v-html="getIconSvg(option.icon)"></div>
            <div class="theme-info">
              <div class="theme-name">{{ option.label }}</div>
            </div>
          </div>
          <div class="option-radio">
            <div class="radio-button" :class="{ checked: option.isCurrent }">
              <div class="radio-dot"></div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 跟随系统模式设置 -->
    <div v-if="currentMode === 'system'" class="settings-card">
      <h3 class="section-title">选择主题</h3>

      <!-- 系统状态显示 -->
      <div class="system-status">
        <div class="status-content">
          <div class="status-item">
            <div class="status-item-icon" v-html="getIconSvg(theme.isSystemDark.value ? 'moon' : 'sun')"></div>
            <div class="status-item-content">
              <span class="status-label">当前系统主题</span>
              <span class="status-value">{{ systemStatus }}</span>
            </div>
          </div>
          <div class="status-item">
            <div class="status-item-icon" v-html="getIconSvg('palette')"></div>
            <div class="status-item-content">
              <span class="status-label">正在使用</span>
              <span class="status-value">{{ currentThemeName }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- 主题选择器组 -->
      <div class="theme-selectors">
        <!-- 浅色主题选择 -->
        <div class="theme-selector">
          <div class="selector-header">
            <div class="selector-icon" v-html="getIconSvg('sun')"></div>
            <h4 class="selector-title">浅色主题</h4>
          </div>
          <div class="selector-content">
            <XSelect
              v-model="selectedLightTheme"
              :options="lightThemeOptions"
              placeholder="选择浅色主题"
              size="medium"
              @change="handleSystemThemeChange"
              class="theme-select"
            />
          </div>
        </div>

        <!-- 深色主题选择 -->
        <div class="theme-selector">
          <div class="selector-header">
            <div class="selector-icon" v-html="getIconSvg('moon')"></div>
            <h4 class="selector-title">深色主题</h4>
          </div>
          <div class="selector-content">
            <XSelect
              v-model="selectedDarkTheme"
              :options="darkThemeOptions"
              placeholder="选择深色主题"
              size="medium"
              @change="handleSystemThemeChange"
              class="theme-select"
            />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .theme-settings {
    padding: 24px;
    background: var(--bg-600);
  }

  .settings-card {
    margin-bottom: 32px;
  }

  .section-title {
    font-size: 20px;
    color: var(--text-100);
    margin-bottom: 16px;
  }

  .mode-selector {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .mode-option input[type='radio'] {
    display: none;
  }

  .mode-content {
    display: flex;
    align-items: center;
    padding: 12px 16px;
    background: var(--bg-500);
    border-radius: 4px;
    cursor: pointer;
    transition: background 0.15s;
  }

  .mode-content:hover {
    background: var(--bg-400);
  }

  .mode-option input[type='radio']:checked + .mode-content {
    background: var(--color-primary);
  }

  .mode-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .mode-icon {
    color: var(--text-300);
    width: 18px;
    height: 18px;
  }

  .mode-option input[type='radio']:checked + .mode-content .mode-icon {
    color: white;
  }

  .mode-label {
    color: var(--text-200);
    font-size: 14px;
  }

  .mode-description {
    font-size: 12px;
    color: var(--text-400);
    margin-top: 2px;
  }

  .mode-option input[type='radio']:checked + .mode-content .mode-label,
  .mode-option input[type='radio']:checked + .mode-content .mode-description {
    color: white;
  }

  .theme-grid {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 8px;
  }

  .theme-card {
    display: flex;
    align-items: center;
    padding: 12px 16px;
    background: var(--bg-500);
    border-radius: 4px;
    cursor: pointer;
    transition: background 0.15s;
  }

  .theme-card:hover {
    background: var(--bg-400);
  }

  .theme-card.active {
    background: var(--color-primary);
  }

  .theme-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .theme-icon {
    color: var(--text-300);
    width: 16px;
    height: 16px;
  }

  .theme-card.active .theme-icon {
    color: white;
  }

  .theme-name {
    color: var(--text-200);
    font-size: 14px;
  }

  .theme-card.active .theme-name {
    color: white;
  }

  .system-status {
    margin-bottom: 24px;
    padding: 16px;
    background: var(--bg-500);
    border-radius: 4px;
    border-left: 3px solid var(--color-primary);
  }

  .status-content {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .status-item {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .status-item-icon {
    color: var(--text-300);
    width: 16px;
    height: 16px;
  }

  .status-item-content {
    display: flex;
    justify-content: space-between;
    flex: 1;
  }

  .status-label {
    color: var(--text-200);
    font-size: 13px;
  }

  .status-value {
    color: var(--color-primary);
    font-size: 13px;
  }

  .theme-selectors {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 24px;
    margin-top: 16px;
  }

  .selector-header {
    display: flex;
    align-items: center;
    margin-bottom: 8px;
    gap: 8px;
  }

  .selector-icon {
    color: var(--text-300);
    width: 16px;
    height: 16px;
  }

  .selector-title {
    font-size: 14px;
    color: var(--text-200);
  }

  :deep(.x-select) {
    background: var(--bg-500);
    border: 1px solid var(--border-300);
    border-radius: 4px;
    color: var(--text-200);
    font-size: 13px;
  }

  :deep(.x-select:focus) {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px var(--color-primary-alpha);
  }

  :deep(.x-select-dropdown) {
    background: var(--bg-400);
    border: 1px solid var(--border-300);
    border-radius: 4px;
  }

  :deep(.x-select-option) {
    padding: 8px 12px;
    font-size: 13px;
    color: var(--text-200);
  }

  :deep(.x-select-option:hover) {
    background: var(--bg-300);
  }

  :deep(.x-select-option.selected) {
    background: var(--color-primary);
    color: white;
  }
</style>
