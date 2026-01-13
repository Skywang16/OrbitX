<script setup lang="ts">
  import { onMounted, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { settingsApi } from '@/api'
  import { debounce } from 'lodash-es'
  import SettingsCard from '../../../SettingsCard.vue'

  const { t } = useI18n()

  const globalRules = ref<string>('')
  const isLoadingRules = ref(false)
  const globalSettings = ref<Awaited<ReturnType<typeof settingsApi.getGlobal>> | null>(null)

  const loadGlobalRules = async () => {
    isLoadingRules.value = true
    try {
      const settings = await settingsApi.getGlobal()
      globalSettings.value = settings
      globalRules.value = settings.rules?.content || ''
    } catch (error) {
      // Error handled silently
    } finally {
      isLoadingRules.value = false
    }
  }

  const saveGlobalRules = async (value: string) => {
    try {
      const rulesToSave = value.trim()
      const settings = globalSettings.value || (await settingsApi.getGlobal())
      settings.rules = settings.rules || { content: '', rulesFiles: [] }
      settings.rules.content = rulesToSave
      await settingsApi.updateGlobal(settings)
      globalSettings.value = settings
    } catch (error) {
      // Error handled silently
    }
  }

  const debouncedSaveGlobalRules = debounce((newValue: string) => {
    saveGlobalRules(newValue)
  }, 500)

  watch(globalRules, debouncedSaveGlobalRules)

  onMounted(() => {
    loadGlobalRules()
  })
</script>

<template>
  <div class="settings-group">
    <h3 class="settings-group-title">{{ t('ai_feature.user_rules') }}</h3>

    <SettingsCard>
      <div class="settings-item">
        <div class="settings-item-header">
          <div class="settings-label">{{ t('ai_feature.user_rules') }}</div>
          <div class="settings-description">{{ t('ai_feature.rules_placeholder') }}</div>
        </div>
      </div>

      <div style="padding: 20px">
        <textarea
          v-model="globalRules"
          class="settings-textarea"
          :placeholder="t('ai_feature.rules_placeholder')"
          :aria-label="t('ai_feature.user_rules')"
          rows="4"
          :disabled="isLoadingRules"
        ></textarea>
      </div>
    </SettingsCard>
  </div>
</template>

<style scoped></style>
