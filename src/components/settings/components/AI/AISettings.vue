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
    max-width: 800px;
    padding: var(--spacing-lg);
  }

  .settings-card {
    background-color: var(--color-primary-alpha);
    border-radius: var(--border-radius);
    padding: var(--spacing-lg);
    margin-bottom: var(--spacing-lg);
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
</style>
