<script setup lang="ts">
  import { ref, onMounted, computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { XButton, XModal, createMessage } from '@/ui'
  import { workspaceIndexApi, type WorkspaceIndex } from '@/api/workspace-index'
  import SettingsCard from '../../SettingsCard.vue'

  const { t } = useI18n()

  // 状态管理
  const workspaces = ref<WorkspaceIndex[]>([])
  const loading = ref(false)
  const deleteDialogVisible = ref(false)
  const workspaceToDelete = ref<WorkspaceIndex | null>(null)

  // 加载所有工作区索引
  const loadWorkspaces = async () => {
    try {
      loading.value = true
      const result = await workspaceIndexApi.getAllWorkspaces()
      workspaces.value = Array.isArray(result) ? result : []
    } catch (error) {
      console.error('Failed to load workspaces:', error)
      workspaces.value = [] // 确保设置为空数组
      createMessage.error(t('settings.vectorIndex.load_error'))
    } finally {
      loading.value = false
    }
  }

  // 刷新工作区索引
  const refreshWorkspace = async (workspace: WorkspaceIndex) => {
    try {
      const updatedWorkspace = await workspaceIndexApi.refreshWorkspace(workspace.id)
      const index = workspaces.value.findIndex(w => w.id === workspace.id)
      if (index !== -1) {
        workspaces.value[index] = updatedWorkspace
      }
      createMessage.success(t('settings.vectorIndex.refresh_success'))
    } catch (error) {
      console.error('Failed to refresh workspace:', error)
      createMessage.error(t('settings.vectorIndex.refresh_error'))
    }
  }

  // 显示删除确认对话框
  const showDeleteDialog = (workspace: WorkspaceIndex) => {
    workspaceToDelete.value = workspace
    deleteDialogVisible.value = true
  }

  // 确认删除工作区索引
  const confirmDelete = async () => {
    if (!workspaceToDelete.value) return

    try {
      await workspaceIndexApi.deleteWorkspace(workspaceToDelete.value.id)
      workspaces.value = workspaces.value.filter(w => w.id !== workspaceToDelete.value!.id)
      createMessage.success(t('settings.vectorIndex.delete_success'))
    } catch (error) {
      console.error('Failed to delete workspace:', error)
      createMessage.error(t('settings.vectorIndex.delete_error'))
    } finally {
      deleteDialogVisible.value = false
      workspaceToDelete.value = null
    }
  }

  // 取消删除
  const cancelDelete = () => {
    deleteDialogVisible.value = false
    workspaceToDelete.value = null
  }

  // 格式化文件大小
  const formatSize = (bytes: number): string => {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
  }

  // 格式化时间
  const formatTime = (dateString: string): string => {
    const date = new Date(dateString)
    return date.toLocaleString()
  }

  // 获取目录名称
  const getDirectoryName = (path: string): string => {
    return path.split('/').pop() || path
  }

  // 获取状态显示文本
  const getStatusText = (status: string): string => {
    switch (status) {
      case 'building':
        return t('settings.vectorIndex.status.building')
      case 'ready':
        return t('settings.vectorIndex.status.ready')
      case 'error':
        return t('settings.vectorIndex.status.error')
      default:
        return status
    }
  }

  // 获取状态样式类
  const getStatusClass = (status: string): string => {
    switch (status) {
      case 'building':
        return 'status-building'
      case 'ready':
        return 'status-ready'
      case 'error':
        return 'status-error'
      default:
        return ''
    }
  }

  // 计算统计信息
  const totalWorkspaces = computed(() => workspaces.value.length)
  const readyWorkspaces = computed(() => workspaces.value.filter(w => w.status === 'ready').length)
  const totalFiles = computed(() => workspaces.value.reduce((sum, w) => sum + w.fileCount, 0))
  const totalSize = computed(() => workspaces.value.reduce((sum, w) => sum + w.indexSizeBytes, 0))

  // 初始化方法，供外部调用
  const init = async () => {
    await loadWorkspaces()
  }

  // 暴露初始化方法给父组件
  defineExpose({
    init,
  })

  onMounted(async () => {
    await init()
  })
</script>

<template>
  <div class="settings-group">
    <h2 class="settings-section-title">{{ t('settings.vectorIndex.title') }}</h2>

    <!-- 统计信息 -->
    <div class="settings-group">
      <h3 class="settings-group-title">{{ t('settings.vectorIndex.statistics_title') }}</h3>
      <SettingsCard>
        <div class="statistics-grid">
          <div class="statistic-item">
            <div class="statistic-value">{{ totalWorkspaces }}</div>
            <div class="statistic-label">{{ t('settings.vectorIndex.total_workspaces') }}</div>
          </div>
          <div class="statistic-item">
            <div class="statistic-value">{{ readyWorkspaces }}</div>
            <div class="statistic-label">{{ t('settings.vectorIndex.ready_workspaces') }}</div>
          </div>
          <div class="statistic-item">
            <div class="statistic-value">{{ totalFiles }}</div>
            <div class="statistic-label">{{ t('settings.vectorIndex.total_files') }}</div>
          </div>
          <div class="statistic-item">
            <div class="statistic-value">{{ formatSize(totalSize) }}</div>
            <div class="statistic-label">{{ t('settings.vectorIndex.total_size') }}</div>
          </div>
        </div>
      </SettingsCard>
    </div>

    <!-- 工作区列表 -->
    <div class="settings-group">
      <div class="settings-group-header">
        <h3 class="settings-group-title">{{ t('settings.vectorIndex.workspace_list_title') }}</h3>
        <XButton size="small" @click="loadWorkspaces" :loading="loading">
          {{ t('settings.vectorIndex.refresh') }}
        </XButton>
      </div>

      <SettingsCard>
        <div v-if="loading" class="loading-state">
          <div class="loading-spinner"></div>
          <div class="loading-text">{{ t('settings.vectorIndex.loading') }}</div>
        </div>

        <div v-else-if="!Array.isArray(workspaces) || workspaces.length === 0" class="empty-state">
          <div class="empty-icon">📁</div>
          <div class="empty-text">{{ t('settings.vectorIndex.no_workspaces') }}</div>
          <div class="empty-description">{{ t('settings.vectorIndex.no_workspaces_description') }}</div>
        </div>

        <div v-else class="workspace-list">
          <div v-for="workspace in workspaces" :key="workspace.id" class="workspace-item">
            <div class="workspace-info">
              <div class="workspace-header">
                <h4 class="workspace-name">
                  {{ workspace.name || getDirectoryName(workspace.workspacePath) }}
                </h4>
                <div class="workspace-status" :class="getStatusClass(workspace.status)">
                  {{ getStatusText(workspace.status) }}
                </div>
              </div>
              <p class="workspace-path">{{ workspace.workspacePath }}</p>
              <div class="workspace-stats">
                <span class="stat-item">
                  <span class="stat-label">{{ t('settings.vectorIndex.files') }}:</span>
                  <span class="stat-value">{{ workspace.fileCount }}</span>
                </span>
                <span class="stat-item">
                  <span class="stat-label">{{ t('settings.vectorIndex.size') }}:</span>
                  <span class="stat-value">{{ formatSize(workspace.indexSizeBytes) }}</span>
                </span>
                <span class="stat-item">
                  <span class="stat-label">{{ t('settings.vectorIndex.updated') }}:</span>
                  <span class="stat-value">{{ formatTime(workspace.updatedAt) }}</span>
                </span>
              </div>
              <div v-if="workspace.errorMessage" class="workspace-error">
                {{ workspace.errorMessage }}
              </div>
            </div>
            <div class="workspace-actions">
              <XButton size="small" @click="refreshWorkspace(workspace)" :disabled="workspace.status === 'building'">
                {{ t('settings.vectorIndex.refresh') }}
              </XButton>
              <XButton
                size="small"
                variant="danger"
                @click="showDeleteDialog(workspace)"
                :disabled="workspace.status === 'building'"
              >
                {{ t('settings.vectorIndex.delete') }}
              </XButton>
            </div>
          </div>
        </div>
      </SettingsCard>
    </div>

    <!-- 删除确认对话框 -->
    <XModal
      v-model:visible="deleteDialogVisible"
      :title="t('settings.vectorIndex.delete_confirm_title')"
      show-footer
      @confirm="confirmDelete"
      @cancel="cancelDelete"
    >
      <p>
        {{
          t('settings.vectorIndex.delete_confirm_message', {
            name: workspaceToDelete?.name || getDirectoryName(workspaceToDelete?.workspacePath || ''),
          })
        }}
      </p>
      <p class="delete-warning">{{ t('settings.vectorIndex.delete_warning') }}</p>
    </XModal>
  </div>
</template>

<style scoped>
  .settings-group-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }

  .statistics-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 24px;
    padding: 16px 0;
  }

  .statistic-item {
    text-align: center;
  }

  .statistic-value {
    font-size: 24px;
    font-weight: 600;
    color: var(--color-primary);
    margin-bottom: 4px;
  }

  .statistic-label {
    font-size: 12px;
    color: var(--text-300);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 40px 20px;
    gap: 12px;
  }

  .loading-spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border-200);
    border-top: 2px solid var(--color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  .loading-text {
    color: var(--text-300);
    font-size: 14px;
  }

  @keyframes spin {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }

  .empty-state {
    text-align: center;
    padding: 40px 20px;
  }

  .empty-icon {
    font-size: 48px;
    margin-bottom: 16px;
    opacity: 0.5;
  }

  .empty-text {
    font-size: 16px;
    font-weight: 500;
    color: var(--text-200);
    margin-bottom: 8px;
  }

  .empty-description {
    font-size: 14px;
    color: var(--text-300);
  }

  .workspace-list {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .workspace-item {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 16px;
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-md);
    background: var(--bg-100);
    transition: border-color 0.2s ease;
  }

  .workspace-item:hover {
    border-color: var(--border-300);
  }

  .workspace-info {
    flex: 1;
    min-width: 0;
  }

  .workspace-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 8px;
  }

  .workspace-name {
    font-size: 16px;
    font-weight: 500;
    color: var(--text-100);
    margin: 0;
  }

  .workspace-status {
    padding: 2px 8px;
    border-radius: var(--border-radius-sm);
    font-size: 12px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .status-building {
    background: var(--color-warning-alpha);
    color: var(--color-warning);
  }

  .status-ready {
    background: var(--color-success-alpha);
    color: var(--color-success);
  }

  .status-error {
    background: var(--color-danger-alpha);
    color: var(--color-danger);
  }

  .workspace-path {
    font-size: 14px;
    color: var(--text-300);
    margin: 0 0 12px 0;
    word-break: break-all;
  }

  .workspace-stats {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    margin-bottom: 8px;
  }

  .stat-item {
    display: flex;
    gap: 4px;
    font-size: 13px;
  }

  .stat-label {
    color: var(--text-300);
  }

  .stat-value {
    color: var(--text-200);
    font-weight: 500;
  }

  .workspace-error {
    font-size: 13px;
    color: var(--color-danger);
    background: var(--color-danger-alpha);
    padding: 8px 12px;
    border-radius: var(--border-radius-sm);
    margin-top: 8px;
  }

  .workspace-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }

  .delete-warning {
    color: var(--color-danger);
    font-size: 14px;
    margin-top: 12px;
    font-weight: 500;
  }

  /* 响应式设计 */
  @media (max-width: 768px) {
    .statistics-grid {
      grid-template-columns: repeat(2, 1fr);
      gap: 16px;
    }

    .workspace-item {
      flex-direction: column;
      gap: 16px;
    }

    .workspace-actions {
      align-self: stretch;
      justify-content: flex-end;
    }

    .workspace-stats {
      flex-direction: column;
      gap: 8px;
    }
  }

  @media (max-width: 480px) {
    .statistics-grid {
      grid-template-columns: 1fr;
    }

    .settings-group-header {
      flex-direction: column;
      align-items: flex-start;
      gap: 12px;
    }
  }
</style>
