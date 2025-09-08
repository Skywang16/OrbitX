<script setup lang="ts">
  import { ref, onMounted, onUnmounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useVectorIndexSettingsStore } from './store'
  import { createMessage } from '@/ui'
  import { handleErrorWithMessage } from '@/utils/errorHandler'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'

  const { t } = useI18n()
  const settingsStore = useVectorIndexSettingsStore()

  const isBuildingIndex = ref(false)
  const buildProgress = ref(0)
  const buildStats = ref({
    totalFiles: 0,
    processedFiles: 0,
    totalChunks: 0,
    currentFile: '',
    elapsedTime: 0,
  })

  let unlistenProgress: UnlistenFn | null = null

  // 开始构建索引
  const startBuildIndex = async () => {
    if (!settingsStore.config?.qdrantUrl) {
      createMessage.warning(t('settings.vectorIndex.configure_connection_first'))
      return
    }

    try {
      isBuildingIndex.value = true
      buildProgress.value = 0
      buildStats.value = {
        totalFiles: 0,
        processedFiles: 0,
        totalChunks: 0,
        currentFile: '',
        elapsedTime: 0,
      }

      // 监听构建进度事件
      unlistenProgress = await listen('vector-index-event', event => {
        const { type, data } = event.payload as {
          type: 'progress' | 'completed' | 'error'
          data: unknown
        }

        switch (type) {
          case 'progress': {
            const d = data as { progress: number; processedFiles: number; totalFiles: number; currentFile?: string }
            buildProgress.value = d.progress * 100
            buildStats.value = {
              ...buildStats.value,
              processedFiles: d.processedFiles,
              totalFiles: d.totalFiles,
              currentFile: d.currentFile || '',
            }
            break
          }

          case 'completed':
            isBuildingIndex.value = false
            {
              const d = data as { totalFiles: number; totalChunks: number; elapsedTime: number }
              buildStats.value = {
                ...buildStats.value,
                totalChunks: d.totalChunks,
                elapsedTime: d.elapsedTime,
              }
            }
            createMessage.success(
              t('settings.vectorIndex.build_completed', {
                files: (data as any).totalFiles,
                chunks: (data as any).totalChunks,
                time: Math.round((data as any).elapsedTime / 1000),
              })
            )
            break

          case 'error':
            isBuildingIndex.value = false
            handleErrorWithMessage(
              new Error((data as { message: string }).message),
              t('settings.vectorIndex.build_failed')
            )
            break
        }
      })

      await settingsStore.buildCodeIndex()
    } catch (error) {
      isBuildingIndex.value = false
      handleErrorWithMessage(error, t('settings.vectorIndex.build_start_failed'))
    }
  }

  // 取消构建索引
  const cancelBuildIndex = async () => {
    try {
      await settingsStore.cancelBuildIndex()
      isBuildingIndex.value = false
      buildProgress.value = 0
      createMessage.info(t('settings.vectorIndex.build_cancelled'))
    } catch (error) {
      handleErrorWithMessage(error, t('settings.vectorIndex.cancel_failed'))
    }
  }

  // 获取索引状态
  const refreshIndexStatus = async () => {
    try {
      await settingsStore.refreshIndexStatus()
    } catch (error) {
      handleErrorWithMessage(error, t('settings.vectorIndex.status_refresh_failed'))
    }
  }

  // 清除索引
  const clearIndex = async () => {
    try {
      await settingsStore.clearIndex()
      createMessage.success(t('settings.vectorIndex.index_cleared'))
    } catch (error) {
      handleErrorWithMessage(error, t('settings.vectorIndex.clear_failed'))
    }
  }

  onMounted(() => {
    refreshIndexStatus()
  })

  onUnmounted(() => {
    if (unlistenProgress) {
      unlistenProgress()
    }
  })
</script>

<template>
  <div class="settings-group">
    <h3 class="settings-group-title">{{ t('settings.vectorIndex.index_management') }}</h3>

    <div class="settings-description" style="margin-bottom: 16px">
      {{ t('settings.vectorIndex.index_management_description') }}
    </div>

    <!-- 索引状态显示 -->
    <div class="settings-item" v-if="settingsStore.indexStatus">
      <div class="settings-item-header">
        <div class="settings-label">{{ t('settings.vectorIndex.current_status') }}</div>
        <div class="settings-description">
          <span v-if="settingsStore.indexStatus.isInitialized" class="status-badge success">
            {{ t('settings.vectorIndex.status_ready') }}
          </span>
          <span v-else class="status-badge warning">
            {{ t('settings.vectorIndex.status_not_ready') }}
          </span>
        </div>
      </div>
      <div class="settings-item-control">
        <x-button variant="secondary" @click="refreshIndexStatus">
          {{ t('common.refresh') }}
        </x-button>
      </div>
    </div>

    <!-- 索引统计信息 -->
    <div class="settings-item" v-if="settingsStore.indexStatus?.totalVectors">
      <div class="settings-item-header">
        <div class="settings-label">{{ t('settings.vectorIndex.index_statistics') }}</div>
        <div class="settings-description">
          {{ t('settings.vectorIndex.total_vectors', { count: settingsStore.indexStatus.totalVectors }) }}
          <br />
          {{
            t('settings.vectorIndex.last_updated', {
              time: settingsStore.indexStatus.lastUpdated
                ? new Date(settingsStore.indexStatus.lastUpdated).toLocaleString()
                : t('config.unknown_time'),
            })
          }}
        </div>
      </div>
    </div>

    <!-- 构建索引控制 -->
    <div class="settings-item">
      <div class="settings-item-header">
        <div class="settings-label">{{ t('settings.vectorIndex.build_index') }}</div>
        <div class="settings-description">
          <span v-if="!isBuildingIndex">
            {{ t('settings.vectorIndex.build_description') }}
          </span>
          <span v-else>
            {{ t('settings.vectorIndex.building_progress', { progress: Math.round(buildProgress) }) }}
            <br />
            <span v-if="buildStats.currentFile" class="current-file">
              {{ t('settings.vectorIndex.processing_file', { file: buildStats.currentFile }) }}
            </span>
          </span>
        </div>
      </div>
      <div class="settings-item-control">
        <x-button
          v-if="!isBuildingIndex"
          variant="primary"
          :disabled="!settingsStore.config?.qdrantUrl"
          @click="startBuildIndex"
        >
          {{ t('settings.vectorIndex.start_build') }}
        </x-button>
        <x-button v-else variant="secondary" @click="cancelBuildIndex">
          {{ t('settings.vectorIndex.cancel_build') }}
        </x-button>
      </div>
    </div>

    <!-- 构建进度条 -->
    <div v-if="isBuildingIndex" class="progress-container">
      <div class="progress-bar">
        <div class="progress-fill" :style="{ width: `${buildProgress}%` }"></div>
      </div>
      <div class="progress-stats">
        <span>{{ buildStats.processedFiles }} / {{ buildStats.totalFiles }} {{ t('settings.vectorIndex.files') }}</span>
        <span v-if="buildStats.totalChunks">{{ buildStats.totalChunks }} {{ t('settings.vectorIndex.chunks') }}</span>
      </div>
    </div>

    <!-- 清除索引 -->
    <div class="settings-item">
      <div class="settings-item-header">
        <div class="settings-label">{{ t('settings.vectorIndex.clear_index') }}</div>
        <div class="settings-description">{{ t('settings.vectorIndex.clear_description') }}</div>
      </div>
      <div class="settings-item-control">
        <x-popconfirm
          :title="t('settings.vectorIndex.clear_confirm_title')"
          :content="t('settings.vectorIndex.clear_confirm_content')"
          @confirm="clearIndex"
        >
          <x-button variant="danger" :disabled="!settingsStore.indexStatus?.isInitialized">
            {{ t('settings.vectorIndex.clear_index') }}
          </x-button>
        </x-popconfirm>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .status-badge {
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
  }

  .status-badge.success {
    background: var(--color-success-alpha);
    color: var(--color-success);
  }

  .status-badge.warning {
    background: var(--color-warning-alpha);
    color: var(--color-warning);
  }

  .current-file {
    font-family: var(--font-family-mono);
    font-size: 11px;
    color: var(--text-400);
  }

  .progress-container {
    margin-top: 12px;
    padding: 16px;
    background: var(--bg-400);
    border-radius: var(--border-radius);
  }

  .progress-bar {
    width: 100%;
    height: 8px;
    background: var(--bg-500);
    border-radius: 4px;
    overflow: hidden;
    margin-bottom: 8px;
  }

  .progress-fill {
    height: 100%;
    background: var(--color-primary);
    transition: width 0.3s ease;
  }

  .progress-stats {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: var(--text-400);
  }
</style>
