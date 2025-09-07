<script setup lang="ts">
  import { ref, reactive, computed, onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useVectorIndexSettingsStore } from './store'
  import { useAISettingsStore } from '../AI/store'
  import { createMessage } from '@/ui'
  import { handleErrorWithMessage } from '@/utils/errorHandler'

  const { t } = useI18n()
  const settingsStore = useVectorIndexSettingsStore()
  const aiSettingsStore = useAISettingsStore()

  const isTestingConnection = ref(false)

  // 配置表单数据
  const configForm = reactive({
    qdrantUrl: '',
    qdrantApiKey: '',
    collectionName: '',
    vectorSize: 1536,
    batchSize: 50,
    maxConcurrentFiles: 4,
    embeddingModelId: '', // 新增：选择的embedding模型ID
  })

  // 计算属性：可用的embedding模型
  const availableEmbeddingModels = computed(() => {
    return aiSettingsStore.models.filter(model => model.modelType === 'embedding')
  })

  // 监听embedding模型选择变化，自动设置向量维度
  const handleEmbeddingModelChange = (modelId: string) => {
    const selectedModel = availableEmbeddingModels.value.find(m => m.id === modelId)
    if (selectedModel) {
      // 根据模型名称推断向量维度
      const modelName = selectedModel.model.toLowerCase()
      if (modelName.includes('text-embedding-3-small') || modelName.includes('text-embedding-ada-002')) {
        configForm.vectorSize = 1536
      } else if (modelName.includes('text-embedding-3-large')) {
        configForm.vectorSize = 3072
      } else {
        // 默认维度，用户可以手动调整
        configForm.vectorSize = 1536
      }
    }
  }

  // 加载当前配置
  const loadCurrentConfig = () => {
    if (settingsStore.config) {
      Object.assign(configForm, {
        qdrantUrl: settingsStore.config.qdrantUrl || '',
        qdrantApiKey: settingsStore.config.qdrantApiKey || '',
        collectionName: settingsStore.config.collectionName || '',
        vectorSize: settingsStore.config.vectorSize || 1536,
        batchSize: settingsStore.config.batchSize || 50,
        maxConcurrentFiles: settingsStore.config.maxConcurrentFiles || 4,
        embeddingModelId: settingsStore.config.embeddingModelId || '',
      })
    }
  }

  // 保存配置
  const saveConfig = async () => {
    try {
      await settingsStore.saveConfig(configForm)
      createMessage.success(t('settings.vectorIndex.config_saved'))
    } catch (error) {
      handleErrorWithMessage(error, t('settings.vectorIndex.config_save_failed'))
    }
  }

  // 测试连接
  const testConnection = async () => {
    if (!configForm.qdrantUrl.trim()) {
      createMessage.warning(t('settings.vectorIndex.url_required'))
      return
    }

    isTestingConnection.value = true

    try {
      const testConfig = {
        qdrantUrl: configForm.qdrantUrl,
        qdrantApiKey: configForm.qdrantApiKey || null,
        collectionName: configForm.collectionName,
        vectorSize: configForm.vectorSize,
        batchSize: configForm.batchSize,
        maxConcurrentFiles: configForm.maxConcurrentFiles,
        chunkSizeRange: [10, 2000] as [number, number],
        supportedExtensions: ['.ts', '.tsx', '.js', '.jsx', '.rs', '.py', '.go', '.java', '.c', '.cpp', '.h', '.hpp'],
        ignorePatterns: ['**/node_modules/**', '**/target/**', '**/dist/**', '**/.git/**', '**/build/**'],
        embeddingModelId: configForm.embeddingModelId || undefined,
      }

      await settingsStore.testConnection(testConfig)
      createMessage.success(t('settings.vectorIndex.connection_test_success'))
    } catch (error) {
      handleErrorWithMessage(error, t('settings.vectorIndex.connection_test_failed'))
    } finally {
      isTestingConnection.value = false
    }
  }

  // 计算属性：表单是否有效
  const isFormValid = computed(() => {
    return (
      configForm.qdrantUrl.trim() &&
      configForm.collectionName.trim() &&
      configForm.vectorSize > 0 &&
      configForm.batchSize > 0 &&
      configForm.maxConcurrentFiles > 0
    )
  })

  // 初始化时加载配置和AI模型
  onMounted(async () => {
    if (!aiSettingsStore.isInitialized) {
      await aiSettingsStore.loadSettings()
    }
    loadCurrentConfig()
  })
</script>

<template>
  <div class="settings-group">
    <h3 class="settings-group-title">{{ t('settings.vectorIndex.connection_config') }}</h3>

    <div class="settings-description" style="margin-bottom: 16px">
      {{ t('settings.vectorIndex.connection_description') }}
    </div>

    <!-- Qdrant 数据库URL -->
    <div class="settings-item">
      <div class="settings-item-header">
        <div class="settings-label">{{ t('settings.vectorIndex.qdrant_url') }}</div>
        <div class="settings-description">{{ t('settings.vectorIndex.qdrant_url_description') }}</div>
      </div>
      <div class="settings-item-control">
        <input
          v-model="configForm.qdrantUrl"
          type="text"
          class="settings-input"
          :placeholder="t('settings.vectorIndex.qdrant_url_placeholder')"
        />
      </div>
    </div>

    <!-- API密钥 -->
    <div class="settings-item">
      <div class="settings-item-header">
        <div class="settings-label">{{ t('settings.vectorIndex.api_key') }}</div>
        <div class="settings-description">{{ t('settings.vectorIndex.api_key_description') }}</div>
      </div>
      <div class="settings-item-control">
        <input
          v-model="configForm.qdrantApiKey"
          type="password"
          class="settings-input"
          :placeholder="t('settings.vectorIndex.api_key_placeholder')"
        />
      </div>
    </div>

    <!-- 集合名称 -->
    <div class="settings-item">
      <div class="settings-item-header">
        <div class="settings-label">{{ t('settings.vectorIndex.collection_name') }}</div>
        <div class="settings-description">{{ t('settings.vectorIndex.collection_name_description') }}</div>
      </div>
      <div class="settings-item-control">
        <input
          v-model="configForm.collectionName"
          type="text"
          class="settings-input"
          :placeholder="t('settings.vectorIndex.collection_name_placeholder')"
        />
      </div>
    </div>

    <!-- Embedding模型选择 -->
    <div class="settings-item">
      <div class="settings-item-header">
        <div class="settings-label">{{ t('settings.vectorIndex.embedding_model') }}</div>
        <div class="settings-description">{{ t('settings.vectorIndex.embedding_model_description') }}</div>
      </div>
      <div class="settings-item-control">
        <x-select
          v-model="configForm.embeddingModelId"
          :options="
            availableEmbeddingModels.map(model => ({
              label: `${model.name} (${model.provider})`,
              value: model.id,
            }))
          "
          :placeholder="t('settings.vectorIndex.select_embedding_model')"
          @update:value="handleEmbeddingModelChange"
        />
        <x-button
          v-if="availableEmbeddingModels.length === 0"
          variant="primary"
          size="small"
          @click="
            () => {
              /* 导航到AI设置页面 */
            }
          "
        >
          {{ t('settings.vectorIndex.add_embedding_model') }}
        </x-button>
      </div>
    </div>

    <!-- 向量维度 -->
    <div class="settings-item">
      <div class="settings-item-header">
        <div class="settings-label">{{ t('settings.vectorIndex.vector_size') }}</div>
        <div class="settings-description">{{ t('settings.vectorIndex.vector_size_description') }}</div>
      </div>
      <div class="settings-item-control">
        <input
          v-model.number="configForm.vectorSize"
          type="number"
          class="settings-input"
          min="128"
          max="4096"
          step="1"
          :placeholder="t('settings.vectorIndex.vector_size_placeholder')"
        />
        <div class="vector-size-hint">
          <span v-if="configForm.vectorSize === 1536" class="hint-text">
            {{ t('settings.vectorIndex.common_for_openai_small') }}
          </span>
          <span v-else-if="configForm.vectorSize === 3072" class="hint-text">
            {{ t('settings.vectorIndex.common_for_openai_large') }}
          </span>
        </div>
      </div>
    </div>

    <!-- 批处理大小 -->
    <div class="settings-item">
      <div class="settings-item-header">
        <div class="settings-label">{{ t('settings.vectorIndex.batch_size') }}</div>
        <div class="settings-description">{{ t('settings.vectorIndex.batch_size_description') }}</div>
      </div>
      <div class="settings-item-control">
        <input
          v-model.number="configForm.batchSize"
          type="number"
          class="settings-input"
          min="10"
          max="200"
          step="10"
        />
      </div>
    </div>

    <!-- 并发文件数 -->
    <div class="settings-item">
      <div class="settings-item-header">
        <div class="settings-label">{{ t('settings.vectorIndex.max_concurrent_files') }}</div>
        <div class="settings-description">{{ t('settings.vectorIndex.max_concurrent_files_description') }}</div>
      </div>
      <div class="settings-item-control">
        <input
          v-model.number="configForm.maxConcurrentFiles"
          type="number"
          class="settings-input"
          min="1"
          max="16"
          step="1"
        />
      </div>
    </div>

    <!-- 连接测试和保存 -->
    <div class="settings-item">
      <div class="settings-item-header">
        <div class="settings-label">{{ t('settings.vectorIndex.test_and_save') }}</div>
        <div class="settings-description">{{ t('settings.vectorIndex.test_connection_description') }}</div>
      </div>
      <div class="settings-item-control">
        <x-button
          variant="secondary"
          :loading="isTestingConnection"
          :disabled="!configForm.qdrantUrl.trim()"
          @click="testConnection"
        >
          {{ t('settings.vectorIndex.test_connection') }}
        </x-button>
        <x-button
          variant="primary"
          :disabled="!isFormValid || settingsStore.isSaving"
          :loading="settingsStore.isSaving"
          @click="saveConfig"
        >
          {{ t('common.save') }}
        </x-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .settings-input {
    width: 100%;
    max-width: 300px;
    height: 32px;
    padding: 0 12px;
    background: var(--bg-500);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius);
    color: var(--text-200);
    font-size: 13px;
    transition: border-color 0.2s ease;
  }

  .settings-input:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  .settings-input::placeholder {
    color: var(--text-400);
  }
  .settings-item-control {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }

  .vector-size-hint {
    display: flex;
    align-items: center;
    margin-left: 8px;
  }

  .hint-text {
    font-size: 11px;
    color: var(--text-400);
    font-style: italic;
  }
</style>
