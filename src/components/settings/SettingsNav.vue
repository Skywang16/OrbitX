<script setup lang="ts">
  import { computed, ref } from 'vue'
  import { useI18n } from 'vue-i18n'

  interface Props {
    activeSection?: string
  }

  interface Emits {
    (e: 'change', section: string): void
  }

  const props = withDefaults(defineProps<Props>(), {
    activeSection: 'ai',
  })

  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const searchQuery = ref('')

  const navigationItems = computed(() => [
    {
      id: 'general',
      label: t('settings.general.title'),
      icon: 'settings',
      description: t('settings.general.description'),
    },
    {
      id: 'ai',
      label: t('settings.ai.title'),
      icon: 'brain',
      description: t('settings.ai.description'),
    },
    {
      id: 'theme',
      label: t('settings.theme.title'),
      icon: 'palette',
      description: t('settings.theme.description'),
    },
    {
      id: 'shortcuts',
      label: t('settings.shortcuts.title'),
      icon: 'keyboard',
      description: t('settings.shortcuts.description'),
    },
    {
      id: 'language',
      label: t('language.title'),
      icon: 'globe',
      description: t('settings.language.description'),
    },
  ])

  const handleItemClick = (sectionId: string) => {
    if (sectionId !== props.activeSection) {
      emit('change', sectionId)
    }
  }
  const filteredNavigationItems = computed(() => {
    if (!searchQuery.value) {
      return navigationItems.value
    }

    const query = searchQuery.value.toLowerCase()
    return navigationItems.value.filter(
      item => item.label.toLowerCase().includes(query) || item.description.toLowerCase().includes(query)
    )
  })

  const getIconSvg = (iconName: string) => {
    const icons: Record<string, string> = {
      palette: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="13.5" cy="6.5" r=".5"/>
        <circle cx="17.5" cy="10.5" r=".5"/>
        <circle cx="8.5" cy="7.5" r=".5"/>
        <circle cx="6.5" cy="12.5" r=".5"/>
        <path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z"/>
      </svg>`,
      brain: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M9.5 2A2.5 2.5 0 0 1 12 4.5v15a2.5 2.5 0 0 1-4.96.44 2.5 2.5 0 0 1-2.96-3.08 3 3 0 0 1-.34-5.58 2.5 2.5 0 0 1 1.32-4.24 2.5 2.5 0 0 1 1.98-3A2.5 2.5 0 0 1 9.5 2Z"/>
        <path d="M14.5 2A2.5 2.5 0 0 0 12 4.5v15a2.5 2.5 0 0 0 4.96.44 2.5 2.5 0 0 0 2.96-3.08 3 3 0 0 0 .34-5.58 2.5 2.5 0 0 0-1.32-4.24 2.5 2.5 0 0 0-1.98-3A2.5 2.5 0 0 0 14.5 2Z"/>
      </svg>`,
      keyboard: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <rect x="2" y="6" width="20" height="12" rx="2"/>
        <circle cx="7" cy="12" r="1"/>
        <circle cx="12" cy="12" r="1"/>
        <circle cx="17" cy="12" r="1"/>
        <circle cx="7" cy="16" r="1"/>
        <circle cx="12" cy="16" r="1"/>
        <circle cx="17" cy="16" r="1"/>
      </svg>`,
      globe: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10"/>
        <line x1="2" y1="12" x2="22" y2="12"/>
        <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
      </svg>`,
      settings: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/>
        <circle cx="12" cy="12" r="3"/>
      </svg>`,
    }
    return icons[iconName] || ''
  }
</script>

<template>
  <nav class="settings-navigation">
    <div class="settings-navigation-header">
      <x-search-input v-model="searchQuery" :placeholder="t('settings.search_placeholder')" />
    </div>

    <ul class="settings-navigation-list">
      <li
        v-for="item in filteredNavigationItems"
        :key="item.id"
        class="settings-navigation-item"
        :class="{ active: item.id === activeSection }"
        @click="handleItemClick(item.id)"
      >
        <div class="settings-navigation-icon" v-html="getIconSvg(item.icon)"></div>
        <div class="settings-navigation-content">
          <div class="settings-navigation-label">
            {{ item.label }}
          </div>
        </div>
      </li>
    </ul>
  </nav>
</template>

<style scoped>
  .settings-navigation {
    background: var(--bg-200);
    height: 100%;
  }

  .settings-navigation-header {
    padding: 16px;
    border-bottom: 1px solid var(--border-200);
    background: var(--bg-200);
  }

  .settings-navigation-list {
    list-style: none;
    padding: 12px 8px;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .settings-navigation-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border-radius: var(--border-radius-lg);
    cursor: pointer;
    transition:
      background-color 0.1s ease,
      border-color 0.1s ease;
    position: relative;
    background: transparent;
    border: none;
  }

  .settings-navigation-item:hover {
    background: var(--bg-300);
  }

  .settings-navigation-item.active {
    background: var(--color-primary-alpha);
    color: var(--color-primary);
    box-shadow: 0 2px 12px rgba(var(--color-primary-rgb), 0.2);
  }

  .settings-navigation-item.active::before {
    content: '';
    position: absolute;
    left: 0;
    top: 50%;
    transform: translateY(-50%);
    width: 3px;
    height: 20px;
    background: var(--color-primary);
    border-radius: 0 2px 2px 0;
  }

  .settings-navigation-icon {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    opacity: 0.7;
    transition: opacity 0.2s ease;
  }

  .settings-navigation-item:hover .settings-navigation-icon,
  .settings-navigation-item.active .settings-navigation-icon {
    opacity: 1;
  }

  .settings-navigation-content {
    flex: 1;
    min-width: 0;
  }

  .settings-navigation-label {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-200);
    display: flex;
    align-items: center;
    gap: 8px;
    transition: color 0.2s ease;
  }

  .settings-navigation-item:hover .settings-navigation-label {
    color: var(--text-100);
  }

  .settings-navigation-item.active .settings-navigation-label {
    color: var(--color-primary);
    font-weight: 600;
  }

  .beta-label {
    font-size: 8px;
    color: var(--color-primary);
    background: var(--color-primary-alpha);
    padding: 2px 6px;
    border-radius: var(--border-radius-sm);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
</style>
