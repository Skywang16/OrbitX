<script setup lang="ts">
  import { useI18n } from 'vue-i18n'
  import LanguageSwitch from '@/components/ui/LanguageSwitch.vue'

  const { t } = useI18n()
</script>

<template>
  <div class="language-settings">
    <div class="settings-section">
      <h3 class="section-title">{{ t('language.title') }}</h3>
      <div class="section-content">
        <div class="language-switch-container">
          <LanguageSwitch />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .language-settings {
    padding: 24px 28px;
    background: var(--bg-200);
    min-height: 100%;
  }

  .settings-section {
    margin-bottom: 32px;
    padding-bottom: 32px;
    border-bottom: 1px solid var(--border-300);
  }

  .settings-section:last-child {
    margin-bottom: 0;
    padding-bottom: 0;
    border-bottom: none;
  }

  .section-title {
    font-size: var(--font-size-lg);
    font-weight: 600;
    color: var(--text-100);
    margin-bottom: 16px;
    margin-top: 0;
  }

  .section-content {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .language-switch-container {
    max-width: 200px;
  }
</style>
