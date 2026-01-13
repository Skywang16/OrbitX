<script setup lang="ts">
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useLayoutStore, type LeftSidebarPanel } from '@/stores/layout'
  import { useGitStore } from '@/stores/git'

  const { t } = useI18n()
  const layoutStore = useLayoutStore()
  const gitStore = useGitStore()

  interface ActivityItem {
    id: LeftSidebarPanel
    icon: string
    title: string
    badge?: number
  }

  const activities = computed<ActivityItem[]>(() => [
    {
      id: 'workspace',
      icon: 'folder',
      title: t('workspace.title'),
    },
    {
      id: 'git',
      icon: 'git',
      title: t('git.title'),
      badge: gitStore.changedCount > 0 ? gitStore.changedCount : undefined,
    },
    {
      id: 'config',
      icon: 'config',
      title: t('config.title'),
    },
  ])

  const isActive = (id: LeftSidebarPanel) => {
    return layoutStore.activeLeftPanel === id
  }

  const handleClick = (id: LeftSidebarPanel) => {
    layoutStore.setActivePanel(id)
  }
</script>

<template>
  <div class="activity-bar">
    <div class="activity-icons">
      <button
        v-for="item in activities"
        :key="item.icon"
        class="activity-btn"
        :class="{ active: isActive(item.id) }"
        :title="item.title"
        @click="handleClick(item.id)"
      >
        <!-- Folder icon -->
        <svg v-if="item.icon === 'folder'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
        </svg>

        <!-- Git icon -->
        <svg v-else-if="item.icon === 'git'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M6 3v12" />
          <path d="M6 7h7a3 3 0 0 1 3 3v11" />
          <circle cx="6" cy="5" r="2" />
          <circle cx="6" cy="15" r="2" />
          <circle cx="16" cy="21" r="2" />
        </svg>

        <!-- Config icon (AI sparkles) -->
        <svg
          v-else-if="item.icon === 'config'"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M9.5 2l1 3.5L14 6.5l-3.5 1L9.5 11l-1-3.5L5 6.5l3.5-1L9.5 2z" />
          <path d="M17 12l.75 2.25L20 15l-2.25.75L17 18l-.75-2.25L14 15l2.25-.75L17 12z" />
          <path d="M6 15l.5 1.5L8 17l-1.5.5L6 19l-.5-1.5L4 17l1.5-.5L6 15z" />
        </svg>

        <span v-if="item.badge" class="activity-badge">{{ item.badge > 99 ? '99+' : item.badge }}</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
  .activity-bar {
    display: flex;
    flex-direction: column;
    width: 40px;
    min-width: 40px;
    background: var(--bg-100);
    border-right: 1px solid var(--border-200);
    padding: 8px 0;
  }

  .activity-icons {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  .activity-btn {
    position: relative;
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-sm);
    cursor: pointer;
    color: var(--text-300);
    transition: all 0.15s ease;
  }

  .activity-btn:hover {
    background: var(--color-hover);
    color: var(--text-200);
  }

  .activity-btn.active {
    background: var(--color-primary-alpha);
    color: var(--color-primary);
  }

  .activity-btn.active::before {
    content: '';
    position: absolute;
    left: 0;
    top: 50%;
    transform: translateY(-50%);
    width: 2px;
    height: 16px;
    background: var(--color-primary);
    border-radius: 0 2px 2px 0;
  }

  .activity-btn svg {
    width: 18px;
    height: 18px;
  }

  .activity-badge {
    position: absolute;
    top: 2px;
    right: 2px;
    min-width: 14px;
    height: 14px;
    padding: 0 4px;
    font-size: 10px;
    font-weight: 500;
    line-height: 14px;
    text-align: center;
    color: white;
    background: var(--color-primary);
    border-radius: 7px;
  }
</style>
