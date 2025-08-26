<script setup lang="ts">
  import { onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useAISettingsStore } from './store'
  import AIFeatureSettings from './AIFeatureSettings.vue'
  import AIModelConfig from './AIModelConfig.vue'

  const aiSettingsStore = useAISettingsStore()
  const { t } = useI18n()

  // 在父组件中统一加载设置（只在必要时加载）
  onMounted(async () => {
    // 使用新的初始化检查机制
    if (!aiSettingsStore.isInitialized && !aiSettingsStore.isLoading) {
      await aiSettingsStore.loadSettings()
    }
  })
</script>

<template>
  <div class="ai-settings">
    <!-- 错误状态 -->
    <div v-if="aiSettingsStore.error" class="error-state">
      <div class="error-icon">⚠️</div>
      <p>{{ t('settings.ai.load_error', { error: aiSettingsStore.error }) }}</p>
      <x-button variant="primary" @click="aiSettingsStore.loadSettings()">{{ t('common.retry') }}</x-button>
    </div>

    <!-- 正常内容 -->
    <template v-else>
      <div class="settings-group">
        <AIModelConfig />
      </div>
      <div class="settings-group">
        <AIFeatureSettings />
      </div>
    </template>
  </div>
</template>

<style scoped>
  .ai-settings {
    padding: 24px 28px;
    background: var(--bg-200);
    min-height: 100%;
  }

  .settings-group {
    margin-bottom: 32px;
    padding-bottom: 32px;
    border-bottom: 1px solid var(--border-300);
  }

  .settings-group:last-child {
    margin-bottom: 0;
    padding-bottom: 0;
    border-bottom: none;
  }

  .loading-state,
  .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 40px 32px;
    background: var(--bg-300);
    border-radius: 4px;
    text-align: center;
  }

  .loading-spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border-300);
    border-top: 2px solid var(--color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 16px;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error-icon {
    font-size: 28px;
    margin-bottom: 12px;
  }

  .error-state p {
    color: var(--text-200);
    font-size: 15px;
    margin-bottom: 16px;
  }

  .error-state :deep(.x-button) {
    background: var(--color-primary);
    border: 1px solid var(--color-primary);
    color: white;
    border-radius: 4px;
    padding: 8px 16px;
    font-size: 14px;
  }

  .error-state :deep(.x-button:hover) {
    background: var(--color-primary-hover);
  }
</style>
