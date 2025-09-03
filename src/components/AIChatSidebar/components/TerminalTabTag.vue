<script setup lang="ts">
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'

  interface Props {
    terminalId?: string
    shell?: string
    cwd?: string
    displayPath?: string
    visible?: boolean
  }

  const props = withDefaults(defineProps<Props>(), {
    terminalId: '',
    shell: '',
    cwd: '',
    displayPath: '',
    visible: false,
  })


  const { t } = useI18n()

  // 计算显示文本
  const displayText = computed(() => {
    if (props.shell && props.displayPath) {
      return `${props.shell} - ${props.displayPath}`
    }
    if (props.shell) {
      return props.shell
    }
    if (props.displayPath) {
      return props.displayPath
    }
    return t('session.current_terminal')
  })

  // 计算工具提示
  const tooltipText = computed(() => {
    const parts = []
    if (props.shell) {
      parts.push(`Shell: ${props.shell}`)
    }
    if (props.cwd) {
      parts.push(`Path: ${props.cwd}`)
    }
    return parts.join('\n') || t('session.current_terminal')
  })
</script>

<template>
  <div v-if="visible" class="terminal-tab-tag">
    <div class="tag-content">
      <div class="tag-icon">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
          <line x1="8" y1="21" x2="16" y2="21" />
          <line x1="12" y1="17" x2="12" y2="21" />
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

  .tag-icon {
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
    cursor: default;
    transition: color 0.1s ease;
  }

</style>
