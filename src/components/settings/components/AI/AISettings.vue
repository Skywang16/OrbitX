<script setup lang="ts">
  import { onMounted } from 'vue'

  import { useAISettingsStore } from './store'
  import AIModelConfig from './components/AIModelConfig.vue'

  const aiSettingsStore = useAISettingsStore()

  // 初始化方法
  const init = async () => {
    if (!aiSettingsStore.isInitialized && !aiSettingsStore.isLoading) {
      await aiSettingsStore.loadSettings()
    }
  }

  // 组件挂载时自动初始化
  onMounted(async () => {
    await init()
  })

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
