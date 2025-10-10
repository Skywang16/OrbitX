<script setup lang="ts">
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'

  interface Props {
    terminalId?: number
    cwd?: string
    displayPath?: string
    visible?: boolean
  }

  interface Emits {
    (e: 'open-navigator'): void
  }

  const props = withDefaults(defineProps<Props>(), {
    terminalId: undefined,
    cwd: '',
    displayPath: '',
    visible: false,
  })

  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  // 计算显示文本 - 只显示工作目录
  const displayText = computed(() => {
    if (props.displayPath) {
      return props.displayPath
    }
    return t('session.current_terminal')
  })

  // 计算工具提示
  const tooltipText = computed(() => {
    if (props.cwd) {
      return `${t('workspace.current_directory')}: ${props.cwd}`
    }
    return t('session.current_terminal')
  })

  const handleTagClick = () => {
    if (props.terminalId && props.cwd) {
      emit('open-navigator')
    }
  }
</script>

<template>
  <div v-if="visible" class="terminal-tab-tag">
    <div class="tag-content" @click="handleTagClick">
      <div class="tag-icon">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
        </svg>
      </div>
      <span class="tag-text" :title="tooltipText">
        {{ displayText }}
      </span>
    </div>
  </div>
</template>

<style scoped>
  .terminal-tab-tag {
    display: inline-block;
    vertical-align: middle;
    margin-right: 8px;
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
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .tag-content:hover {
    background-color: var(--bg-400);
    border-color: var(--border-300);
    color: var(--text-200);
  }

  .tag-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 14px;
    height: 14px;
    color: var(--color-primary);
  }

  .tag-text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    pointer-events: none;
    transition: color 0.1s ease;
  }
</style>
