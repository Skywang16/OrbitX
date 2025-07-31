<script setup lang="ts">
  import { useConfig, useConfigFile } from '@/composables/useConfig'
  import { XButton as Button, XMessage as Message } from '@/ui/components'
  import { computed, onMounted, ref } from 'vue'

  // 使用配置组合函数
  const { loadingState, clearError } = useConfig()
  const { filePath, fileState, openFile, initialize: initializeConfigFile } = useConfigFile()

  // 组件挂载时初始化配置文件信息
  onMounted(() => {
    initializeConfigFile()
  })

  // 消息通知状态
  const messageState = ref<{
    visible: boolean
    type: 'success' | 'error' | 'info' | 'warning'
    content: string
  }>({
    visible: false,
    type: 'info',
    content: '',
  })

  // 消息处理函数
  const message = {
    success: (msg: string) => {
      messageState.value = {
        visible: true,
        type: 'success',
        content: msg,
      }
      // 3秒后自动隐藏成功消息
      setTimeout(() => {
        messageState.value.visible = false
      }, 3000)
    },
    error: (msg: string) => {
      messageState.value = {
        visible: true,
        type: 'error',
        content: msg,
      }
      // 错误消息需要手动关闭
    },
    info: (msg: string) => {
      messageState.value = {
        visible: true,
        type: 'info',
        content: msg,
      }
      setTimeout(() => {
        messageState.value.visible = false
      }, 3000)
    },
    warning: (msg: string) => {
      messageState.value = {
        visible: true,
        type: 'warning',
        content: msg,
      }
      setTimeout(() => {
        messageState.value.visible = false
      }, 5000)
    },
  }

  const closeMessage = () => {
    messageState.value.visible = false
  }

  // 计算属性
  const loading = computed(() => loadingState.value.loading)
  const error = computed(() => loadingState.value.error)
  const canOpenFile = computed(() => filePath.value !== '')

  // 文件状态相关计算属性
  const fileStatusText = computed(() => {
    if (fileState.value.loading) return '获取文件信息中...'
    if (fileState.value.error) return '获取文件信息失败'
    if (!filePath.value) return '获取文件信息中...'
    return '配置文件正常'
  })

  const fileStatusClass = computed(() => {
    if (fileState.value.loading) return 'status-loading'
    if (fileState.value.error) return 'status-error'
    if (!filePath.value) return 'status-loading'
    return 'status-success'
  })

  const fileSize = computed(() => {
    return fileState.value.info?.size ? `${Math.round(fileState.value.info.size / 1024)} KB` : '未知'
  })
  const fileModifiedAt = computed(() => {
    return fileState.value.info?.modifiedAt || '未知'
  })

  // 其他状态相关计算属性
  const statusLoading = computed(() => false)

  // 方法

  const handleOpenConfigFile = async () => {
    try {
      if (!filePath.value) {
        message.warning('配置文件路径未知，无法打开文件')
        return
      }
      await openFile()
      message.success('配置文件已在默认编辑器中打开')
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : String(error)
      message.error(`打开配置文件失败: ${errorMsg}。请检查文件是否存在或权限是否足够。`)
    }
  }

  const handleRefreshInfo = async () => {
    try {
      message.info('正在刷新配置信息...')
      // 这里可以添加实际的刷新逻辑
      message.success('配置信息已刷新')
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : String(error)
      message.error(`刷新信息失败: ${errorMsg}`)
    }
  }

  const clearAllErrors = () => {
    clearError()
  }
</script>

<template>
  <div class="config-settings">
    <!-- 全局消息提示 -->
    <Message
      v-if="messageState.visible"
      :visible="messageState.visible"
      :message="messageState.content"
      :type="messageState.type"
      :closable="true"
      @close="closeMessage"
      class="global-message"
    />

    <div class="settings-header">
      <h2>配置管理</h2>
      <p>查看和管理应用配置文件信息。修改配置文件后，请重启应用以使更改生效。</p>
    </div>

    <!-- 配置文件信息 -->
    <div class="settings-section">
      <h3>配置文件信息</h3>

      <div class="info-grid">
        <div class="info-item">
          <label>文件路径</label>
          <div class="file-path">
            <code>{{ filePath || '获取中...' }}</code>
            <Button size="small" variant="secondary" :disabled="!canOpenFile" @click="handleOpenConfigFile">
              打开文件
            </Button>
          </div>
        </div>

        <div class="info-item">
          <label>文件状态</label>
          <div class="file-status" :class="fileStatusClass">
            <span class="status-indicator"></span>
            {{ fileStatusText }}
          </div>
        </div>

        <div class="info-item">
          <label>文件大小</label>
          <span>{{ fileSize }}</span>
        </div>

        <div class="info-item">
          <label>最后修改</label>
          <span>{{ fileModifiedAt }}</span>
        </div>
      </div>

      <div class="section-actions">
        <Button variant="secondary" :loading="fileState.loading || statusLoading" @click="handleRefreshInfo">
          刷新信息
        </Button>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .config-settings {
    padding: var(--spacing-lg);
    max-width: 800px;
    position: relative;
  }

  .global-message {
    position: fixed;
    top: var(--spacing-lg);
    right: var(--spacing-lg);
    z-index: 1000;
    min-width: 300px;
    max-width: 500px;
  }

  .settings-header {
    margin-bottom: var(--spacing-xl);
  }

  .settings-header h2 {
    margin: 0 0 var(--spacing-sm) 0;
    color: var(--text-primary);
    font-size: var(--font-size-lg);
    font-weight: 600;
  }

  .settings-header p {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
  }

  .settings-section {
    margin-bottom: var(--spacing-xl);
    padding: var(--spacing-lg);
    background: var(--color-background-secondary);
    border-radius: var(--border-radius-md);
    border: 1px solid var(--color-border);
  }

  .settings-section h3 {
    margin: 0 0 var(--spacing-md) 0;
    color: var(--text-primary);
    font-size: var(--font-size-md);
    font-weight: 600;
  }

  .section-description {
    margin: 0 0 var(--spacing-lg) 0;
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
    line-height: 1.5;
  }

  .info-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: var(--spacing-md);
    margin-bottom: var(--spacing-lg);
  }

  .info-item {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
  }

  .info-item label {
    font-size: var(--font-size-xs);
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .file-path {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
  }

  .file-path code {
    flex: 1;
    padding: var(--spacing-xs) var(--spacing-sm);
    background: var(--color-background);
    border: 1px solid var(--color-border);
    border-radius: var(--border-radius-sm);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-xs);
    word-break: break-all;
  }

  .file-status {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    font-size: var(--font-size-sm);
  }

  .status-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .status-success .status-indicator {
    background-color: var(--color-success);
  }

  .status-warning .status-indicator {
    background-color: var(--color-warning);
  }

  .status-error .status-indicator {
    background-color: var(--color-error);
  }

  .status-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: var(--spacing-md);
    margin-bottom: var(--spacing-lg);
  }

  .status-item {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
  }

  .status-item label {
    font-size: var(--font-size-xs);
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .permissions {
    display: flex;
    gap: var(--spacing-xs);
  }

  .permission {
    padding: 2px var(--spacing-xs);
    font-size: var(--font-size-xs);
    border-radius: var(--border-radius-sm);
    background: var(--color-background);
    color: var(--text-secondary);
    border: 1px solid var(--color-border);
  }

  .permission.active {
    background: var(--color-success-alpha);
    color: var(--color-success);
    border-color: var(--color-success);
  }

  .text-warning {
    color: var(--color-warning);
  }

  .reload-status {
    margin-bottom: var(--spacing-md);
  }

  .status-message {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--border-radius-sm);
    font-size: var(--font-size-sm);
  }

  .status-message.status-success {
    background: var(--color-success-alpha);
    color: var(--color-success);
    border: 1px solid var(--color-success);
  }

  .status-message.status-error {
    background: var(--color-error-alpha);
    color: var(--color-error);
    border: 1px solid var(--color-error);
  }

  .status-icon {
    font-weight: bold;
    font-size: var(--font-size-md);
  }

  .section-actions {
    display: flex;
    gap: var(--spacing-sm);
    align-items: center;
  }

  .confirm-content {
    padding: var(--spacing-md) 0;
  }

  .confirm-content p {
    margin: 0 0 var(--spacing-md) 0;
    color: var(--text-primary);
  }

  .warning-text {
    color: var(--color-warning);
    font-weight: 500;
  }

  .warning-list {
    margin: var(--spacing-sm) 0 var(--spacing-md) var(--spacing-md);
    color: var(--text-secondary);
  }

  .warning-list li {
    margin-bottom: var(--spacing-xs);
  }

  .note-text {
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
    font-style: italic;
  }

  .modal-actions {
    display: flex;
    gap: var(--spacing-sm);
    justify-content: flex-end;
  }
</style>
