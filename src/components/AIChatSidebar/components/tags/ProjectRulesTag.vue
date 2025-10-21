<script setup lang="ts">
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'

  interface Props {
    rulesFile?: string | null
    visible?: boolean
  }

  interface Emits {
    (e: 'click'): void
  }

  const props = withDefaults(defineProps<Props>(), {
    rulesFile: null,
    visible: false,
  })

  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const displayRules = computed(() => {
    if (props.rulesFile) {
      return props.rulesFile
    }
    return '...'
  })

  const tooltipText = computed(() => {
    if (props.rulesFile) {
      return `${t('ai_feature.project_rules')}: ${props.rulesFile} · ${t('ai_feature.click_to_switch')}`
    }
    return t('ai_feature.project_rules_description')
  })

  const handleClick = () => {
    emit('click')
  }
</script>

<template>
  <div v-if="visible" class="project-rules-tag">
    <div class="tag-content" @click="handleClick">
      <div class="tag-icon">
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
          <polyline points="14 2 14 8 20 8" />
          <line x1="16" y1="13" x2="8" y2="13" />
          <line x1="16" y1="17" x2="8" y2="17" />
          <polyline points="10 9 9 9 8 9" />
        </svg>
      </div>
      <span class="tag-text" :title="tooltipText">
        {{ displayRules }}
      </span>
    </div>
  </div>
</template>

<style scoped>
  .project-rules-tag {
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
