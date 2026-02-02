<script setup lang="ts">
  import { useI18n } from 'vue-i18n'

  import { useAISettingsStore } from './store'
  import AIModelConfig from './components/AIModelConfig.vue'

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
  <div class="ai-settings">
    <!-- Model Configuration Section -->
    <div class="settings-section">
      <AIModelConfig />
    </div>
  </div>
</template>

<style scoped>
  .ai-settings {
    display: flex;
    flex-direction: column;
    gap: 32px;
  }

  .settings-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
</style>
