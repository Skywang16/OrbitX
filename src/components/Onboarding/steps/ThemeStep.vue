<template>
  <div class="theme-step">
    <div class="step-header">
      <h2 class="step-title">{{ t('onboarding.theme.title') }}</h2>
      <p class="step-description">{{ t('onboarding.theme.description') }}</p>
    </div>

    <div class="theme-options">
      <div
        v-for="theme in themes"
        :key="theme.id"
        class="theme-option"
        :class="{ selected: selectedTheme === theme.id }"
        @click="selectTheme(theme.id)"
      >
        <div class="theme-preview" :class="`theme-${theme.id}`">
          <div class="preview-header">
            <div class="preview-controls">
              <div class="control-dot red"></div>
              <div class="control-dot yellow"></div>
              <div class="control-dot green"></div>
            </div>
          </div>
          <div class="preview-content">
            <div class="preview-line long"></div>
            <div class="preview-line medium"></div>
            <div class="preview-line short"></div>
            <div class="preview-cursor"></div>
          </div>
        </div>
        <div class="theme-info">
          <h3 class="theme-name">{{ t(theme.name) }}</h3>
          <p class="theme-desc">{{ t(theme.description) }}</p>
        </div>
        <div class="theme-check" v-if="selectedTheme === theme.id">
          <CheckIcon />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useThemeStore } from '@/stores/theme'
  import CheckIcon from './icons/CheckIcon.vue'

  const { t } = useI18n()
  const theme = useThemeStore()

  const selectedTheme = ref('dark')

  const themes = [
    {
      id: 'dark',
      name: 'onboarding.theme.options.dark.name',
      description: 'onboarding.theme.options.dark.description',
    },
    {
      id: 'light',
      name: 'onboarding.theme.options.light.name',
      description: 'onboarding.theme.options.light.description',
    },
    {
      id: 'system',
      name: 'onboarding.theme.options.auto.name',
      description: 'onboarding.theme.options.auto.description',
    },
  ]

  const selectTheme = async (themeId: string) => {
    selectedTheme.value = themeId

    try {
      if (themeId === 'system') {
        await theme.enableFollowSystem('light', 'dark')
      } else {
        await theme.disableFollowSystem()
        await theme.switchToTheme(themeId)
      }
    } catch (error) {
      console.error('Failed to switch theme:', error)
    }
  }

  onMounted(async () => {
    try {
      await theme.initialize()

      // 获取当前主题状态
      if (theme.isFollowingSystem) {
        selectedTheme.value = 'system'
      } else {
        // 获取当前主题
        const currentTheme = theme.currentThemeName
        selectedTheme.value = currentTheme || 'dark'
      }
    } catch (error) {
      console.error('Failed to initialize theme:', error)
      // 回退到系统偏好
      if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
        selectedTheme.value = 'dark'
      } else {
        selectedTheme.value = 'light'
      }
    }
  })
</script>

<style scoped>
  .theme-step {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 100%;
    max-width: 500px;
  }

  .step-header {
    text-align: center;
    margin-bottom: 40px;
  }

  .step-title {
    font-size: 32px;
    font-weight: 600;
    color: var(--text-100);
    margin: 0 0 12px 0;
  }

  .step-description {
    font-size: 16px;
    color: var(--text-400);
    margin: 0;
    line-height: 1.5;
  }

  .theme-options {
    display: flex;
    flex-direction: column;
    gap: 16px;
    width: 100%;
    margin-bottom: 40px;
  }

  .theme-option {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 20px;
    background: var(--bg-200);
    border: 2px solid var(--border-100);
    border-radius: var(--border-radius-xl);
    cursor: pointer;
    transition: all 0.2s ease;
    position: relative;
  }

  .theme-option:not(.selected):hover {
    border-color: var(--color-primary);
    background: var(--bg-300);
  }

  .theme-option.selected {
    border-color: var(--color-primary);
    background: var(--bg-300);
  }

  .theme-preview {
    flex-shrink: 0;
    width: 80px;
    height: 60px;
    border-radius: var(--border-radius-lg);
    overflow: hidden;

    position: relative;
  }

  .theme-dark {
    background: #1a1a1a;
    color: #e5e5e5;
  }

  .theme-dark .preview-header {
    background: #2a2a2a;
    border-bottom: 1px solid #333;
  }

  .theme-dark .preview-line {
    background: #e5e5e5;
  }

  .theme-dark .preview-cursor {
    background: #00ff88;
  }

  .theme-light {
    background: #ffffff;
    color: #1a1a1a;
  }

  .theme-light .preview-header {
    background: #f5f5f5;
    border-bottom: 1px solid #e0e0e0;
  }

  .theme-light .preview-line {
    background: #1a1a1a;
  }

  .theme-light .preview-cursor {
    background: #007acc;
  }

  .theme-system {
    background: linear-gradient(90deg, #1a1a1a 50%, #ffffff 50%);
    position: relative;
  }

  .theme-system .preview-header {
    background: linear-gradient(90deg, #2a2a2a 50%, #f5f5f5 50%);
    border-bottom: 1px solid #666;
  }

  .theme-system .preview-line {
    background: linear-gradient(90deg, #e5e5e5 50%, #1a1a1a 50%);
  }

  .theme-system .preview-cursor {
    background: linear-gradient(90deg, #00ff88 50%, #007acc 50%);
  }

  .preview-header {
    height: 16px;
    display: flex;
    align-items: center;
    padding: 0 8px;
  }

  .preview-controls {
    display: flex;
    gap: 3px;
  }

  .control-dot {
    width: 4px;
    height: 4px;
    border-radius: 50%;
  }

  .control-dot.red {
    background: #ff5f57;
  }

  .control-dot.yellow {
    background: #ffbd2e;
  }

  .control-dot.green {
    background: #28ca42;
  }

  .preview-content {
    padding: 6px 8px;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .preview-line {
    height: 2px;
    opacity: 0.8;
  }

  .preview-line.long {
    width: 60px;
  }

  .preview-line.medium {
    width: 45px;
  }

  .preview-line.short {
    width: 30px;
  }

  .preview-cursor {
    width: 1px;
    height: 8px;
    margin-top: 2px;
    animation: blink 1.5s infinite;
  }

  @keyframes blink {
    0%,
    50% {
      opacity: 1;
    }
    51%,
    100% {
      opacity: 0;
    }
  }

  .theme-info {
    flex: 1;
    text-align: left;
  }

  .theme-name {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-200);
    margin: 0 0 4px 0;
  }

  .theme-desc {
    font-size: 14px;
    color: var(--text-400);
    margin: 0;
    line-height: 1.4;
  }

  .theme-check {
    width: 24px;
    height: 24px;
    color: var(--text-100);
    flex-shrink: 0;
  }

  .step-actions {
    width: 100%;
  }

  .continue-button {
    width: 100%;
    font-weight: 600;
    padding: 16px 32px;
  }

  @media (max-width: 480px) {
    .step-title {
      font-size: 28px;
    }

    .theme-option {
      padding: 16px;
      gap: 12px;
    }

    .theme-preview {
      width: 60px;
      height: 45px;
    }

    .theme-name {
      font-size: 16px;
    }

    .theme-desc {
      font-size: 13px;
    }
  }
</style>
