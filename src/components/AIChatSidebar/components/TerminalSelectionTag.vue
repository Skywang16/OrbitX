<script setup lang="ts">
  interface Props {
    selectedText?: string
    selectionInfo?: string
    visible?: boolean
  }

  const props = withDefaults(defineProps<Props>(), {
    selectedText: '',
    selectionInfo: '',
    visible: false,
  })

  const emit = defineEmits<{
    clear: []
    insert: []
  }>()

  // 简化显示文本逻辑
  const displayText = () => props.selectionInfo || '已选择终端内容'
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
      <span class="tag-text" @click="emit('insert')" title="点击插入到输入框">{{ displayText() }}</span>
      <button class="tag-close" @click="emit('clear')" title="清除选择">
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
    border-radius: 4px;
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
