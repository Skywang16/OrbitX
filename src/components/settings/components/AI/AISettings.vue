<script setup lang="ts">
  import { onMounted, ref } from 'vue'
  import { useAISettingsStore } from './store'
  import AIFeatureSettings from './AIFeatureSettings.vue'
  import AIModelConfig from './AIModelConfig.vue'

  const aiSettingsStore = useAISettingsStore()

  // 当前活动的子设置页面
  const activeSubSection = ref<string>('models')

  // 在父组件中统一加载设置（只在必要时加载）
  onMounted(async () => {
    // 如果数据已经存在且不在加载中，就不重复加载
    if (!aiSettingsStore.settings.models.length && !aiSettingsStore.isLoading) {
      await aiSettingsStore.loadSettings()
    }
  })

  // 子设置导航项
  const subSections = [
    {
      id: 'models',
      label: '模型配置',
      description: '管理AI模型连接和配置',
      icon: 'settings',
    },
    {
      id: 'features',
      label: '功能设置',
      description: '配置AI功能开关和参数',
      icon: 'toggles',
    },
  ]

  // AI设置状态（暂时未使用，为后续功能预留）
  // const aiSettings = computed(() => settingsStore.settings.ai)

  // 处理子设置切换
  const handleSubSectionChange = (sectionId: string) => {
    activeSubSection.value = sectionId
  }

  // 获取图标SVG
  const getIconSvg = (iconName: string) => {
    const icons: Record<string, string> = {
      settings: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="3"/>
        <path d="M12 1v6m0 6v6m11-7h-6m-6 0H1m11-7a4 4 0 0 0-8 0m8 14a4 4 0 0 0-8 0"/>
      </svg>`,
      toggles: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <rect width="20" height="12" x="2" y="6" rx="6" ry="6"/>
        <circle cx="8" cy="12" r="2"/>
      </svg>`,
      shield: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
      </svg>`,
    }
    return icons[iconName] || ''
  }
</script>

<template>
  <div class="ai-settings">
    <!-- 子设置导航 -->
    <div class="sub-navigation">
      <div
        v-for="section in subSections"
        :key="section.id"
        class="sub-nav-item"
        :class="{ active: section.id === activeSubSection }"
        @click="handleSubSectionChange(section.id)"
      >
        <div class="sub-nav-icon" v-html="getIconSvg(section.icon)"></div>
        <div class="sub-nav-content">
          <div class="sub-nav-label">{{ section.label }}</div>
          <div class="sub-nav-description">{{ section.description }}</div>
        </div>
      </div>
    </div>

    <!-- 设置内容区域 -->
    <div class="settings-content">
      <!-- 错误状态 -->
      <div v-if="aiSettingsStore.error" class="error-state">
        <div class="error-icon">⚠️</div>
        <p>加载AI设置失败: {{ aiSettingsStore.error }}</p>
        <x-button variant="primary" @click="aiSettingsStore.loadSettings()">重试</x-button>
      </div>

      <!-- 正常内容 -->
      <template v-else>
        <!-- 模型配置 -->
        <AIModelConfig v-if="activeSubSection === 'models'" />

        <!-- 功能设置 -->
        <AIFeatureSettings v-if="activeSubSection === 'features'" />
      </template>
    </div>
  </div>
</template>

<style scoped>
  .ai-settings {
    max-width: 800px;
    padding: var(--spacing-lg);
  }

  .settings-header {
    margin-bottom: var(--spacing-xl);
  }

  .settings-title {
    font-size: var(--font-size-xl);
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 var(--spacing-xs) 0;
  }

  .settings-description {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.5;
  }

  .sub-navigation {
    display: flex;
    gap: var(--spacing-sm);
    margin-bottom: var(--spacing-xl);
    padding: var(--spacing-xs);
    background-color: var(--color-background-secondary);
    border-radius: var(--border-radius);
    border: 1px solid var(--border-color);
  }

  .sub-nav-item {
    flex: 1;
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-md);
    cursor: pointer;
    transition: all 0.2s ease;
    border-radius: var(--border-radius);
    background-color: transparent;
  }

  .sub-nav-item:hover {
    background-color: var(--color-background-hover);
  }

  .sub-nav-item.active {
    background-color: var(--color-primary-alpha);
    border: 1px solid var(--color-primary);
  }

  .sub-nav-item.active .sub-nav-label {
    color: var(--color-primary);
    font-weight: 600;
  }

  .sub-nav-item.active .sub-nav-icon {
    color: var(--color-primary);
  }

  .sub-nav-icon {
    flex-shrink: 0;
    color: var(--text-secondary);
    transition: color 0.2s ease;
  }

  .sub-nav-content {
    flex: 1;
    min-width: 0;
  }

  .sub-nav-label {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 2px;
  }

  .sub-nav-description {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    line-height: 1.3;
  }

  .settings-content {
    background-color: var(--color-background);
    border-radius: var(--border-radius);
  }

  .loading-state,
  .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--spacing-xl);
    text-align: center;
  }

  .loading-spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--border-color);
    border-top: 3px solid var(--color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: var(--spacing-md);
  }

  @keyframes spin {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }

  .error-icon {
    font-size: 2rem;
    margin-bottom: var(--spacing-sm);
  }

  /* 移除原有按钮样式，使用组件库样式 */
</style>
