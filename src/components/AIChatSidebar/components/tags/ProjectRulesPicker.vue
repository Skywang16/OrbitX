<script setup lang="ts">
  import { ref, onMounted, computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { workspaceApi } from '@/api'

  interface Props {
    currentRules: string | null
    cwd?: string
  }

  interface Emits {
    (e: 'select', rulesFile: string): void
    (e: 'close'): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const selectedRules = ref<string>('')
  const loading = ref(false)
  const availableFiles = ref<string[]>([])

  const rulesOptions = computed(() =>
    availableFiles.value.map(filename => ({
      value: filename,
      label: filename,
    }))
  )

  const loadCurrentRules = async () => {
    loading.value = true
    const rules = await workspaceApi.getProjectRules()
    selectedRules.value = rules || 'CLAUDE.md'
    loading.value = false
  }

  const loadAvailableFiles = async () => {
    if (!props.cwd) return

    const files = await workspaceApi.listAvailableRulesFiles(props.cwd)
    availableFiles.value = files
    // 如果当前没有可用文件，至少显示默认的
    if (files.length === 0) {
      availableFiles.value = ['CLAUDE.md']
    }
  }

  const handleRulesClick = async (value: string) => {
    await workspaceApi.setProjectRules(value || null)
    selectedRules.value = value
    emit('select', value)
    emit('close')
  }

  const isCurrentRules = (value: string) => {
    return value === selectedRules.value
  }

  onMounted(() => {
    loadCurrentRules()
    loadAvailableFiles()
  })
</script>

<template>
  <div class="project-rules-picker">
    <div class="body">
      <div v-if="loading" class="loading-state">
        <div class="spinner"></div>
        <p>{{ t('common.loading') }}</p>
      </div>

      <div v-else class="rules-list">
        <div
          v-for="option in rulesOptions"
          :key="option.value"
          class="rules-item"
          :class="{ current: isCurrentRules(option.value) }"
          @click="handleRulesClick(option.value)"
        >
          <svg
            v-if="isCurrentRules(option.value)"
            class="check-icon"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="3"
          >
            <polyline points="20 6 9 17 4 12" />
          </svg>
          <span class="rules-name">{{ option.label }}</span>
        </div>

        <div v-if="availableFiles.length === 0" class="empty-state">
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <path
              d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
            <polyline points="14 2 14 8 20 8" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
          <p>{{ t('ai_feature.no_rules_files') }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .project-rules-picker {
    overflow: hidden;
  }

  .body {
    padding: var(--spacing-lg);
    max-height: 220px;
    overflow-y: auto;
  }

  .loading-state,
  .error-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--spacing-xl) var(--spacing-lg);
    color: var(--text-300);
    text-align: center;
  }

  .loading-state .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border-300);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin-bottom: var(--spacing-sm);
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .empty-state svg {
    opacity: 0.5;
    margin-bottom: var(--spacing-sm);
  }

  .rules-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .rules-item {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--border-radius-sm);
    cursor: pointer;
    transition: all 0.15s ease;
    color: var(--text-200);
    min-height: 32px;
  }

  .rules-item:hover {
    background: var(--bg-300);
    color: var(--text-100);
  }

  .rules-item.current {
    color: var(--color-primary);
    font-weight: 500;
  }

  .rules-item.current:hover {
    background: var(--bg-300);
  }

  .check-icon {
    flex-shrink: 0;
    color: var(--color-primary);
  }

  .rules-name {
    font-size: var(--font-size-sm);
    font-family: var(--font-family-mono);
    word-break: break-all;
  }

  .body::-webkit-scrollbar {
    width: 6px;
  }

  .body::-webkit-scrollbar-track {
    background: transparent;
  }

  .body::-webkit-scrollbar-thumb {
    background: var(--border-300);
    border-radius: 3px;
  }

  .body::-webkit-scrollbar-thumb:hover {
    background: var(--border-400);
  }
</style>
