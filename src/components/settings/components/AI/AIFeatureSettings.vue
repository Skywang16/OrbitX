<script setup lang="ts">
  import { useAISettingsStore } from './store'
  import type { AISettings } from '@/types'
  import { computed, ref } from 'vue'
  import { ai } from '@/api/ai'

  const aiSettingsStore = useAISettingsStore()

  // 用户前置提示词
  const userPrefixPrompt = ref<string>('')
  const isLoadingPrefix = ref(false)
  const isSavingPrefix = ref(false)

  // AI功能设置
  const aiFeatures = computed({
    get: () => aiSettingsStore.settings?.features || {},
    set: (value: AISettings['features']) => {
      aiSettingsStore.updateSettings({ features: value })
    },
  })

  // 更新特定功能设置
  const updateFeature = (featureName: keyof AISettings['features'], updates: Record<string, unknown>) => {
    if (!aiFeatures.value || !aiFeatures.value[featureName]) return

    aiFeatures.value = {
      ...aiFeatures.value,
      [featureName]: {
        ...aiFeatures.value[featureName],
        ...updates,
      },
    }
  }

  // 安全获取功能设置值
  const getFeatureValue = (featureId: string, settingKey: string): unknown => {
    const feature = aiFeatures.value[featureId as keyof typeof aiFeatures.value]
    if (!feature || typeof feature !== 'object') return undefined
    return (feature as Record<string, unknown>)[settingKey]
  }

  // 加载用户前置提示词
  const loadUserPrefixPrompt = async () => {
    isLoadingPrefix.value = true
    try {
      const prompt = await ai.getUserPrefixPrompt()
      userPrefixPrompt.value = prompt || ''
    } catch (error) {
      console.error('加载用户前置提示词失败:', error)
    } finally {
      isLoadingPrefix.value = false
    }
  }

  // 保存用户前置提示词
  const saveUserPrefixPrompt = async () => {
    isSavingPrefix.value = true
    try {
      const promptToSave = userPrefixPrompt.value.trim() || null
      await ai.setUserPrefixPrompt(promptToSave)
      // 可以添加成功提示
    } catch (error) {
      console.error('保存用户前置提示词失败:', error)
      // 可以添加错误提示
    } finally {
      isSavingPrefix.value = false
    }
  }

  // 组件挂载时加载前置提示词
  import { onMounted } from 'vue'
  onMounted(() => {
    loadUserPrefixPrompt()
  })

  // 功能配置项
  const featureConfigs = [
    {
      id: 'chat',
      title: 'AI 聊天功能',
      description: '配置AI聊天和代码解释功能',
      icon: 'message-circle',
      settings: [
        {
          key: 'enabled',
          label: '启用AI聊天',
          type: 'toggle',
          description: '启用AI聊天功能，可以与AI进行对话',
        },
        {
          key: 'model',
          label: '聊天模型',
          type: 'select',
          description: '选择用于聊天的AI模型',
          options: computed(() => aiSettingsStore.settings.models.map(m => ({ value: m.name, label: m.name }))),
        },
        {
          key: 'explanation',
          label: '代码解释功能',
          type: 'toggle',
          description: '启用后可以选中代码右键发送到聊天区进行解释',
        },
        {
          key: 'maxHistoryLength',
          label: '最大历史记录',
          type: 'range',
          description: '聊天历史记录的最大条数',
          min: 10,
          max: 1000,
          step: 10,
        },
        {
          key: 'contextWindowSize',
          label: '上下文窗口大小',
          type: 'range',
          description: '发送给AI的上下文token数量',
          min: 1000,
          max: 8000,
          step: 500,
        },
      ],
    },
  ]

  // 获取图标SVG
  const getIconSvg = (iconName: string) => {
    const icons: Record<string, string> = {
      'message-circle': `<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z"/>
      </svg>`,
      zap: `<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polygon points="13,2 3,14 12,14 11,22 21,10 12,10 13,2"/>
      </svg>`,

      'help-circle': `<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10"/>
        <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/>
        <point cx="12" cy="17"/>
      </svg>`,
      'alert-triangle': `<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
        <line x1="12" y1="9" x2="12" y2="13"/>
        <line x1="12" y1="17" x2="12.01" y2="17"/>
      </svg>`,
    }
    return icons[iconName] || ''
  }
</script>

<template>
  <div class="ai-feature-settings">
    <!-- 用户前置提示词设置 -->
    <div class="prefix-prompt-section">
      <div class="section-header">
        <div class="section-icon">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14,2 14,8 20,8" />
            <line x1="16" y1="13" x2="8" y2="13" />
            <line x1="16" y1="17" x2="8" y2="17" />
            <polyline points="10,9 9,9 8,9" />
          </svg>
        </div>
        <div class="section-info">
          <h3 class="section-title">用户前置提示词</h3>
          <p class="section-description">设置一个通用的前置提示词，它会自动添加到所有AI请求的前面</p>
        </div>
      </div>

      <div class="prefix-prompt-content">
        <div class="prompt-input-container">
          <textarea
            v-model="userPrefixPrompt"
            class="prompt-textarea"
            placeholder="在这里输入你的前置提示词，例如：请用中文回答所有问题..."
            rows="4"
            :disabled="isLoadingPrefix"
          ></textarea>
        </div>

        <div class="prompt-actions">
          <x-button
            variant="secondary"
            size="sm"
            @click="userPrefixPrompt = ''"
            :disabled="!userPrefixPrompt.trim() || isSavingPrefix"
          >
            清除
          </x-button>
          <x-button size="sm" @click="saveUserPrefixPrompt" :loading="isSavingPrefix" :disabled="isLoadingPrefix">
            保存
          </x-button>
        </div>
      </div>
    </div>

    <!-- 功能配置列表 -->
    <div class="feature-list">
      <div v-for="feature in featureConfigs" :key="feature.id" class="feature-card">
        <!-- 功能头部 -->
        <div class="feature-header">
          <div class="feature-icon" v-html="getIconSvg(feature.icon)"></div>
          <div class="feature-info">
            <h3 class="feature-title">{{ feature.title }}</h3>
            <p class="feature-description">{{ feature.description }}</p>
          </div>
          <div class="feature-status">
            <div
              class="status-indicator"
              :class="{
                enabled: getFeatureValue(feature.id, 'enabled'),
                disabled: !getFeatureValue(feature.id, 'enabled'),
              }"
            ></div>
          </div>
        </div>

        <!-- 功能设置 -->
        <div class="feature-settings">
          <div v-for="setting in feature.settings" :key="setting.key" class="setting-item">
            <!-- 开关类型 -->
            <div v-if="setting.type === 'toggle'" class="setting-toggle">
              <x-switch
                :model-value="getFeatureValue(feature.id, setting.key)"
                @update:model-value="
                  (newValue: any) =>
                    updateFeature(feature.id as keyof typeof aiFeatures, {
                      [setting.key]: newValue,
                    })
                "
              />
              <div class="toggle-text">
                <span class="setting-label">{{ setting.label }}</span>
                <span class="setting-description">{{ setting.description }}</span>
              </div>
            </div>

            <!-- 选择器类型 -->
            <div v-else-if="setting.type === 'select'" class="setting-select">
              <label class="setting-label">{{ setting.label }}</label>
              <x-select
                :model-value="getFeatureValue(feature.id, setting.key)"
                :options="setting.options"
                @update:model-value="
                  (newValue: any) =>
                    updateFeature(feature.id as keyof typeof aiFeatures, {
                      [setting.key]: newValue,
                    })
                "
              />
              <p class="setting-description">{{ setting.description }}</p>
            </div>

            <!-- 数字输入类型 -->
            <div v-else-if="setting.type === 'number'" class="setting-number">
              <div class="number-header">
                <label class="setting-label">{{ setting.label }}</label>
                <span class="current-value">
                  {{ getFeatureValue(feature.id, setting.key) }}
                </span>
              </div>
              <input
                :value="getFeatureValue(feature.id, setting.key)"
                type="range"
                class="range-input"
                :min="setting.min"
                :max="setting.max"
                :step="setting.step"
                @input="
                  updateFeature(feature.id as keyof typeof aiFeatures, {
                    [setting.key]: parseInt(($event.target as HTMLInputElement).value),
                  })
                "
              />
              <div class="range-labels">
                <span class="range-min">{{ setting.min }}</span>
                <span class="range-max">{{ setting.max }}</span>
              </div>
              <p class="setting-description">{{ setting.description }}</p>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 全局操作 -->
    <div class="global-actions">
      <x-button variant="secondary" @click="() => aiSettingsStore.resetToDefaults()">重置为默认</x-button>

      <x-button
        @click="
          () => {
            // 全部启用
            const allEnabled = { ...aiFeatures }
            Object.keys(allEnabled).forEach(key => {
              allEnabled[key as keyof typeof allEnabled].enabled = true
            })
            Object.assign(aiFeatures, allEnabled)
          }
        "
      >
        全部启用
      </x-button>
    </div>
  </div>
</template>

<style scoped>
  .ai-feature-settings {
    width: 100%;
  }

  /* 前置提示词设置样式 */
  .prefix-prompt-section {
    background-color: var(--color-background);
    border: 2px solid var(--border-color);
    border-radius: var(--border-radius-lg);
    padding: var(--spacing-md);
    margin-bottom: var(--spacing-lg);
  }

  .section-header {
    display: flex;
    align-items: flex-start;
    gap: var(--spacing-md);
    margin-bottom: var(--spacing-md);
  }

  .section-icon {
    flex-shrink: 0;
    color: var(--color-primary);
    margin-top: 2px;
  }

  .section-info {
    flex: 1;
  }

  .section-title {
    font-size: var(--font-size-md);
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 var(--spacing-xs) 0;
  }

  .section-description {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.5;
  }

  .prefix-prompt-content {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
  }

  .prompt-input-container {
    width: 100%;
  }

  .prompt-textarea {
    width: 100%;
    min-height: 100px;
    padding: var(--spacing-sm);
    border: 1px solid var(--border-color);
    border-radius: var(--border-radius);
    background-color: var(--color-background);
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    font-family: inherit;
    line-height: 1.5;
    resize: vertical;
    transition: border-color 0.2s ease;
  }

  .prompt-textarea:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px var(--color-primary-alpha);
  }

  .prompt-textarea:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .prompt-textarea::placeholder {
    color: var(--text-secondary);
  }

  .prompt-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--spacing-sm);
  }

  .feature-list {
    display: grid;
    gap: var(--spacing-md);
  }

  .feature-card {
    background-color: var(--color-background);
    border: 2px solid var(--border-color);
    border-radius: var(--border-radius-lg);
    padding: var(--spacing-sm) var(--spacing-md);
    transition: all 0.2s ease;
  }

  .feature-header {
    display: flex;
    align-items: flex-start;
    gap: var(--spacing-md);
    margin-bottom: var(--spacing-lg);
  }

  .feature-icon {
    flex-shrink: 0;
    color: var(--color-primary);
    margin-top: 2px;
  }

  .feature-info {
    flex: 1;
  }

  .feature-title {
    font-size: var(--font-size-md);
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 var(--spacing-xs) 0;
  }

  .feature-description {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.5;
  }

  .feature-status {
    flex-shrink: 0;
  }

  .status-indicator {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    transition: all 0.2s ease;
  }

  .status-indicator.enabled {
    background-color: var(--color-success);
    box-shadow: 0 0 0 2px var(--color-success-alpha);
  }

  .status-indicator.disabled {
    background-color: var(--text-secondary);
  }

  .feature-settings {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
  }

  .setting-item {
    padding: var(--spacing-sm);
  }

  .setting-toggle {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
  }

  .toggle-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .setting-label {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-primary);
  }

  .setting-description {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    line-height: 1.4;
    margin: var(--spacing-xs) 0 0 0;
  }

  .setting-select {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
  }

  .setting-select .setting-label {
    margin-bottom: var(--spacing-xs);
  }

  .setting-number {
    width: 100%;
  }

  .number-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--spacing-sm);
  }

  .current-value {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--color-primary);
    background-color: var(--color-primary-alpha);
    padding: 2px var(--spacing-xs);
    border-radius: var(--border-radius);
  }

  .range-input {
    width: 100%;
    height: 6px;
    background-color: var(--color-background-hover);
    border-radius: 3px;
    outline: none;
    margin-bottom: var(--spacing-xs);
    cursor: pointer;
  }

  .range-input::-webkit-slider-thumb {
    appearance: none;
    width: 18px;
    height: 18px;
    background-color: var(--color-primary);
    border-radius: 50%;
    cursor: pointer;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
  }

  .range-input::-moz-range-thumb {
    width: 18px;
    height: 18px;
    background-color: var(--color-primary);
    border-radius: 50%;
    cursor: pointer;
    border: none;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
  }

  .range-labels {
    display: flex;
    justify-content: space-between;
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    margin-bottom: var(--spacing-xs);
  }

  .global-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--spacing-md);
    padding-top: var(--spacing-lg);
    border-top: 1px solid var(--border-color);
  }

  .action-button {
    padding: var(--spacing-sm) var(--spacing-lg);
    border-radius: var(--border-radius);
    font-size: var(--font-size-sm);
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
    border: 1px solid;
  }

  .action-button.secondary {
    background-color: transparent;
    border-color: var(--border-color);
    color: var(--text-primary);
  }

  .action-button.secondary:hover {
    background-color: var(--color-background-hover);
  }

  .action-button.primary {
    background-color: var(--color-primary);
    border-color: var(--color-primary);
    color: white;
  }

  .action-button.primary:hover {
    background-color: var(--color-primary-hover);
  }
</style>
