<script setup lang="ts">
  import { checkpointApi } from '@/api/checkpoint'
  import { useCheckpoint } from '@/composables/useCheckpoint'
  import { useRollbackDialogStore } from '@/stores/rollbackDialog'
  import { useWorkspaceStore } from '@/stores/workspace'
  import type { FileChangeType } from '@/types/domain/checkpoint'
  import { XButton, XModal } from '@/ui'
  import { ref } from 'vue'
  import { useI18n } from 'vue-i18n'

  const emit = defineEmits<{
    rollback: [result: { success: boolean; message: string; restoreContent?: string }]
  }>()

  const { t } = useI18n()
  const store = useRollbackDialogStore()
  const workspaceStore = useWorkspaceStore()
  const { refreshCheckpoints } = useCheckpoint()
  const isConfirming = ref(false)
  const formatErrorMessage = (error: unknown): string => {
    return error instanceof Error ? error.message : String(error)
  }

  const getChangeIcon = (type: FileChangeType) => {
    switch (type) {
      case 'modified':
        return 'M'
      case 'added':
        return 'A'
      case 'deleted':
        return 'D'
    }
  }

  const getChangeClass = (type: FileChangeType) => {
    switch (type) {
      case 'modified':
        return 'change-modified'
      case 'added':
        return 'change-added'
      case 'deleted':
        return 'change-deleted'
    }
  }

  const handleConfirm = async () => {
    if (isConfirming.value || !store.state) return

    isConfirming.value = true
    const { checkpoint, workspacePath } = store.state

    try {
      const result = await checkpointApi.rollback(checkpoint.id)
      if (!result) {
        emit('rollback', {
          success: false,
          message: t('checkpoint.rollback_failed'),
        })
        return
      }

      if (result.failedFiles.length > 0) {
        emit('rollback', {
          success: false,
          message: t('checkpoint.rollback_partial', {
            restored: result.restoredFiles.length,
            failed: result.failedFiles.length,
          }),
        })
        return
      }

      await refreshCheckpoints(checkpoint.threadId, workspacePath)
      await workspaceStore.fetchMessages(checkpoint.threadId)
      await workspaceStore.fetchSubagents(checkpoint.threadId)
      await workspaceStore.loadThreadViews(workspacePath)

      emit('rollback', {
        success: true,
        message: t('checkpoint.rollback_success', { count: result.restoredFiles.length }),
        restoreContent: store.state?.messageContent || '',
      })
    } catch (error) {
      console.error('[RollbackConfirmDialog] Rollback error:', error)
      emit('rollback', {
        success: false,
        message: formatErrorMessage(error),
      })
    } finally {
      isConfirming.value = false
      store.close()
    }
  }

  const handleClose = () => {
    store.close()
  }
</script>

<template>
  <XModal
    :visible="store.visible"
    size="small"
    :show-header="false"
    :show-footer="false"
    no-padding
    modal-class="rollback-dialog"
    @close="handleClose"
  >
    <!-- Header -->
    <div class="dialog-header">
      <div class="dialog-icon">
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <polyline points="1 4 1 10 7 10" />
          <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
        </svg>
      </div>
      <div class="dialog-header-text">
        <h2 class="dialog-title">{{ t('checkpoint.confirm_revert') }}</h2>
        <p class="dialog-subtitle">{{ t('checkpoint.revert_changes_desc') }}</p>
      </div>
    </div>

    <!-- Body -->
    <div class="dialog-body">
      <div v-if="store.loading" class="dialog-loading">
        <svg class="spinner" viewBox="0 0 16 16">
          <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="2" fill="none" stroke-dasharray="30 10" />
        </svg>
        <span>{{ t('checkpoint.loading_files') }}</span>
      </div>

      <div v-else-if="store.files.length > 0" class="file-list">
        <div v-for="file in store.files" :key="file.filePath" class="file-item">
          <span class="change-badge" :class="getChangeClass(file.changeType)">
            {{ getChangeIcon(file.changeType) }}
          </span>
          <span class="file-path">{{ file.filePath }}</span>
        </div>
      </div>

      <div v-else class="dialog-empty">
        {{ t('checkpoint.no_files_changed') }}
      </div>
    </div>

    <!-- Footer -->
    <div class="dialog-footer">
      <div class="dialog-footer-left"></div>
      <div class="dialog-footer-right">
        <XButton variant="secondary" :disabled="isConfirming" @click="handleClose">
          {{ t('dialog.cancel') }}
        </XButton>
        <XButton variant="primary" :disabled="store.loading" :loading="isConfirming" @click="handleConfirm">
          {{ t('dialog.confirm') }}
        </XButton>
      </div>
    </div>
  </XModal>
</template>

<style scoped>
  .dialog-header {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 24px 24px 16px;
    flex-shrink: 0;
  }

  .dialog-header-text {
    flex: 1;
    min-width: 0;
  }

  .dialog-icon {
    width: 48px;
    height: 48px;
    border-radius: var(--border-radius-xl);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
    color: var(--color-warning);
  }

  .dialog-icon svg {
    width: 24px;
    height: 24px;
  }

  .dialog-title {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-100);
    margin: 0 0 2px;
  }

  .dialog-subtitle {
    font-size: 13px;
    color: var(--text-300);
    line-height: 1.4;
    margin: 0;
  }

  /* Body */
  .dialog-body {
    flex: 1;
    overflow-y: auto;
    padding: 0 24px 16px;
    min-height: 0;
  }

  .dialog-body::-webkit-scrollbar {
    width: 6px;
  }
  .dialog-body::-webkit-scrollbar-track {
    background: transparent;
  }
  .dialog-body::-webkit-scrollbar-thumb {
    background: var(--border-300);
    border-radius: var(--border-radius-xs);
  }

  .dialog-loading {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 24px;
    justify-content: center;
    color: var(--text-400);
    font-size: 13px;
  }

  .dialog-empty {
    padding: 24px;
    text-align: center;
    color: var(--text-400);
    font-size: 13px;
  }

  .file-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 240px;
    overflow-y: auto;
    background: var(--bg-200);
    border-radius: var(--border-radius-xl);
    padding: 12px;
  }

  .file-list::-webkit-scrollbar {
    width: 6px;
  }
  .file-list::-webkit-scrollbar-track {
    background: transparent;
  }
  .file-list::-webkit-scrollbar-thumb {
    background: var(--border-300);
    border-radius: var(--border-radius-xs);
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    border-radius: var(--border-radius-md);
    font-size: 13px;
    transition: background 0.1s ease;
  }

  .file-item:hover {
    background: var(--bg-300);
  }

  .change-badge {
    flex-shrink: 0;
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--border-radius-sm);
    font-size: 11px;
    font-weight: 700;
    line-height: 1;
  }

  .change-modified {
    background: color-mix(in srgb, var(--color-warning) 15%, transparent);
    color: var(--color-warning);
  }

  .change-added {
    background: color-mix(in srgb, var(--color-success) 15%, transparent);
    color: var(--color-success);
  }

  .change-deleted {
    background: color-mix(in srgb, var(--color-error) 15%, transparent);
    color: var(--color-error);
  }

  .file-path {
    flex: 1;
    color: var(--text-200);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-family-mono);
    font-size: 12px;
  }

  /* Footer */
  .dialog-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 24px;
    flex-shrink: 0;
  }

  .dialog-footer-left,
  .dialog-footer-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }
</style>

<!-- XModal Teleports to body — scoped cannot reach it -->
<style>
  .rollback-dialog.modal-container {
    max-width: 480px;
  }

  .rollback-dialog > .modal-body {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
</style>
