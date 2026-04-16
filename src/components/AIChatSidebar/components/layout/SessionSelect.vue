<script setup lang="ts">
  import type { ThreadRecord } from '@/api/workspace'
  import type { SelectOption } from '@/ui'
  import { XSelect } from '@/ui'
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'

  // Props definition
  interface Props {
    threads: ThreadRecord[]
    currentThreadId: number | null
    selectedLabel?: string | null
    loading?: boolean
  }

  // Emits definition
  interface Emits {
    (e: 'select-thread', threadId: number): void
    (e: 'create-new-thread'): void
    (e: 'refresh-threads'): void
  }

  const props = withDefaults(defineProps<Props>(), {
    loading: false,
  })

  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const selectOptions = computed<SelectOption[]>(() => {
    return props.threads.map(thread => ({
      label: thread.title || t('chat.new_session'),
      value: thread.id,
      description: `${thread.messageCount} ${t('session.messages')} · ${formatSessionTime(thread.updatedAt)}`,
    }))
  })

  const displayValue = computed(() => {
    return props.selectedLabel || t('chat.new_session')
  })

  import { formatSessionTime } from '@/utils/dateFormatter'

  const handleSelectChange = (value: string | number | null | Array<string | number>) => {
    if (value !== null && !Array.isArray(value)) {
      emit('select-thread', Number(value))
    }
  }

  const handleVisibleChange = (visible: boolean) => {
    if (visible) {
      emit('refresh-threads')
    }
  }
</script>

<template>
  <div class="session-select">
    <XSelect
      :model-value="props.currentThreadId"
      :options="selectOptions"
      :placeholder="displayValue"
      size="small"
      borderless
      filterable
      :filter-placeholder="t('session.search_placeholder')"
      :no-data-text="t('session.no_data')"
      max-height="300px"
      @update:modelValue="handleSelectChange"
      @visible-change="handleVisibleChange"
    />
  </div>
</template>

<style scoped>
  .session-select {
    flex: 1;
    min-width: 0;
    max-width: 100%;
  }

  .session-select :deep(.x-select) {
    width: 100%;
  }

  .session-select :deep(.x-select__input) {
    padding: 0.3em 1.6em 0.3em 0.6em;
    min-height: 1.6em;
    font-size: clamp(10px, 3.5vw, 14px);
    border-radius: var(--border-radius-sm);
    transition: all 0.2s ease;
    color: var(--text-200);
  }

  .session-select :deep(.x-select__input:hover) {
    background-color: var(--color-hover);
  }

  .session-select :deep(.x-select--open .x-select__input) {
    background-color: var(--color-hover);
  }

  .session-select :deep(.x-select__value) {
    font-weight: 500;
    color: var(--text-200);
  }

  .session-select :deep(.x-select__placeholder) {
    color: var(--text-400);
    font-weight: 400;
  }

  .session-select :deep(.x-select__arrow) {
    right: 4px;
    width: 18px;
    height: 18px;
  }

  .session-select :deep(.x-select__arrow svg) {
    width: 16px;
    height: 16px;
  }

  .session-select :deep(.x-select__option) {
    padding: 0.5em 0.8em;
    min-height: 2.2em;
    color: var(--text-200);
  }

  .session-select :deep(.x-select__option-label) {
    font-size: clamp(10px, 3.2vw, 13px);
    font-weight: 500;
    color: inherit;
  }

  .session-select :deep(.x-select__option-description) {
    font-size: clamp(8px, 2.8vw, 11px);
    margin-top: 0.1em;
    opacity: 0.7;
    color: var(--text-400);
  }
</style>
