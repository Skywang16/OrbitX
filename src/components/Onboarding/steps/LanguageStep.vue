<template>
  <div class="language-step">
    <div class="step-header">
      <h2 class="step-title">{{ t('settings.language.title') }}</h2>
      <p class="step-description">{{ t('settings.language.description') }}</p>
    </div>

    <div class="language-options">
      <button
        v-for="lang in languages"
        :key="lang.code"
        class="language-option"
        :class="{ selected: selectedLanguage === lang.code }"
        @click="selectLanguage(lang.code)"
      >
        <div class="language-info">
          <div class="language-name">{{ lang.name }}</div>
          <div class="language-native">{{ lang.nativeName }}</div>
        </div>
        <CheckIcon v-if="selectedLanguage === lang.code" class="language-check" />
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { getCurrentLocale, setLocale } from '@/i18n'
  import CheckIcon from './icons/CheckIcon.vue'

  const { t } = useI18n()

  const languages = [
    {
      code: 'zh-CN',
      name: '中文',
      nativeName: '简体中文',
    },
    {
      code: 'en-US',
      name: 'English',
      nativeName: 'English',
    },
  ]

  const selectedLanguage = computed(() => getCurrentLocale())
  const selectLanguage = (langCode: string) => setLocale(langCode)
</script>

<style scoped>
  .language-step {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 100%;
    max-width: 500px;
  }

  .step-header {
    text-align: center;
    margin-bottom: 40px;
  }

  .step-title {
    font-size: 32px;
    font-weight: 600;
    color: var(--text-100);
    margin: 0 0 12px 0;
  }

  .step-description {
    font-size: 16px;
    color: var(--text-400);
    margin: 0;
    line-height: 1.5;
  }

  .language-options {
    display: flex;
    flex-direction: column;
    gap: 16px;
    width: 100%;
    margin-bottom: 40px;
  }

  .language-option {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 20px;
    background: var(--bg-200);
    border: 2px solid var(--border-100);
    border-radius: 12px;
    cursor: pointer;
    transition: all 0.2s ease;
    position: relative;
  }

  .language-option:hover {
    border-color: var(--color-primary);
    background: var(--bg-300);
  }

  .language-option.selected {
    border-color: var(--color-primary);
    background: var(--bg-300);
  }

  .language-info {
    flex: 1;
    text-align: left;
  }

  .language-name {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-200);
    margin: 0 0 4px 0;
  }

  .language-native {
    font-size: 14px;
    color: var(--text-400);
    margin: 0;
    line-height: 1.4;
  }

  .language-check {
    width: 24px;
    height: 24px;
    color: var(--text-100);
    flex-shrink: 0;
  }
</style>
