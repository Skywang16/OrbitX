<template>
  <div class="empty-state">
    <h1 class="title">OrbitX</h1>
    <div class="shortcuts">
      <div class="shortcut" @click="handleNewTabClick">
        <span class="shortcut-desc">{{ t('shortcuts.actions.new_tab') }}</span>
        <div class="keys">
          <kbd>⌘</kbd>
          <kbd>T</kbd>
        </div>
      </div>
      <div class="shortcut" @click="handleToggleAISidebarClick">
        <span class="shortcut-desc">{{ t('shortcuts.actions.toggle_ai_sidebar') }}</span>
        <div class="keys">
          <kbd>⌘</kbd>
          <kbd>I</kbd>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { useI18n } from 'vue-i18n'
  import { shortcutActionsService } from '@/shortcuts/actions'

  const { t } = useI18n()

  const handleNewTabClick = async () => {
    await shortcutActionsService.newTab()
  }

  const handleToggleAISidebarClick = () => {
    shortcutActionsService.toggleAISidebar()
  }
</script>

<style scoped>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    background: var(--bg-200);
  }

  .title {
    font-size: 48px;
    font-weight: 300;
    color: var(--text-200);
    margin: 0 0 48px 0;
    letter-spacing: -0.02em;
  }

  .shortcuts {
    display: flex;
    flex-direction: column;
    gap: 16px;
    align-items: center;
  }

  .shortcut {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 12px 20px;
    background: var(--bg-300);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-lg);
    transition: all 0.2s ease;
    min-width: 200px;
    cursor: pointer;
    user-select: none;
  }

  .shortcut:hover {
    background: var(--bg-400);
    border-color: var(--border-300);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }

  .shortcut:active {
    transform: translateY(0);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  }

  .keys {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    height: 24px;
    padding: 0 8px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-200);
    background: var(--bg-500);
    border: 1px solid var(--border-400);
    border-radius: var(--border-radius);
    box-shadow:
      0 2px 4px rgba(0, 0, 0, 0.1),
      0 0 0 1px rgba(255, 255, 255, 0.05) inset;
  }

  .shortcut-desc {
    font-size: 13px;
    color: var(--text-300);
    font-weight: 500;
    text-align: center;
  }
</style>
