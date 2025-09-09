<script setup lang="ts">
  import { onMounted, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { aiApi } from '@/api'
  import { debounce } from 'lodash-es'
  import SettingsCard from '../../SettingsCard.vue'

  const { t } = useI18n()

  const userPrefixPrompt = ref<string>('')
  const isLoadingPrefix = ref(false)

  const loadUserPrefixPrompt = async () => {
    isLoadingPrefix.value = true
    try {
      const prompt = await aiApi.getUserPrefixPrompt()
      userPrefixPrompt.value = prompt || ''
    } catch (error) {
      // Error handled silently
    } finally {
      isLoadingPrefix.value = false
    }
  }

  const saveUserPrefixPrompt = async (value: string) => {
    try {
      const promptToSave = value.trim() || null
      await aiApi.setUserPrefixPrompt(promptToSave)
    } catch (error) {
      // Error handled silently
    }
  }

  const debouncedSave = debounce((newValue: string) => {
    saveUserPrefixPrompt(newValue)
  }, 500)

  watch(userPrefixPrompt, debouncedSave)

  onMounted(() => {
    loadUserPrefixPrompt()
  })
</script>

<template>
  <div class="settings-group">
    <h3 class="settings-group-title">{{ t('ai_feature.user_system_prompt') }}</h3>

    <SettingsCard>
      <div class="settings-item">
        <div class="settings-item-header">
          <div class="settings-label">{{ t('ai_feature.user_system_prompt') }}</div>
          <div class="settings-description">{{ t('ai_feature.prompt_placeholder') }}</div>
        </div>
      </div>

      <div style="padding: 0 20px 20px 20px">
        <textarea
          v-model="userPrefixPrompt"
          class="settings-textarea"
          :placeholder="t('ai_feature.prompt_placeholder')"
          :aria-label="t('ai_feature.user_system_prompt')"
          rows="4"
          :disabled="isLoadingPrefix"
        ></textarea>
      </div>
    </SettingsCard>
  </div>
</template>

<style scoped></style>
