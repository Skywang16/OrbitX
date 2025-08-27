<script setup lang="ts">
  import { useI18n } from 'vue-i18n'
  import LanguageSwitch from '@/components/ui/LanguageSwitch.vue'

  const { t } = useI18n()
</script>

<template>
  <div class="settings-group">
    <h2 class="settings-section-title">{{ t('settings.language.title') }}</h2>

    <div class="settings-group">
      <h3 class="settings-group-title">{{ t('language.title') }}</h3>

      <div class="settings-item">
        <div class="settings-item-header">
          <div class="settings-label">{{ t('language.interface_language') }}</div>
          <div class="settings-description">{{ t('settings.language.description') }}</div>
        </div>
        <div class="settings-item-control">
          <LanguageSwitch />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped></style>
