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
    activeSection: 'theme',
  })

  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const searchQuery = ref('')

  // 设置导航项目列表 - 使用计算属性确保语言切换时能实时更新
  const navigationItems = computed(() => [
    {
      id: 'theme',
      label: t('settings.theme.title'),
      icon: 'palette',
      description: t('settings.theme.description'),
    },
    {
      id: 'ai',
      label: t('settings.ai.title'),
      icon: 'brain',
      description: t('settings.ai.description'),
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

  // 处理导航项点击
  const handleItemClick = (sectionId: string) => {
    if (sectionId !== props.activeSection) {
      emit('change', sectionId)
    }
  }

  // 过滤后的导航项目
  const filteredNavigationItems = computed(() => {
    if (!searchQuery.value) {
      return navigationItems.value
    }

    const query = searchQuery.value.toLowerCase()
    return navigationItems.value.filter(
      item => item.label.toLowerCase().includes(query) || item.description.toLowerCase().includes(query)
    )
  })

  // 获取图标SVG
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
    <div class="navigation-header">
      <x-search-input v-model="searchQuery" :placeholder="t('settings.search_placeholder')" />
    </div>

    <ul class="navigation-list">
      <li
        v-for="item in filteredNavigationItems"
        :key="item.id"
        class="navigation-item"
        :class="{ active: item.id === activeSection }"
        @click="handleItemClick(item.id)"
      >
        <div class="item-icon" v-html="getIconSvg(item.icon)"></div>
        <div class="item-content">
          <div class="item-label">{{ item.label }}</div>
        </div>
      </li>
    </ul>
  </nav>
</template>

<style scoped>
  .settings-navigation {
    padding: var(--spacing-md) 0;
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .navigation-header {
    padding: var(--spacing-md) var(--spacing-lg);
    border-bottom: 1px solid var(--border-300);
    margin-bottom: var(--spacing-sm);
    flex-shrink: 0;
  }

  .navigation-list {
    list-style: none;
    margin: 0;
    padding: 0;
    flex: 1;
    overflow-y: auto;
  }

  .navigation-item {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    padding: var(--spacing-sm) var(--spacing-lg);
    cursor: pointer;
    transition: all 0.2s ease;
    border-left: 2px solid transparent;
    min-height: 40px;
  }

  .navigation-item:hover {
    background-color: var(--bg-400);
  }

  .navigation-item.active {
    background-color: var(--color-primary-alpha);
    border-left: 2px solid var(--color-primary);
  }

  .navigation-item.active .item-label {
    color: var(--color-primary);
    font-weight: 600;
  }

  .item-icon {
    flex-shrink: 0;
    color: var(--text-400);
    transition: color 0.2s ease;
    width: 18px;
    height: 18px;
  }

  .navigation-item.active .item-icon {
    color: var(--color-primary);
  }

  .item-content {
    flex: 1;
    min-width: 0;
  }

  .item-label {
    font-size: var(--font-size-md);
    font-weight: 500;
    color: var(--text-200);
  }
</style>
