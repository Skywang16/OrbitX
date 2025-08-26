<script setup lang="ts">
  import { useI18n } from 'vue-i18n'
  import LanguageSwitch from '@/components/ui/LanguageSwitch.vue'

  const { t } = useI18n()
</script>

<template>
  <div class="language-settings">
    <div class="settings-group">
      <h3 class="section-title">{{ t('language.title') }}</h3>
      <div class="language-item">
        <span class="language-label">界面语言</span>
        <LanguageSwitch />
      </div>
    </div>
  </div>
</template>

<style scoped>
  .language-settings {
    padding: 32px 28px;
    background: var(--bg-200);
    min-height: 100%;
  }

  .settings-group {
    margin-bottom: 32px;
    padding-bottom: 32px;
    border-bottom: 1px solid var(--border-300);
  }

  .settings-group:last-child {
    margin-bottom: 0;
    padding-bottom: 0;
    border-bottom: none;
  }

  .section-title {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-100);
    margin: 0 0 16px 0;
    padding: 0;
  }

  .language-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .language-label {
    font-size: 15px;
    font-weight: 500;
    color: var(--text-200);
  }
</style>
