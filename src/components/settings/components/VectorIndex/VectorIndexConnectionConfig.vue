<script setup lang="ts">
  import { ref, reactive, computed, onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useVectorIndexSettingsStore } from './store'
  import { useAISettingsStore } from '../AI/store'

  const { t } = useI18n()
  const settingsStore = useVectorIndexSettingsStore()
  const aiSettingsStore = useAISettingsStore()

  const isTestingConnection = ref(false)

  // 配置表单数据
  const configForm = reactive({
    qdrantUrl: '',
    qdrantApiKey: '',
    collectionName: '',
    embeddingModelId: '', // 必需：选择的embedding模型ID
    maxConcurrentFiles: 4,
  })

  // 计算属性：可用的embedding模型
  const availableEmbeddingModels = computed(() => {
    return aiSettingsStore.models.filter(model => model.modelType === 'embedding')
  })

  // 监听embedding模型选择变化
  const handleEmbeddingModelChange = (modelId: string) => {
    // 向量维度将由后端根据模型自动推断，前端不再需要处理
    console.log('选择的embedding模型ID:', modelId)
  }

  // 加载当前配置
  const loadCurrentConfig = () => {
    if (settingsStore.config) {
      Object.assign(configForm, {
        qdrantUrl: settingsStore.config.qdrantUrl || '',
        qdrantApiKey: settingsStore.config.qdrantApiKey || '',
        collectionName: settingsStore.config.collectionName || '',
        embeddingModelId: settingsStore.config.embeddingModelId || '',
        maxConcurrentFiles: settingsStore.config.maxConcurrentFiles || 4,
      })
    }
  }

  // 保存配置
  const saveConfig = async () => {
    await settingsStore.saveConfig(configForm)
  }

  // 测试连接
  const testConnection = async () => {
    if (!configForm.qdrantUrl.trim()) return

    isTestingConnection.value = true

    const testConfig = {
      qdrantUrl: configForm.qdrantUrl,
      qdrantApiKey: configForm.qdrantApiKey || null,
      collectionName: configForm.collectionName,
      embeddingModelId: configForm.embeddingModelId,
      maxConcurrentFiles: configForm.maxConcurrentFiles,
    }

    await settingsStore.testConnection(testConfig)
    isTestingConnection.value = false
  }

  // 计算属性：表单是否有效
  const isFormValid = computed(() => {
    return (
      configForm.qdrantUrl.trim() &&
      configForm.collectionName.trim() &&
      configForm.embeddingModelId.trim() &&
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
