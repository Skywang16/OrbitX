<script setup lang="ts">
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'

  interface Props {
    selectedText?: string
    displayText?: string
    visible?: boolean
  }

  interface Emits {
    (e: 'clear'): void
    (e: 'insert'): void
  }

  const props = withDefaults(defineProps<Props>(), {
    selectedText: '',
    displayText: '',
    visible: false,
  })

  const emit = defineEmits<Emits>()
  const { t } = useI18n()
</script>

<template>
  <div v-if="visible && selectedText" class="terminal-selection-tag">
    <div class="tag-content">
      <div class="tag-icon">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
          <line x1="8" y1="21" x2="16" y2="21" />
          <line x1="12" y1="17" x2="12" y2="21" />
        </svg>
      </div>
      <span class="tag-text" @click="emit('insert')" :title="`${t('session.click_to_insert')}: ${props.displayText}`">
        {{ props.displayText }}
      </span>
      <button class="tag-close" @click="emit('clear')" :title="t('session.clear_selection')">
        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="18" y1="6" x2="6" y2="18" />
          <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
  .terminal-selection-tag {
    margin-bottom: 8px;
  }

  .tag-content {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background-color: var(--bg-500);
    border: 1px solid var(--border-400);
    border-radius: var(--border-radius-sm);
    padding: 4px 8px;
    font-size: 12px;
    color: var(--text-300);
    max-width: 100%;
  }

  .tag-icon,
  .tag-close {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .tag-icon {
    color: var(--color-primary);
  }

  .tag-text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    cursor: pointer;
    transition: color 0.1s ease;
  }

  .tag-text:hover {
    color: var(--color-primary);
  }

  .tag-close {
    justify-content: center;
    background: none;
    border: none;
    color: var(--text-400);
    cursor: pointer;
    padding: 2px;
    border-radius: 50%;
    transition: all 0.1s ease;
  }

  .tag-close:hover {
    background-color: var(--bg-600);
    color: var(--text-200);
  }

  .tag-close:active {
    transform: scale(0.95);
  }
</style>
