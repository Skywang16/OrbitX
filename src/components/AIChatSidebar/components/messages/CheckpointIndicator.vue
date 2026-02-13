<script setup lang="ts">
  import { useI18n } from 'vue-i18n'
  import type { CheckpointSummary } from '@/types/domain/checkpoint'
  import { useRollbackDialogStore } from '@/stores/rollbackDialog'

  interface Props {
    checkpoint?: CheckpointSummary | null
    workspacePath: string
    messageContent?: string | null
  }

  const props = defineProps<Props>()

  const { t } = useI18n()
  const rollbackStore = useRollbackDialogStore()

  const openConfirmDialog = () => {
    if (!props.checkpoint) return
    rollbackStore.open({
      checkpoint: props.checkpoint,
      workspacePath: props.workspacePath,
      messageContent: props.messageContent ?? '',
    })
  }
</script>

<template>
  <button
    v-if="checkpoint"
    class="rollback-btn"
    type="button"
    :title="t('checkpoint.rollback')"
    @click.stop="openConfirmDialog"
  >
    <svg class="rollback-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M3 10h10a5 5 0 0 1 5 5v2" />
      <path d="M7 6l-4 4 4 4" />
    </svg>
  </button>
</template>

<style scoped>
  .rollback-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-500);
    cursor: pointer;
    opacity: 0.5;
    transition: all 0.15s ease;
    pointer-events: auto;
    position: relative;
    z-index: 1;
  }

  .rollback-btn:hover:not(:disabled) {
    opacity: 1;
    background: var(--bg-400);
    color: var(--color-primary);
  }

  .rollback-icon {
    width: 14px;
    height: 14px;
    pointer-events: none;
  }
</style>
