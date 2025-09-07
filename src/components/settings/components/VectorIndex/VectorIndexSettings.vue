<script setup lang="ts">
  import { onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { createMessage } from '@/ui'
  import { useVectorIndexSettingsStore } from './store'
  import VectorIndexConnectionConfig from './VectorIndexConnectionConfig.vue'
  import VectorIndexManagement from './VectorIndexManagement.vue'

  const vectorIndexSettingsStore = useVectorIndexSettingsStore()
  const { t } = useI18n()

  onMounted(async () => {
    if (!vectorIndexSettingsStore.isInitialized && !vectorIndexSettingsStore.isLoading) {
      try {
        await vectorIndexSettingsStore.loadSettings()
      } catch (error) {
        createMessage.error(
          t('settings.vectorIndex.load_error', { error: error instanceof Error ? error.message : String(error) })
        )
      }
    }
  })
</script>

<template>
  <div class="settings-group">
    <h2 class="settings-section-title">{{ t('settings.vectorIndex.title') }}</h2>

    <div class="settings-group">
      <VectorIndexConnectionConfig />
    </div>
    <div class="settings-group">
      <VectorIndexManagement />
    </div>
  </div>
</template>

<style scoped></style>
