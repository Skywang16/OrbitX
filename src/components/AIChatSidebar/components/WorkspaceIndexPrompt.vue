<template>
  <div class="workspace-index-prompt">
    <div class="prompt-content">
      <div class="prompt-icon">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M12 2L2 7L12 12L22 7L12 2Z" stroke="currentColor" stroke-width="2" stroke-linejoin="round" />
          <path d="M2 17L12 22L22 17" stroke="currentColor" stroke-width="2" stroke-linejoin="round" />
          <path d="M2 12L12 17L22 12" stroke="currentColor" stroke-width="2" stroke-linejoin="round" />
        </svg>
      </div>

      <div class="prompt-text">
        <h3>{{ t('storage.workspace_no_index_prompt') }}</h3>
        <p>{{ t('storage.workspace_no_index_description') }}</p>
        <div v-if="currentPath" class="current-path">
          <span class="path-label">{{ t('storage.workspace_path') }}:</span>
          <span class="path-value">{{ currentPath }}</span>
        </div>
      </div>
    </div>

    <!-- 构建按钮 -->
    <div class="prompt-actions">
      <button v-if="!isBuilding" class="build-button" :disabled="!currentPath" @click="handleBuildIndex">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path
            d="M12 5V19M5 12H19"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
        {{ t('storage.build_workspace_index') }}
      </button>

      <!-- 构建进度 -->
      <div v-else class="build-progress">
        <div class="progress-header">
          <div class="progress-text">
            <span class="progress-title">{{ t('storage.building_workspace_index') }}</span>
            <span v-if="buildProgress.currentFile" class="progress-file">{{ buildProgress.currentFile }}</span>
          </div>
          <button class="cancel-button" @click="handleCancelBuild">
            {{ t('storage.cancel_build') }}
          </button>
        </div>

        <div class="progress-bar">
          <div class="progress-fill" :style="{ width: `${buildProgress.percentage}%` }"></div>
        </div>

        <div class="progress-stats">
          <span>{{ buildProgress.processedFiles }} / {{ buildProgress.totalFiles }} 文件</span>
          <span v-if="buildProgress.percentage >= 0">{{ Math.round(buildProgress.percentage) }}%</span>
        </div>
      </div>
    </div>

    <!-- 错误信息 -->
    <div v-if="errorMessage" class="error-message">
      <div class="error-content">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2" />
          <line x1="15" y1="9" x2="9" y2="15" stroke="currentColor" stroke-width="2" />
          <line x1="9" y1="9" x2="15" y2="15" stroke="currentColor" stroke-width="2" />
        </svg>
        <span>{{ errorMessage }}</span>
      </div>
      <button class="retry-button" @click="handleBuildIndex">
        {{ t('storage.retry_build') }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { workspaceIndexApi } from '@/api/workspace-index'
  import type { WorkspaceIndex } from '@/api/workspace-index'

  interface BuildProgress {
    percentage: number
    processedFiles: number
    totalFiles: number
    currentFile?: string
  }

  const { t } = useI18n()

  // Props
  interface Props {
    currentPath?: string
  }

  const props = withDefaults(defineProps<Props>(), {
    currentPath: undefined,
  })

  // Emits
  const emit = defineEmits<{
    indexBuilt: [workspace: WorkspaceIndex]
    buildStarted: []
    buildCancelled: []
  }>()

  // State
  const isBuilding = ref(false)
  const errorMessage = ref<string>('')
  const buildProgress = ref<BuildProgress>({
    percentage: 0,
    processedFiles: 0,
    totalFiles: 0,
    currentFile: undefined,
  })

  // 构建索引
  const handleBuildIndex = async () => {
    if (!props.currentPath) {
      errorMessage.value = '无法获取当前工作目录'
      return
    }

    try {
      errorMessage.value = ''
      isBuilding.value = true
      emit('buildStarted')

      // 重置进度
      buildProgress.value = {
        percentage: 0,
        processedFiles: 0,
        totalFiles: 0,
        currentFile: undefined,
      }

      // 开始构建索引
      const workspace = await workspaceIndexApi.buildWorkspaceIndex({
        path: props.currentPath,
        name: getDirectoryName(props.currentPath),
      })

      // 构建成功
      emit('indexBuilt', workspace)
    } catch (error) {
      console.error('Index build failed:', error)
      errorMessage.value = error instanceof Error ? error.message : t('storage.index_build_failed')
    } finally {
      isBuilding.value = false
    }
  }

  // 取消构建
  const handleCancelBuild = () => {
    isBuilding.value = false
    errorMessage.value = ''
    emit('buildCancelled')
  }

  // 获取目录名称
  const getDirectoryName = (path: string): string => {
    return path.split('/').pop() || path
  }

  // 模拟构建进度更新（实际应该通过事件或轮询获取）
  // const simulateBuildProgress = () => {
    if (!isBuilding.value) return

    const interval = setInterval(() => {
      if (!isBuilding.value) {
        clearInterval(interval)
        return
      }

      buildProgress.value.percentage = Math.min(buildProgress.value.percentage + Math.random() * 10, 95)
      buildProgress.value.processedFiles = Math.floor(
        (buildProgress.value.percentage / 100) * buildProgress.value.totalFiles
      )

      // 模拟当前处理文件
      const files = ['src/main.ts', 'package.json', 'README.md', 'src/components/App.vue']
      buildProgress.value.currentFile = files[Math.floor(Math.random() * files.length)]

      if (buildProgress.value.percentage >= 95) {
        clearInterval(interval)
      }
    }, 500)
  }

  onMounted(() => {
    // 设置初始总文件数（实际应该从API获取）
    buildProgress.value.totalFiles = 100
  })
</script>

<style scoped>
  .workspace-index-prompt {
    padding: 24px;
    background: var(--bg-200);
    border-radius: var(--border-radius-md);
    border: 1px solid var(--border-200);
    margin: 16px;
  }

  .prompt-content {
    display: flex;
    gap: 16px;
    margin-bottom: 20px;
  }

  .prompt-icon {
    flex-shrink: 0;
    width: 48px;
    height: 48px;
    background: var(--accent-100);
    border-radius: var(--border-radius-md);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent-600);
  }

  .prompt-text h3 {
    margin: 0 0 8px 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--text-100);
  }

  .prompt-text p {
    margin: 0 0 12px 0;
    font-size: 14px;
    color: var(--text-200);
    line-height: 1.5;
  }

  .current-path {
    font-size: 12px;
    color: var(--text-300);
  }

  .path-label {
    font-weight: 500;
  }

  .path-value {
    font-family: var(--font-mono);
    background: var(--bg-300);
    padding: 2px 6px;
    border-radius: var(--border-radius-sm);
    margin-left: 4px;
  }

  .prompt-actions {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .build-button {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    background: var(--accent-500);
    color: white;
    border: none;
    border-radius: var(--border-radius-md);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .build-button:hover:not(:disabled) {
    background: var(--accent-600);
    transform: translateY(-1px);
  }

  .build-button:disabled {
    background: var(--bg-400);
    color: var(--text-400);
    cursor: not-allowed;
  }

  .build-progress {
    background: var(--bg-100);
    border-radius: var(--border-radius-md);
    padding: 16px;
    border: 1px solid var(--border-200);
  }

  .progress-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 12px;
  }

  .progress-text {
    flex: 1;
  }

  .progress-title {
    display: block;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-100);
    margin-bottom: 4px;
  }

  .progress-file {
    display: block;
    font-size: 12px;
    color: var(--text-300);
    font-family: var(--font-mono);
  }

  .cancel-button {
    padding: 6px 12px;
    background: transparent;
    color: var(--text-300);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius-sm);
    font-size: 12px;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .cancel-button:hover {
    background: var(--bg-300);
    color: var(--text-200);
  }

  .progress-bar {
    width: 100%;
    height: 6px;
    background: var(--bg-300);
    border-radius: 3px;
    overflow: hidden;
    margin-bottom: 8px;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent-500);
    transition: width 0.3s ease;
    border-radius: 3px;
  }

  .progress-stats {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: var(--text-300);
  }

  .error-message {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px;
    background: var(--error-100);
    border: 1px solid var(--error-200);
    border-radius: var(--border-radius-md);
    margin-top: 12px;
  }

  .error-content {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--error-600);
    font-size: 14px;
  }

  .retry-button {
    padding: 6px 12px;
    background: var(--error-500);
    color: white;
    border: none;
    border-radius: var(--border-radius-sm);
    font-size: 12px;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .retry-button:hover {
    background: var(--error-600);
  }
</style>
