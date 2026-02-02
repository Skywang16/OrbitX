<script setup lang="ts">
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { getCurrentLocale, setLocale } from '@/i18n'
  import SettingsCard from '../../SettingsCard.vue'

  const { t } = useI18n()

  const currentLocale = computed(() => getCurrentLocale())

  const languages = [
    { code: 'en-US', name: 'English', nativeName: 'English' },
    { code: 'zh-CN', name: 'Chinese (Simplified)', nativeName: '简体中文' },
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
            <div class="settings-label">{{ lang.nativeName }}</div>
            <div class="settings-description">{{ lang.name }}</div>
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
</style>
