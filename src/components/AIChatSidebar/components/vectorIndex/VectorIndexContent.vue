<script setup lang="ts">
  import { computed, ref, watch, onMounted, reactive } from 'vue'
  import { terminalContextApi, aiApi } from '@/api'
  import { useI18n } from 'vue-i18n'
  import { useEditorStore } from '@/stores/Editor'
  import { useTerminalStore } from '@/stores/Terminal'
  import { homeDir } from '@tauri-apps/api/path'
  import { getPathBasename } from '@/utils/path'
  import { useAISettingsStore } from '@/components/settings/components/AI/store'
  import type { AIModelTestConnectionInput } from '@/api/ai/types'
  import { AuthType } from '@/types/oauth'

  const { t } = useI18n()
  const aiSettingsStore = useAISettingsStore()

  interface Props {
    indexStatus: {
      hasIndex: boolean
      path: string
      size?: string
    }
    isBuilding?: boolean
    buildProgress?: number
  }

  interface Emits {
    (e: 'build'): void
    (e: 'delete'): void
    (e: 'refresh'): void
    (e: 'cancel'): void
  }

  const props = withDefaults(defineProps<Props>(), {
    isBuilding: false,
    buildProgress: 0,
  })
  const emit = defineEmits<Emits>()

  const handleBuild = () => {
    emit('build')
  }

  const handleDelete = () => {
    emit('delete')
  }

  const handleRefresh = () => {
    emit('refresh')
  }

  const handleCancel = () => {
    emit('cancel')
  }

  const statusText = computed(() => {
    if (props.indexStatus.hasIndex) {
      return t('ck.index_ready')
    }
    return t('ck.index_not_ready')
  })

  const tabManagerStore = useEditorStore()
  const terminalStore = useTerminalStore()

  const displayPath = ref(props.indexStatus.path)
  const resolvedPath = ref<string>(props.indexStatus.path || '.')
  const homePath = ref<string>('')
  const indexSize = ref<string>('')

  const normalize = (p: string) => p.replace(/\\/g, '/').replace(/\/$/, '')

  const refreshDisplayPath = async () => {
    let p = props.indexStatus.path
    if (!p || p === '.') {
      const activeTab = tabManagerStore.activeTab
      if (activeTab && activeTab.type === 'terminal') {
        const terminal = terminalStore.terminals.find(t => t.id === activeTab.context.paneId)
        if (terminal?.cwd) p = terminal.cwd
      }
      if (!p || p === '.') {
        try {
          const ctx = await terminalContextApi.getActiveTerminalContext()
          const cwd = ctx?.currentWorkingDirectory
          if (cwd) p = cwd
        } catch (e) {
          console.error('Failed to get active terminal context', e)
        }
      }
    }
    resolvedPath.value = p || '.'
    displayPath.value = p && p !== '.' ? getPathBasename(p) : '.'
  }

  const canBuild = computed(() => {
    // 首先检查是否配置了 embedding 模型
    if (!hasEmbeddingModel.value) return false

    const pRaw = resolvedPath.value
    if (!pRaw) return false
    const p = normalize(pRaw)
    if (p === '.' || p === '~' || p === '/' || /^[A-Za-z]:$/.test(p)) return false
    if (homePath.value) {
      const h = normalize(homePath.value)
      if (p === h) return false
    }
    return true
  })

  watch(
    () => props.indexStatus,
    newStatus => {
      refreshDisplayPath()
      // 直接使用 indexStatus 中的 size 字段
      if (newStatus.hasIndex && newStatus.path) {
        indexSize.value = newStatus.size || ''
      } else {
        indexSize.value = ''
      }
    },
    { deep: true, immediate: true }
  )

  onMounted(async () => {
    refreshDisplayPath()
    homeDir()
      .then(path => (homePath.value = path))
      .catch(() => {})
    // 加载向量模型配置
    await aiSettingsStore.loadModels()
  })

  // ========== 向量模型配置 ==========
  interface EmbeddingModelConfig {
    apiUrl: string
    apiKey: string
    modelName: string
    dimension: number
  }

  // 当前配置的向量模型
  const embeddingModel = computed(() => aiSettingsStore.embeddingModels[0] || null)
  const hasEmbeddingModel = computed(() => !!embeddingModel.value)

  // 表单数据
  const formData = reactive<EmbeddingModelConfig>({
    apiUrl: '',
    apiKey: '',
    modelName: '',
    dimension: 1536,
  })

  const errors = ref<Record<string, string>>({})
  const isSubmitting = ref(false)
  const isTesting = ref(false)
  const showForm = ref(false)

  // 监听 embeddingModel 变化，同步到表单
  watch(
    embeddingModel,
    model => {
      if (model) {
        formData.apiUrl = model.apiUrl || 'https://api.openai.com/v1'
        formData.apiKey = model.apiKey || ''
        formData.modelName = model.model || 'text-embedding-3-small'
        formData.dimension = model.options?.dimension || 1536
      }
    },
    { immediate: true }
  )

  const validateForm = () => {
    errors.value = {}
    if (!formData.apiUrl.trim()) errors.value.apiUrl = t('embedding_model.validation.api_url_required')
    if (!formData.apiKey.trim()) errors.value.apiKey = t('embedding_model.validation.api_key_required')
    if (!formData.modelName.trim()) errors.value.modelName = t('embedding_model.validation.model_name_required')
    if (!formData.dimension || formData.dimension < 1)
      errors.value.dimension = t('embedding_model.validation.dimension_required')
    return Object.keys(errors.value).length === 0
  }

  const handleSaveEmbeddingModel = async () => {
    if (!validateForm()) return
    isSubmitting.value = true
    try {
      const modelData = {
        provider: 'openai_compatible' as const,
        authType: AuthType.ApiKey,
        apiUrl: formData.apiUrl,
        apiKey: formData.apiKey,
        model: formData.modelName,
        modelType: 'embedding' as const,
        options: { dimension: formData.dimension },
      }
      if (embeddingModel.value) {
        await aiSettingsStore.updateModel(embeddingModel.value.id, modelData)
      } else {
        await aiSettingsStore.addModel(modelData)
      }
      showForm.value = false
    } finally {
      isSubmitting.value = false
    }
  }

  const handleTestConnection = async () => {
    if (!validateForm()) return
    isTesting.value = true
    try {
      const testConfig: AIModelTestConnectionInput = {
        provider: 'openai_compatible',
        authType: AuthType.ApiKey,
        apiUrl: formData.apiUrl,
        apiKey: formData.apiKey,
        model: formData.modelName,
        modelType: 'embedding',
        options: { dimension: formData.dimension },
      }
      await aiApi.testConnectionWithConfig(testConfig)
    } finally {
      isTesting.value = false
    }
  }

  const handleDeleteEmbeddingModel = async () => {
    if (embeddingModel.value) {
      await aiSettingsStore.removeModel(embeddingModel.value.id)
    }
  }

  const handleEditEmbeddingModel = () => {
    showForm.value = true
  }

  const handleCancelForm = () => {
    showForm.value = false
    // 重置表单到当前模型状态
    if (embeddingModel.value) {
      formData.apiUrl = embeddingModel.value.apiUrl || 'https://api.openai.com/v1'
      formData.apiKey = embeddingModel.value.apiKey || ''
      formData.modelName = embeddingModel.value.model || 'text-embedding-3-small'
      formData.dimension = embeddingModel.value.options?.dimension || 1536
    }
    errors.value = {}
  }
</script>

<template>
  <div class="vector-index-content">
    <div class="header">
      <div class="title-section">
        <h3 class="title">{{ t('ck.title') }}</h3>
        <p class="subtitle">{{ t('ck.subtitle') }}</p>
      </div>
    </div>

    <div class="body">
      <!-- 向量模型配置区域 -->
      <div class="embedding-config-section">
        <div class="section-header">
          <span class="section-title">{{ t('ck.embedding_model') }}</span>
        </div>

        <!-- 已配置模型：显示模型信息 -->
        <div v-if="hasEmbeddingModel && !showForm" class="model-display">
          <div class="model-info">
            <div class="model-name">{{ embeddingModel?.model }}</div>
            <div class="model-detail">
              {{ t('embedding_model.dimension') }}: {{ embeddingModel?.options?.dimension }}
            </div>
          </div>
          <div class="model-actions">
            <x-button size="small" variant="secondary" @click="handleEditEmbeddingModel">
              {{ t('ai_model.edit') }}
            </x-button>
            <x-button size="small" variant="danger" @click="handleDeleteEmbeddingModel">
              {{ t('ai_model.delete') }}
            </x-button>
          </div>
        </div>

        <!-- 未配置或编辑模式：显示表单 -->
        <div v-if="!hasEmbeddingModel || showForm" class="embedding-form">
          <!-- API URL -->
          <div class="form-row">
            <div class="form-group">
              <label class="form-label">{{ t('embedding_model.api_url') }} *</label>
              <input
                v-model="formData.apiUrl"
                type="url"
                class="form-input"
                :class="{ error: errors.apiUrl }"
                placeholder="https://api.openai.com/v1"
              />
              <div v-if="errors.apiUrl" class="error-message">{{ errors.apiUrl }}</div>
            </div>
          </div>

          <!-- API Key -->
          <div class="form-row">
            <div class="form-group">
              <label class="form-label">{{ t('embedding_model.api_key') }} *</label>
              <input
                v-model="formData.apiKey"
                type="password"
                class="form-input"
                :class="{ error: errors.apiKey }"
                :placeholder="t('embedding_model.api_key_placeholder')"
              />
              <div v-if="errors.apiKey" class="error-message">{{ errors.apiKey }}</div>
            </div>
          </div>

          <!-- Model Name -->
          <div class="form-row">
            <div class="form-group">
              <label class="form-label">{{ t('embedding_model.model_name') }} *</label>
              <input
                v-model="formData.modelName"
                type="text"
                class="form-input"
                :class="{ error: errors.modelName }"
                placeholder="text-embedding-3-small"
              />
              <div v-if="errors.modelName" class="error-message">{{ errors.modelName }}</div>
            </div>
          </div>

          <!-- Dimension -->
          <div class="form-row">
            <div class="form-group">
              <label class="form-label">{{ t('embedding_model.dimension') }} *</label>
              <input
                v-model.number="formData.dimension"
                type="number"
                class="form-input"
                :class="{ error: errors.dimension }"
                placeholder="1536"
                min="64"
                max="8192"
              />
              <div class="form-description">{{ t('embedding_model.dimension_hint') }}</div>
              <div v-if="errors.dimension" class="error-message">{{ errors.dimension }}</div>
            </div>
          </div>

          <!-- 表单按钮 -->
          <div class="form-actions">
            <x-button variant="secondary" size="small" :loading="isTesting" @click="handleTestConnection">
              {{ isTesting ? t('ai_model.testing') : t('ai_model.test_connection') }}
            </x-button>
            <div class="form-actions-right">
              <x-button v-if="showForm" variant="secondary" size="small" @click="handleCancelForm">
                {{ t('common.cancel') }}
              </x-button>
              <x-button variant="primary" size="small" :loading="isSubmitting" @click="handleSaveEmbeddingModel">
                {{ t('common.save') }}
              </x-button>
            </div>
          </div>
        </div>
      </div>

      <div class="divider"></div>
      <div class="workspace-section">
        <div class="workspace-info">
          <div class="workspace-label">{{ t('ck.current_workspace') }}</div>
          <div class="workspace-path">{{ displayPath }}</div>
        </div>

        <!-- 构建中状态：显示横向进度条 + 取消按钮 -->
        <div v-if="isBuilding" class="building-section">
          <div class="progress-container">
            <div class="progress-bar-wrapper">
              <div class="progress-bar">
                <div class="progress-fill" :style="{ width: buildProgress + '%' }"></div>
              </div>
              <span class="progress-text">{{ Math.round(buildProgress) }}%</span>
            </div>
          </div>
          <x-button size="small" variant="secondary" @click="handleCancel">
            {{ t('ck.cancel_build') }}
          </x-button>
        </div>
        <template v-else>
          <!-- 未构建状态：显示构建按钮 -->
          <x-button
            v-if="!indexStatus.hasIndex"
            variant="primary"
            :disabled="!canBuild"
            :title="!canBuild ? t('ck.build_index_tooltip_disabled') : t('ck.build_index_tooltip_enabled')"
            @click="handleBuild"
          >
            {{ t('ck.build_index_button') }}
          </x-button>

          <!-- 已构建：显示状态 + 操作按钮 -->
          <div v-else class="indexed-section">
            <div class="status-row">
              <div class="status-info">
                <span class="status-text">{{ statusText }}</span>
                <div class="index-size-info" v-if="indexSize">
                  <span class="size-value">{{ indexSize }}</span>
                </div>
              </div>
              <div class="action-buttons">
                <x-button size="small" variant="secondary" :disabled="!canBuild" @click="handleBuild">
                  {{ t('ck.build_index_button') }}
                </x-button>
                <x-button size="small" variant="primary" @click="handleRefresh">
                  <template #icon>
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                      <path d="M3 3v5h5" />
                      <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />
                      <path d="M21 21v-5h-5" />
                    </svg>
                  </template>
                  {{ t('ck.refresh') }}
                </x-button>
                <x-button size="small" variant="danger" @click="handleDelete">
                  <template #icon>
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <polyline points="3,6 5,6 21,6"></polyline>
                      <path
                        d="m19,6v14a2,2 0 0,1 -2,2H7a2,2 0 0,1 -2,-2V6m3,0V4a2,2 0 0,1 2,-2h4a2,2 0 0,1 2,2v2"
                      ></path>
                      <line x1="10" x2="10" y1="11" y2="17"></line>
                      <line x1="14" x2="14" y1="11" y2="17"></line>
                    </svg>
                  </template>
                  {{ t('ck.delete') }}
                </x-button>
              </div>
            </div>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .vector-index-content {
    overflow: hidden;
  }
  .header {
    padding: var(--spacing-lg) var(--spacing-lg) var(--spacing-md) var(--spacing-lg);
    border-bottom: 1px solid var(--border-200);
  }
  .title-section {
    text-align: left;
  }
  .title {
    font-size: var(--font-size-lg);
    font-weight: 600;
    color: var(--text-100);
    margin: 0 0 var(--spacing-xs) 0;
  }
  .subtitle {
    font-size: var(--font-size-sm);
    color: var(--text-300);
    margin: 0;
    line-height: 1.4;
  }
  .body {
    padding: var(--spacing-lg);
  }
  .workspace-section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
  }
  .workspace-info {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
  }
  .workspace-label {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-200);
  }
  .workspace-path {
    font-size: var(--font-size-sm);
    color: var(--text-100);
    font-family: var(--font-family-mono);
    background: var(--bg-300);
    padding: var(--spacing-sm);
    border-radius: var(--border-radius-sm);
    border: 1px solid var(--border-200);
    word-break: break-all;
  }
  .indexed-section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
  }
  .status-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--spacing-md);
  }
  .status-info {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
  }
  .status-text {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-100);
  }
  .index-size-info {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    font-size: var(--font-size-xs);
  }
  .size-value {
    color: var(--text-200);
    font-family: var(--font-family-mono);
    background: var(--bg-300);
    padding: 2px var(--spacing-xs);
    border-radius: var(--border-radius-xs);
    border: 1px solid var(--border-200);
  }
  .action-buttons {
    display: flex;
    gap: var(--spacing-sm);
  }
  .action-buttons :deep(.x-button) {
    gap: 6px;
  }
  .building-section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
  }
  .progress-container {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
  }
  .progress-bar-wrapper {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
  }
  .progress-bar {
    flex: 1;
    height: 8px;
    background: var(--bg-300);
    border-radius: var(--border-radius-sm);
    overflow: hidden;
    border: 1px solid var(--border-200);
  }
  .progress-fill {
    height: 100%;
    background: var(--color-primary);
    transition: width 0.3s ease;
    border-radius: var(--border-radius-sm);
  }
  .progress-text {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-200);
    min-width: 45px;
    text-align: right;
    font-family: var(--font-family-mono);
  }

  /* 向量模型配置区域样式 */
  .embedding-config-section {
    margin-bottom: var(--spacing-md);
  }
  .section-header {
    margin-bottom: var(--spacing-sm);
  }
  .section-title {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-200);
  }
  .model-display {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--bg-300);
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--border-radius-sm);
    border: 1px solid var(--border-200);
  }
  .model-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .model-name {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-100);
    font-family: var(--font-family-mono);
  }
  .model-detail {
    font-size: var(--font-size-xs);
    color: var(--text-300);
  }
  .model-actions {
    display: flex;
    gap: var(--spacing-xs);
  }
  .divider {
    height: 1px;
    background: var(--border-200);
    margin: var(--spacing-md) 0;
  }
  .embedding-form {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
  }
  .form-row {
    display: flex;
    flex-direction: column;
  }
  .form-group {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
  }
  .form-label {
    font-size: var(--font-size-xs);
    font-weight: 500;
    color: var(--text-200);
  }
  .form-input {
    width: 100%;
    height: 32px;
    padding: 0 var(--spacing-sm);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius-sm);
    background-color: var(--bg-400);
    color: var(--text-200);
    font-size: var(--font-size-sm);
    font-family: var(--font-family);
    box-sizing: border-box;
    transition: all 0.15s ease;
  }
  .form-input:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px var(--color-primary-alpha);
  }
  .form-input.error {
    border-color: var(--color-error);
  }
  .form-description {
    font-size: var(--font-size-xs);
    color: var(--text-400);
    line-height: 1.4;
  }
  .error-message {
    font-size: var(--font-size-xs);
    color: var(--color-error);
    line-height: 1.4;
  }
  .form-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: var(--spacing-sm);
  }
  .form-actions-right {
    display: flex;
    gap: var(--spacing-xs);
  }
</style>
