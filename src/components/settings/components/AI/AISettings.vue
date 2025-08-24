<script setup lang="ts">
  import { onMounted } from 'vue'
  import { useAISettingsStore } from './store'
  import AIFeatureSettings from './AIFeatureSettings.vue'
  import AIModelConfig from './AIModelConfig.vue'

  const aiSettingsStore = useAISettingsStore()

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
      <p>加载AI设置失败: {{ aiSettingsStore.error }}</p>
      <x-button variant="primary" @click="aiSettingsStore.loadSettings()">重试</x-button>
    </div>

    <!-- 正常内容 -->
    <template v-else>
      <div class="settings-card">
        <AIModelConfig />
      </div>
      <div class="settings-card">
        <AIFeatureSettings />
      </div>
    </template>
  </div>
</template>

<style scoped>
  .ai-settings {
    padding: 24px;
    background: var(--bg-600);
    min-height: 100vh;
  }

  .settings-card {
    margin-bottom: 32px;
  }

  .loading-state,
  .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 48px 24px;
    background: var(--bg-500);
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
    font-size: 32px;
    margin-bottom: 12px;
  }

  .error-state p {
    color: var(--error-text);
    font-size: 14px;
    margin-bottom: 16px;
  }

  .error-state :deep(.x-button) {
    background: var(--color-primary);
    border: 1px solid var(--color-primary);
    color: white;
    border-radius: 4px;
    padding: 8px 16px;
    font-size: 13px;
  }

  .error-state :deep(.x-button:hover) {
    background: var(--color-primary-hover);
  }
</style>
