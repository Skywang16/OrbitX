<script setup lang="ts">
  import { computed } from 'vue'
  import { XSelect } from '@/ui'
  import type { SelectOption } from '@/ui'
  import type { Conversation } from '@/types/features/ai/chat'

  // Props定义
  interface Props {
    sessions: Conversation[]
    currentSessionId: number | null
    loading?: boolean
  }

  // Emits定义
  interface Emits {
    (e: 'select-session', sessionId: number): void
    (e: 'create-new-session'): void
    (e: 'delete-session', sessionId: number): void
    (e: 'refresh-sessions'): void
  }

  const props = withDefaults(defineProps<Props>(), {
    loading: false,
  })

  const emit = defineEmits<Emits>()

  const selectOptions = computed<SelectOption[]>(() => {
    return props.sessions.map(session => ({
      label: session.title || '未命名会话',
      value: session.id,
      description: `${session.messageCount} 条消息 · ${formatSessionTime(session.updatedAt)}`,
    }))
  })

  const displayValue = computed(() => {
    if (!props.currentSessionId) return '选择会话'
    const session = props.sessions.find(s => s.id === props.currentSessionId)
    return session?.title || '当前会话'
  })

  import { formatSessionTime } from '@/utils/dateFormatter'

  const handleSelectChange = (value: string | number | null | Array<string | number>) => {
    if (value !== null && !Array.isArray(value)) {
      emit('select-session', Number(value))
    }
  }

  const handleVisibleChange = (visible: boolean) => {
    if (visible) {
      emit('refresh-sessions')
    }
  }
</script>

<template>
  <div class="session-select">
    <XSelect
      :model-value="props.currentSessionId"
      :options="selectOptions"
      :placeholder="displayValue"
      size="small"
      borderless
      filterable
      filter-placeholder="搜索会话..."
      no-data-text="暂无会话历史"
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
    border-radius: 3px;
    transition: all 0.2s ease;
  }

  .session-select :deep(.x-select__input:hover) {
    background-color: var(--color-hover, rgba(0, 0, 0, 0.05));
  }

  .session-select :deep(.x-select--open .x-select__input) {
    background-color: var(--color-hover, rgba(0, 0, 0, 0.05));
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
    width: 10px;
    height: 10px;
  }

  .session-select :deep(.x-select__arrow svg) {
    width: 8px;
    height: 8px;
  }

  /* 下拉选项样式优化 */
  .session-select :deep(.x-select__option) {
    padding: 0.5em 0.8em;
    min-height: 2.2em;
  }

  .session-select :deep(.x-select__option-label) {
    font-size: clamp(10px, 3.2vw, 13px);
    font-weight: 500;
  }

  .session-select :deep(.x-select__option-description) {
    font-size: clamp(8px, 2.8vw, 11px);
    margin-top: 0.1em;
    opacity: 0.7;
  }
</style>
