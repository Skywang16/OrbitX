<script setup lang="ts">
  import { onMounted, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { aiApi } from '@/api'
  import { debounce } from 'lodash-es'
  import SettingsCard from '../../../SettingsCard.vue'

  const { t } = useI18n()

  const userRules = ref<string>('')
  const isLoadingRules = ref(false)

  const loadUserRules = async () => {
    isLoadingRules.value = true
    try {
      const rules = await aiApi.getUserRules()
      userRules.value = rules || ''
    } catch (error) {
      // Error handled silently
    } finally {
      isLoadingRules.value = false
    }
  }

  const saveUserRules = async (value: string) => {
    try {
      const rulesToSave = value.trim() || null
      await aiApi.setUserRules(rulesToSave)
    } catch (error) {
      // Error handled silently
    }
  }


  const debouncedSaveUserRules = debounce((newValue: string) => {
    saveUserRules(newValue)
  }, 500)

  watch(userRules, debouncedSaveUserRules)

  onMounted(() => {
    loadUserRules()
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
          v-model="userRules"
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
