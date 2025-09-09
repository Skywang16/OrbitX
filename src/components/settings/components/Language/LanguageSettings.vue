<script setup lang="ts">
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { getCurrentLocale, setLocale } from '@/i18n'
  import { XSelect } from '@/ui'
  import SettingsCard from '../../SettingsCard.vue'

  const { t } = useI18n()

  const currentLocale = computed(() => getCurrentLocale())

  const languageOptions = computed(() => [
    { label: t('language.chinese'), value: 'zh-CN' },
    { label: t('language.english'), value: 'en-US' },
  ])

  const handleLanguageChange = async (value: string | number | (string | number)[] | null) => {
    if (typeof value === 'string') {
      await setLocale(value)
    }
  }
</script>

<template>
  <div class="settings-group">
    <h2 class="settings-section-title">{{ t('settings.language.title') }}</h2>

    <div class="settings-group">
      <h3 class="settings-group-title">{{ t('language.title') }}</h3>

      <SettingsCard>
        <div class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">{{ t('language.interface_language') }}</div>
            <div class="settings-description">{{ t('settings.language.description') }}</div>
          </div>
          <div class="settings-item-control">
            <div class="language-switch">
              <x-select
                :model-value="currentLocale"
                :options="languageOptions"
                @update:model-value="handleLanguageChange"
                placeholder="Language"
              />
            </div>
          </div>
        </div>
      </SettingsCard>
    </div>
  </div>
</template>

<style scoped></style>
