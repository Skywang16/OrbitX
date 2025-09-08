<script setup lang="ts">
  import { onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { handleErrorWithMessage } from '@/utils/errorHandler'
  import { useAISettingsStore } from './store'
  import AIFeatureSettings from './AIFeatureSettings.vue'
  import AIModelConfig from './AIModelConfig.vue'

  const aiSettingsStore = useAISettingsStore()
  const { t } = useI18n()

  onMounted(async () => {
    if (!aiSettingsStore.isInitialized && !aiSettingsStore.isLoading) {
      try {
        await aiSettingsStore.loadSettings()
      } catch (error) {
        handleErrorWithMessage(
          error,
          t('settings.ai.load_error', { error: error instanceof Error ? error.message : String(error) })
        )
      }
    }
  })
</script>

<template>
  <div class="settings-group">
    <h2 class="settings-section-title">{{ t('settings.ai.title') }}</h2>

    <div class="settings-group">
      <AIModelConfig />
    </div>
    <div class="settings-group">
      <AIFeatureSettings />
    </div>
  </div>
</template>

<style scoped></style>
