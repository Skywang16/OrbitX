<script setup lang="ts">
  import { ref, onMounted, onBeforeUnmount } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useRollbackDialogStore } from '@/stores/rollbackDialog'
  import { checkpointApi } from '@/api/checkpoint'
  import type { FileChangeType } from '@/types/domain/checkpoint'

  const emit = defineEmits<{
    rollback: [result: { success: boolean; message: string; messageId: number }]
  }>()

  const { t } = useI18n()
  const store = useRollbackDialogStore()
  const isConfirming = ref(false)

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
    const { checkpoint, messageId, workspacePath } = store.state

    try {
      if (checkpoint && checkpoint.fileCount > 0) {
        const result = await checkpointApi.rollback(checkpoint.id, workspacePath)
        if (!result) {
          emit('rollback', {
            success: false,
            message: t('checkpoint.rollback_failed'),
            messageId,
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
            messageId,
          })
          return
        }
      }

      emit('rollback', {
        success: true,
        message: t('checkpoint.rollback_success', { count: checkpoint?.fileCount ?? 0 }),
        messageId,
      })
    } catch (error) {
      console.error('[RollbackConfirmDialog] Rollback error:', error)
      emit('rollback', {
        success: false,
        message: String(error),
        messageId,
      })
    } finally {
      isConfirming.value = false
      store.close()
    }
  }

  const handleClose = () => {
    store.close()
  }

  const handleBackdropClick = (event: MouseEvent) => {
    if (event.target === event.currentTarget) {
      handleClose()
    }
  }

  const handleKeydown = (event: KeyboardEvent) => {
    if (event.key === 'Escape') {
      handleClose()
    }
  }

  onMounted(() => {
    document.addEventListener('keydown', handleKeydown)
  })

  onBeforeUnmount(() => {
    document.removeEventListener('keydown', handleKeydown)
  })
</script>

<template>
  <div v-if="store.visible" class="rollback-overlay" @click="handleBackdropClick">
    <div class="rollback-dialog">
      <div class="dialog-header">
        <span class="dialog-title">{{ t('checkpoint.confirm_revert') }}</span>
        <button class="close-btn" @click="handleClose">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        </button>
      </div>

      <div class="dialog-body">
        <p class="dialog-desc">{{ t('checkpoint.revert_changes_desc') }}</p>

        <div v-if="store.loading" class="loading-state">
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

        <div v-else class="empty-state">
          {{ t('checkpoint.no_files_changed') }}
        </div>
      </div>

      <div class="dialog-footer">
        <button class="btn btn-secondary" @click="handleClose" :disabled="isConfirming">
          {{ t('dialog.cancel') }} (esc)
        </button>
        <button class="btn btn-primary" @click="handleConfirm" :disabled="isConfirming || store.loading">
          <svg v-if="isConfirming" class="spinner" viewBox="0 0 16 16">
            <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="2" fill="none" stroke-dasharray="30 10" />
          </svg>
          {{ t('dialog.confirm') }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .rollback-overlay {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    animation: fadeIn 0.2s ease;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  .rollback-dialog {
    background: var(--bg-100);
    border: 1px solid var(--border-300);
    border-radius: 8px;
    width: calc(100% - 32px);
    max-width: 400px;
    max-height: calc(100% - 64px);
    display: flex;
    flex-direction: column;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
  }

  .dialog-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-200);
  }

  .dialog-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-100);
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-400);
    cursor: pointer;
    transition: all 0.15s;
  }

  .close-btn:hover {
    background: var(--bg-300);
    color: var(--text-100);
  }

  .dialog-body {
    flex: 1;
    padding: 16px;
    overflow-y: auto;
  }

  .dialog-desc {
    margin: 0 0 12px;
    font-size: 13px;
    color: var(--text-200);
  }

  .loading-state {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 16px;
    color: var(--text-300);
    font-size: 13px;
  }

  .file-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 200px;
    overflow-y: auto;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    background: var(--bg-200);
    border-radius: 4px;
    font-size: 12px;
  }

  .change-badge {
    flex-shrink: 0;
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    font-size: 11px;
    font-weight: 600;
  }

  .change-modified {
    background: rgba(234, 179, 8, 0.2);
    color: #eab308;
  }

  .change-added {
    background: rgba(34, 197, 94, 0.2);
    color: #22c55e;
  }

  .change-deleted {
    background: rgba(239, 68, 68, 0.2);
    color: #ef4444;
  }

  .file-path {
    flex: 1;
    color: var(--text-200);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
  }

  .empty-state {
    padding: 16px;
    text-align: center;
    color: var(--text-400);
    font-size: 13px;
  }

  .dialog-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--border-200);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: 13px;
    font-weight: 500;
    border-radius: 6px;
    border: none;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: var(--bg-300);
    color: var(--text-200);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--bg-400);
  }

  .btn-primary {
    background: var(--color-primary);
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .spinner {
    width: 14px;
    height: 14px;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
</style>
