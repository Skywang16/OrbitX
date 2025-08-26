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
      palette: `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="13.5" cy="6.5" r=".5"/>
        <circle cx="17.5" cy="10.5" r=".5"/>
        <circle cx="8.5" cy="7.5" r=".5"/>
        <circle cx="6.5" cy="12.5" r=".5"/>
        <path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z"/>
      </svg>`,
      brain: `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M9.5 2A2.5 2.5 0 0 1 12 4.5v15a2.5 2.5 0 0 1-4.96.44 2.5 2.5 0 0 1-2.96-3.08 3 3 0 0 1-.34-5.58 2.5 2.5 0 0 1 1.32-4.24 2.5 2.5 0 0 1 1.98-3A2.5 2.5 0 0 1 9.5 2Z"/>
        <path d="M14.5 2A2.5 2.5 0 0 0 12 4.5v15a2.5 2.5 0 0 0 4.96.44 2.5 2.5 0 0 0 2.96-3.08 3 3 0 0 0 .34-5.58 2.5 2.5 0 0 0-1.32-4.24 2.5 2.5 0 0 0-1.98-3A2.5 2.5 0 0 0 14.5 2Z"/>
      </svg>`,
      keyboard: `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <rect x="2" y="6" width="20" height="12" rx="2"/>
        <circle cx="7" cy="12" r="1"/>
        <circle cx="12" cy="12" r="1"/>
        <circle cx="17" cy="12" r="1"/>
        <circle cx="7" cy="16" r="1"/>
        <circle cx="12" cy="16" r="1"/>
        <circle cx="17" cy="16" r="1"/>
      </svg>`,
      globe: `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10"/>
        <line x1="2" y1="12" x2="22" y2="12"/>
        <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
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
          <div class="settings-navigation-label">{{ item.label }}</div>
        </div>
      </li>
    </ul>
  </nav>
</template>

<style scoped>
  /* 响应式设计 */
  @media (max-width: 480px) {
    .settings-navigation {
      padding: 8px 0;
    }

    .settings-navigation-header {
      padding: 8px 12px;
      margin-bottom: 4px;
    }

    .settings-navigation-list {
      display: flex;
      flex-direction: row;
      overflow-x: auto;
      overflow-y: hidden;
      gap: 8px;
      padding: 0 12px;
      max-height: none;
    }

    .settings-navigation-item {
      flex: 0 0 auto;
      min-width: 120px;
      padding: 8px 12px;
      border-radius: var(--border-radius);
      background: var(--bg-400);
      border: 1px solid var(--border-300);
      transition: all 0.2s ease;
    }

    .settings-navigation-item:hover {
      background: var(--bg-500);
      border-color: var(--border-400);
    }

    .settings-navigation-item.active {
      background: var(--color-primary-alpha);
      border-color: var(--color-primary);
    }

    .settings-navigation-content {
      text-align: center;
    }

    .settings-navigation-label {
      font-size: 12px;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .settings-navigation-icon {
      display: none;
    }
  }

  @media (max-width: 320px) {
    .settings-navigation-item {
      min-width: 80px;
      padding: 4px 6px;
    }

    .settings-navigation-label {
      font-size: 10px;
    }
  }
</style>
