<script setup lang="ts">
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { getCurrentLocale, setLocale } from '@/i18n'
  import SettingsCard from '../../SettingsCard.vue'

  const { t } = useI18n()

  const currentLocale = computed(() => getCurrentLocale())

  const languages = [
    { code: 'en-US', name: 'English', nativeName: 'English', flag: '🇺🇸' },
    { code: 'zh-CN', name: 'Chinese (Simplified)', nativeName: '简体中文', flag: '🇨🇳' },
  ]

  const handleLanguageChange = async (code: string) => {
    await setLocale(code)
  }
</script>

<template>
  <div class="language-settings">
    <!-- Language Selection -->
    <div class="settings-section">
      <h3 class="settings-section-title">{{ t('settings.language.interface_language') || 'Interface Language' }}</h3>

      <SettingsCard>
        <div
          v-for="lang in languages"
          :key="lang.code"
          class="settings-item clickable language-item"
          :class="{ active: currentLocale === lang.code }"
          @click="handleLanguageChange(lang.code)"
        >
          <div class="settings-item-header">
            <div class="language-info">
              <span class="language-flag">{{ lang.flag }}</span>
              <div class="language-text">
                <div class="settings-label">{{ lang.nativeName }}</div>
                <div class="settings-description">{{ lang.name }}</div>
              </div>
            </div>
          </div>
          <div class="settings-item-control">
            <div class="radio-indicator" :class="{ checked: currentLocale === lang.code }">
              <svg
                v-if="currentLocale === lang.code"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="3"
                class="check-icon"
              >
                <polyline points="20 6 9 17 4 12" />
              </svg>
            </div>
          </div>
        </div>
      </SettingsCard>
    </div>

    <!-- Language Info -->
    <div class="settings-section">
      <div class="language-info-box">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="info-icon">
          <circle cx="12" cy="12" r="10" />
          <line x1="12" y1="16" x2="12" y2="12" />
          <line x1="12" y1="8" x2="12.01" y2="8" />
        </svg>
        <p>{{ t('settings.language.change_note') || 'Language changes will take effect immediately.' }}</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .language-settings {
    display: flex;
    flex-direction: column;
    gap: 32px;
  }

  .settings-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  /* Language Item */
  .language-item {
    padding: 16px 20px !important;
  }

  .language-info {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .language-flag {
    font-size: 24px;
    line-height: 1;
  }

  .language-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .language-text .settings-label {
    font-size: 14px;
  }

  .language-text .settings-description {
    margin: 0;
  }

  /* Radio Indicator */
  .radio-indicator {
    width: 22px;
    height: 22px;
    border: 2px solid var(--border-300);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
  }

  .radio-indicator.checked {
    border-color: var(--color-primary);
    background: var(--color-primary);
  }

  .check-icon {
    width: 14px;
    height: 14px;
    color: white;
  }

  /* Info Box */
  .language-info-box {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 14px 16px;
    background: color-mix(in srgb, var(--color-primary) 8%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-primary) 20%, transparent);
    border-radius: 10px;
  }

  .info-icon {
    flex-shrink: 0;
    width: 18px;
    height: 18px;
    color: var(--color-primary);
  }

  .language-info-box p {
    font-size: 13px;
    color: var(--text-200);
    line-height: 1.5;
    margin: 0;
  }
</style>
