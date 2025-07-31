<script setup lang="ts">
  import { computed, ref } from 'vue'

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

  const searchQuery = ref('')

  // 设置导航项目列表
  const navigationItems = [
    {
      id: 'theme',
      label: '主题设置',
      icon: 'palette',
      description: '外观和主题配置',
    },
    {
      id: 'config',
      label: '配置管理',
      icon: 'settings',
      description: '配置文件信息和管理',
    },
    {
      id: 'ai',
      label: 'AI 设置',
      icon: 'brain',
      description: 'AI模型和功能配置',
    },
  ]

  // 处理导航项点击
  const handleItemClick = (sectionId: string) => {
    if (sectionId !== props.activeSection) {
      emit('change', sectionId)
    }
  }

  // 过滤后的导航项目
  const filteredNavigationItems = computed(() => {
    if (!searchQuery.value) {
      return navigationItems
    }

    const query = searchQuery.value.toLowerCase()
    return navigationItems.filter(
      item => item.label.toLowerCase().includes(query) || item.description.toLowerCase().includes(query)
    )
  })

  // 获取图标SVG
  const getIconSvg = (iconName: string) => {
    const icons: Record<string, string> = {
      palette: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="13.5" cy="6.5" r=".5"/>
        <circle cx="17.5" cy="10.5" r=".5"/>
        <circle cx="8.5" cy="7.5" r=".5"/>
        <circle cx="6.5" cy="12.5" r=".5"/>
        <path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z"/>
      </svg>`,
      settings: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="3"/>
        <path d="M12 1v6m0 6v6m11-7h-6m-6 0H1m15.5-3.5L19 8.5m-14 7L7.5 13m11 3.5L16.5 19m-9-9L5 7.5"/>
      </svg>`,
      brain: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M9.5 2A2.5 2.5 0 0 1 12 4.5v15a2.5 2.5 0 0 1-4.96.44 2.5 2.5 0 0 1-2.96-3.08 3 3 0 0 1-.34-5.58 2.5 2.5 0 0 1 1.32-4.24 2.5 2.5 0 0 1 1.98-3A2.5 2.5 0 0 1 9.5 2Z"/>
        <path d="M14.5 2A2.5 2.5 0 0 0 12 4.5v15a2.5 2.5 0 0 0 4.96.44 2.5 2.5 0 0 0 2.96-3.08 3 3 0 0 0 .34-5.58 2.5 2.5 0 0 0-1.32-4.24 2.5 2.5 0 0 0-1.98-3A2.5 2.5 0 0 0 14.5 2Z"/>
      </svg>`,
    }
    return icons[iconName] || ''
  }
</script>

<template>
  <nav class="settings-navigation">
    <div class="navigation-header">
      <x-search-input v-model="searchQuery" placeholder="搜索" />
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
          <div class="item-description">{{ item.description }}</div>
        </div>
      </li>
    </ul>
  </nav>
</template>

<style scoped>
  .settings-navigation {
    padding: var(--spacing-md) 0;
  }

  .navigation-header {
    padding: var(--spacing-md);
    border-bottom: 1px solid var(--border-color);
    margin-bottom: var(--spacing-md);
  }

  .navigation-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .navigation-item {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm) var(--spacing-md);
    cursor: pointer;
    transition: all 0.2s ease;
    border-left: 3px solid transparent;
  }

  .navigation-item:hover {
    background-color: var(--color-background-hover);
  }

  .navigation-item.active {
    background-color: var(--color-primary-alpha);
  }

  .navigation-item.active .item-label {
    color: var(--color-primary);
    font-weight: 600;
  }

  .item-icon {
    flex-shrink: 0;
    color: var(--text-secondary);
    transition: color 0.2s ease;
  }

  .navigation-item.active .item-icon {
    color: var(--color-primary);
  }

  .item-content {
    flex: 1;
    min-width: 0;
  }

  .item-label {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 2px;
  }

  .item-description {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    line-height: 1.3;
  }
</style>
