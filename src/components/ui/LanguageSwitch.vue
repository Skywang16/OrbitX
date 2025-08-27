<template>
  <div class="language-switch">
    <x-select
      :model-value="currentLocale"
      :options="languageOptions"
      @update:model-value="handleLanguageChange"
      placeholder="Language"
    />
  </div>
</template>

<script setup lang="ts">
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { getCurrentLocale, setLocale } from '@/i18n'
  import { XSelect } from '@/ui'

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

<style scoped>
  .language-switch {
    min-width: 120px;
  }
</style>
