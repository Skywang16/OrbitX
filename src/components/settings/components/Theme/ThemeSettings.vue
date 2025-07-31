<script setup lang="ts">
  import { computed, onMounted, ref, watch } from 'vue'
  import { useTheme } from '../../../../composables/useTheme'

  const theme = useTheme()

  // 组件挂载时初始化主题系统
  onMounted(async () => {
    try {
      await theme.initialize()
    } catch (error) {
      console.error('主题系统初始化失败:', error)
    }
  })

  // 本地状态 - 设置默认值
  const selectedLightTheme = ref('light')
  const selectedDarkTheme = ref('dark')

  // 计算属性
  const currentMode = computed(() => {
    const isFollowing = theme.isFollowingSystem.value
    console.log('currentMode 计算:', {
      isFollowing,
      themeConfig: theme.themeConfig.value,
      configStatus: theme.configStatus.value,
    })
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
  const lightThemeOptions = computed(() => {
    return theme.themeOptions.value.filter((option: any) => option.type === 'light' || option.type === 'auto')
  })

  const darkThemeOptions = computed(() => {
    return theme.themeOptions.value.filter((option: any) => option.type === 'dark' || option.type === 'auto')
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
    console.log('handleModeChange 调用:', { mode, currentMode: currentMode.value })
    try {
      if (mode === 'system') {
        // 确保有默认的浅色和深色主题
        const lightTheme = selectedLightTheme.value || 'light'
        const darkTheme = selectedDarkTheme.value || 'dark'

        console.log('启用跟随系统模式:', { lightTheme, darkTheme })
        await theme.enableFollowSystem(lightTheme, darkTheme)
        console.log('跟随系统模式启用完成，新状态:', {
          isFollowing: theme.isFollowingSystem.value,
          currentMode: currentMode.value,
        })
      } else {
        console.log('禁用跟随系统模式')
        await theme.disableFollowSystem()
        console.log('跟随系统模式禁用完成，新状态:', {
          isFollowing: theme.isFollowingSystem.value,
          currentMode: currentMode.value,
        })
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
        console.error('更新系统主题设置失败:', error)
      }
    }
  }
</script>

<template>
  <div class="theme-settings">
    <!-- 主题设置 -->
    <div class="settings-content">
      <!-- 模式选择 -->
      <div class="settings-section">
        <h3 class="section-title">主题模式</h3>
        <div class="mode-selector">
          <label class="mode-option">
            <input
              type="radio"
              value="manual"
              :checked="currentMode === 'manual'"
              @change="handleModeChange('manual')"
            />
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
            <input
              type="radio"
              value="system"
              :checked="currentMode === 'system'"
              @change="handleModeChange('system')"
            />
            <div class="mode-content">
              <div class="mode-left">
                <div class="mode-icon" v-html="getIconSvg('monitor')"></div>
                <div class="mode-info">
                  <div class="mode-label">跟随系统</div>
                  <div class="mode-description">根据系统设置自动切换浅色/深色主题</div>
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
      <div v-if="currentMode === 'manual'" class="settings-section">
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
      <div v-if="currentMode === 'system'" class="settings-section">
        <h3 class="section-title">系统主题设置</h3>

        <!-- 系统状态显示 -->
        <div class="system-status">
          <div class="status-item">
            <span class="status-label">当前系统主题:</span>
            <span class="status-value">{{ systemStatus }}</span>
          </div>
          <div class="status-item">
            <span class="status-label">正在使用:</span>
            <span class="status-value">{{ currentThemeName }}</span>
          </div>
        </div>

        <!-- 浅色主题选择 -->
        <div class="theme-selector">
          <h4 class="selector-title">
            <div class="title-icon" v-html="getIconSvg('sun')"></div>
            浅色主题
          </h4>
          <select v-model="selectedLightTheme" @change="handleSystemThemeChange" class="theme-select">
            <option v-for="option in lightThemeOptions" :key="option.value" :value="option.value">
              {{ option.label }}
            </option>
          </select>
        </div>

        <!-- 深色主题选择 -->
        <div class="theme-selector">
          <h4 class="selector-title">
            <div class="title-icon" v-html="getIconSvg('moon')"></div>
            深色主题
          </h4>
          <select v-model="selectedDarkTheme" @change="handleSystemThemeChange" class="theme-select">
            <option v-for="option in darkThemeOptions" :key="option.value" :value="option.value">
              {{ option.label }}
            </option>
          </select>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .theme-settings {
    padding: 20px;
    max-width: 800px;
  }

  .action-button {
    padding: 8px 16px;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
  }

  .action-button:hover {
    background: #2563eb;
  }

  .settings-section {
    margin-bottom: 32px;
  }

  .section-title {
    font-size: 18px;
    font-weight: 600;
    margin-bottom: 16px;
    color: var(--text-primary);
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
  }

  .mode-selector {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .mode-option {
    display: block;
    cursor: pointer;
  }

  .mode-option input[type='radio'] {
    display: none;
  }

  .mode-content {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--spacing-sm) var(--spacing-md);
    border: 2px solid var(--border-color);
    border-radius: var(--border-radius-lg);
    transition: all 0.2s ease;
  }

  .mode-left {
    display: flex;
    align-items: center;
    flex: 1;
  }

  .mode-content:hover {
    border-color: var(--border-color-hover);
    background-color: var(--color-background-hover);
  }

  .mode-option input[type='radio']:checked + .mode-content {
    border-color: var(--color-primary);
    background-color: var(--color-primary-alpha);
  }

  .mode-icon {
    margin-right: 12px;
    color: var(--text-secondary);
  }

  .mode-option input[type='radio']:checked + .mode-content .mode-icon {
    color: var(--color-primary);
  }

  .mode-label {
    font-weight: 500;
    color: var(--text-primary);
  }

  .mode-description {
    font-size: 14px;
    color: var(--text-secondary);
    margin-top: 4px;
  }

  .option-radio {
    flex-shrink: 0;
  }

  .radio-button {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border-color);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s ease;
  }

  .radio-button.checked {
    border-color: var(--color-primary);
  }

  .radio-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: var(--color-primary);
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .radio-button.checked .radio-dot {
    opacity: 1;
  }

  .theme-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 16px;
  }

  .theme-card {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--spacing-sm) var(--spacing-md);
    border: 2px solid var(--border-color);
    border-radius: var(--border-radius-lg);
    cursor: pointer;
    transition: all 0.2s ease;
    background-color: var(--color-background);
    min-height: 60px;
  }

  .theme-left {
    display: flex;
    align-items: center;
    flex: 1;
  }

  .theme-card:hover {
    border-color: var(--border-color-hover);
    background-color: var(--color-background-hover);
  }

  .theme-card.active {
    border-color: var(--color-primary);
    background-color: var(--color-primary-alpha);
  }

  .theme-icon {
    margin-right: 12px;
    color: var(--text-secondary);
  }

  .theme-card.active .theme-icon {
    color: var(--color-primary);
  }

  .theme-info {
    flex: 1;
  }

  .theme-name {
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 4px;
  }

  .system-status {
    background: #f9fafb;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    padding: 16px;
    margin-bottom: 24px;
  }

  .status-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }

  .status-item:last-child {
    margin-bottom: 0;
  }

  .status-label {
    font-weight: 500;
    color: var(--text-primary);
  }

  .status-value {
    color: var(--text-secondary);
  }

  .theme-selector {
    margin-bottom: 20px;
  }

  .selector-title {
    display: flex;
    align-items: center;
    font-size: 16px;
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 8px;
  }

  .title-icon {
    margin-right: 8px;
    color: var(--text-secondary);
  }

  .theme-select {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    background: white;
    font-size: 14px;
  }

  .theme-select:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
  }
</style>
