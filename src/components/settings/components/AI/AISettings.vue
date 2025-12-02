<script setup lang="ts">
  import { useI18n } from 'vue-i18n'

  import { useAISettingsStore } from './store'
  import AIFeatureSettings from './components/AIFeatureSettings.vue'
  import AIModelConfig from './components/AIModelConfig.vue'
  import EmbeddingModelConfig from './components/EmbeddingModelConfig.vue'

  const aiSettingsStore = useAISettingsStore()
  const { t } = useI18n()

  // 初始化方法，供外部调用
  const init = async () => {
    if (!aiSettingsStore.isInitialized && !aiSettingsStore.isLoading) {
      await aiSettingsStore.loadSettings()
    }
  }

  // 暴露初始化方法给父组件
  defineExpose({
    init,
  })
</script>

<template>
  <div class="settings-group">
    <h2 class="settings-section-title">{{ t('settings.ai.title') }}</h2>

    <div class="settings-group">
      <AIModelConfig />
    </div>
    <div class="settings-group">
      <EmbeddingModelConfig />
    </div>
    <div class="settings-group">
      <AIFeatureSettings />
    </div>
  </div>
</template>

<style scoped></style>
