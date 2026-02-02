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
    activeSection: 'general',
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
</script>

<template>
  <nav class="settings-nav">
    <div class="settings-nav-header">
      <x-search-input
        v-model="searchQuery"
        :placeholder="t('settings.search_placeholder')"
        class="settings-nav-search"
      />
    </div>

    <div class="settings-nav-content">
      <ul class="settings-nav-list">
        <li
          v-for="item in filteredNavigationItems"
          :key="item.id"
          class="settings-nav-item"
          :class="{ active: item.id === activeSection }"
          @click="handleItemClick(item.id)"
        >
          <div class="settings-nav-icon">
            <!-- Settings Icon -->
            <svg v-if="item.icon === 'settings'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="3" />
              <path
                d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
              />
            </svg>
            <!-- Brain/AI Icon -->
            <svg
              v-else-if="item.icon === 'brain'"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <path
                d="M9.5 2A2.5 2.5 0 0 1 12 4.5v15a2.5 2.5 0 0 1-4.96.44 2.5 2.5 0 0 1-2.96-3.08 3 3 0 0 1-.34-5.58 2.5 2.5 0 0 1 1.32-4.24 2.5 2.5 0 0 1 1.98-3A2.5 2.5 0 0 1 9.5 2Z"
              />
              <path
                d="M14.5 2A2.5 2.5 0 0 0 12 4.5v15a2.5 2.5 0 0 0 4.96.44 2.5 2.5 0 0 0 2.96-3.08 3 3 0 0 0 .34-5.58 2.5 2.5 0 0 0-1.32-4.24 2.5 2.5 0 0 0-1.98-3A2.5 2.5 0 0 0 14.5 2Z"
              />
            </svg>
            <!-- Palette Icon -->
            <svg
              v-else-if="item.icon === 'palette'"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <circle cx="13.5" cy="6.5" r="0.5" fill="currentColor" />
              <circle cx="17.5" cy="10.5" r="0.5" fill="currentColor" />
              <circle cx="8.5" cy="7.5" r="0.5" fill="currentColor" />
              <circle cx="6.5" cy="12.5" r="0.5" fill="currentColor" />
              <path
                d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z"
              />
            </svg>
            <!-- Keyboard Icon -->
            <svg
              v-else-if="item.icon === 'keyboard'"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <rect x="2" y="4" width="20" height="16" rx="2" ry="2" />
              <path d="M6 8h.001" />
              <path d="M10 8h.001" />
              <path d="M14 8h.001" />
              <path d="M18 8h.001" />
              <path d="M8 12h.001" />
              <path d="M12 12h.001" />
              <path d="M16 12h.001" />
              <path d="M7 16h10" />
            </svg>
            <!-- Globe Icon -->
            <svg
              v-else-if="item.icon === 'globe'"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <circle cx="12" cy="12" r="10" />
              <line x1="2" y1="12" x2="22" y2="12" />
              <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
            </svg>
          </div>
          <span class="settings-nav-label">{{ item.label }}</span>
          <div class="settings-nav-indicator"></div>
        </li>
      </ul>
    </div>

    <div v-if="filteredNavigationItems.length === 0" class="settings-nav-empty">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="settings-nav-empty-icon">
        <circle cx="11" cy="11" r="8" />
        <path d="m21 21-4.35-4.35" />
      </svg>
      <span>{{ t('settings.no_results') || 'No results found' }}</span>
    </div>
  </nav>
</template>

<style scoped>
  .settings-nav {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-200);
  }

  .settings-nav-header {
    padding: 16px;
    border-bottom: 1px solid var(--border-100);
    flex-shrink: 0;
  }

  .settings-nav-search {
    width: 100%;
  }

  .settings-nav-content {
    flex: 1;
    overflow-y: auto;
    padding: 12px 8px;
    scrollbar-width: none;
    -ms-overflow-style: none;
  }

  .settings-nav-content::-webkit-scrollbar {
    display: none;
  }

  .settings-nav-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .settings-nav-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s ease;
    position: relative;
    color: var(--text-300);
  }

  .settings-nav-item:hover {
    background: var(--bg-300);
    color: var(--text-100);
  }

  .settings-nav-item.active {
    background: color-mix(in srgb, var(--color-primary) 12%, transparent);
    color: var(--color-primary);
  }

  .settings-nav-icon {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    transition: transform 0.15s ease;
  }

  .settings-nav-icon svg {
    width: 18px;
    height: 18px;
  }

  .settings-nav-item:hover .settings-nav-icon {
    transform: scale(1.05);
  }

  .settings-nav-label {
    font-size: 13px;
    font-weight: 500;
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .settings-nav-indicator {
    position: absolute;
    left: 0;
    top: 50%;
    transform: translateY(-50%) scaleY(0);
    width: 3px;
    height: 16px;
    background: var(--color-primary);
    border-radius: 0 2px 2px 0;
    transition: transform 0.2s ease;
  }

  .settings-nav-item.active .settings-nav-indicator {
    transform: translateY(-50%) scaleY(1);
  }

  .settings-nav-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 32px 16px;
    color: var(--text-500);
    text-align: center;
  }

  .settings-nav-empty-icon {
    width: 32px;
    height: 32px;
    opacity: 0.5;
  }

  .settings-nav-empty span {
    font-size: 13px;
  }
</style>
