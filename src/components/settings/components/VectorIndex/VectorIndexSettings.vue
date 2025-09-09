<template>
  <div class="settings-group">
    <h2 class="settings-section-title">{{ t('settings.vectorIndex.title') }}</h2>

    <!-- 功能开关 -->
    <div class="settings-group">
      <h3 class="settings-group-title">{{ t('settings.vectorIndex.feature_toggle') }}</h3>
      <SettingsCard>
        <div class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">{{ t('settings.vectorIndex.feature_enabled') }}</div>
            <div class="settings-description">{{ t('settings.vectorIndex.feature_enabled_description') }}</div>
          </div>
          <div class="settings-item-control">
            <!-- 连接状态指示器 -->
            <span v-if="globalSettings.enabled" style="margin-right: 8px; color: var(--text-400); font-size: 12px">
              <template v-if="isConnecting">{{ t('common.loading') }}</template>
              <template v-else>
                <span :style="{ color: isConnected ? 'var(--color-success)' : 'var(--color-error)' }">
                  {{ connectionStatusText }}
                </span>
              </template>
            </span>
            <x-switch v-model="globalSettings.enabled" :disabled="isConnecting" @update:modelValue="onEnabledChange" />
          </div>
        </div>
      </SettingsCard>
    </div>

    <!-- 工作区管理 -->
    <div class="settings-group">
      <h3 class="settings-group-title">{{ t('settings.vectorIndex.workspace_management') }}</h3>

      <SettingsCard v-if="globalSettings.workspaces.length > 0">
        <div v-for="(workspace, index) in globalSettings.workspaces" :key="index" class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">{{ workspace }}</div>
            <div class="settings-description">{{ t('settings.vectorIndex.workspace_path_description') }}</div>
          </div>
          <div class="settings-item-control">
            <x-button variant="secondary" @click="removeWorkspace(index)">
              {{ t('settings.vectorIndex.remove_workspace') }}
            </x-button>
          </div>
        </div>
      </SettingsCard>

      <SettingsCard v-else>
        <div class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">{{ t('settings.vectorIndex.no_workspaces') }}</div>
            <div class="settings-description">{{ t('settings.vectorIndex.no_workspaces_description') }}</div>
          </div>
        </div>
      </SettingsCard>
    </div>

    <!-- 连接配置 -->
    <div class="settings-group">
      <h3 class="settings-group-title">{{ t('settings.vectorIndex.connection_config') }}</h3>

      <div class="settings-description" style="margin-bottom: 16px">
        {{ t('settings.vectorIndex.connection_description') }}
      </div>

      <SettingsCard>
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
              @click="navigateToAISettings"
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
            <x-popconfirm
              :title="t('settings.vectorIndex.save_confirm_title')"
              :description="t('settings.vectorIndex.save_confirm_description')"
              :confirm-text="t('common.save')"
              :cancel-text="t('common.cancel')"
              type="info"
              placement="top"
              trigger-text="保存配置"
              trigger-button-variant="primary"
              :trigger-button-props="{ disabled: !isFormValid || isSaving, loading: isSaving }"
              @confirm="saveConfig"
            />
          </div>
        </div>
      </SettingsCard>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, reactive, computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { createMessage } from '@/ui'
  import { vectorIndexApi } from '@/api/vector-index'
  import { vectorIndexAppSettingsApi } from '@/api/vector-index/app-settings'
  import { useAISettingsStore } from '../AI/store'
  import type { VectorIndexConfig } from '@/api/vector-index'

  // UI 组件
  import XSwitch from '@/ui/components/Switch.vue'
  import XButton from '@/ui/components/Button.vue'
  import XSelect from '@/ui/components/Select.vue'
  import XPopconfirm from '@/ui/components/Popconfirm.vue'
  import SettingsCard from '../../SettingsCard.vue'

  const { t } = useI18n()
  const aiSettingsStore = useAISettingsStore()

  // 全局设置状态
  const globalSettings = reactive({
    enabled: false,
    workspaces: [] as string[],
  })

  // 连接状态
  const isConnecting = ref(false)
  const isConnected = ref<boolean | null>(null)

  // 配置表单数据
  const configForm = reactive({
    qdrantUrl: '',
    qdrantApiKey: '',
    collectionName: '',
    embeddingModelId: '',
    maxConcurrentFiles: 4,
  })

  // 状态管理
  const isInitialized = ref(false)
  const isLoading = ref(false)
  const isSaving = ref(false)
  const isTestingConnection = ref(false)

  // 计算属性
  const connectionStatusText = computed(() => {
    if (isConnecting.value) return '...'
    if (isConnected.value === true) return t('settings.vectorIndex.connection_success')
    if (isConnected.value === false) return t('settings.vectorIndex.connection_failed')
    // 当功能开启但连接状态为 null 时，显示需要重启的提示
    if (globalSettings.enabled) {
      return t('settings.vectorIndex.restart_required')
    }
    return t('settings.vectorIndex.not_initialized')
  })

  const availableEmbeddingModels = computed(() => {
    return aiSettingsStore.models.filter(model => model.modelType === 'embedding')
  })

  const isFormValid = computed(() => {
    return (
      configForm.qdrantUrl.trim() &&
      configForm.collectionName.trim() &&
      configForm.embeddingModelId.trim() &&
      configForm.maxConcurrentFiles > 0
    )
  })

  // 加载全局设置
  const loadGlobalSettings = async () => {
    try {
      const appSettings = await vectorIndexAppSettingsApi.getSettings()
      globalSettings.enabled = appSettings.enabled
      globalSettings.workspaces = [...appSettings.workspaces]
    } catch (error) {
      console.error('加载向量索引全局设置失败:', error)
      createMessage.error(
        t('settings.vectorIndex.load_error', { error: error instanceof Error ? error.message : String(error) })
      )
    }
  }

  // 加载连接配置
  const loadConnectionConfig = async () => {
    try {
      const config = await vectorIndexApi.getConfig()
      Object.assign(configForm, {
        qdrantUrl: config.qdrantUrl || '',
        qdrantApiKey: config.qdrantApiKey || '',
        collectionName: config.collectionName || '',
        embeddingModelId: config.embeddingModelId || '',
        maxConcurrentFiles: config.maxConcurrentFiles || 4,
      })
    } catch (error) {
      console.error('加载向量索引配置失败:', error)
    }
  }

  // 统一的设置加载函数
  const loadAllSettings = async () => {
    if (isLoading.value) return

    isLoading.value = true

    try {
      // 并行加载所有设置
      await Promise.all([loadGlobalSettings(), loadConnectionConfig()])

      isInitialized.value = true
    } catch (error) {
      console.error('加载设置失败:', error)
      createMessage.error(
        t('settings.vectorIndex.load_error', { error: error instanceof Error ? error.message : String(error) })
      )
    } finally {
      isLoading.value = false
    }
  }

  // 保存全局设置
  const saveGlobalSettings = async () => {
    try {
      await vectorIndexAppSettingsApi.saveSettings({
        enabled: globalSettings.enabled,
        workspaces: globalSettings.workspaces,
      })
    } catch (error) {
      console.error('保存向量索引全局设置失败:', error)
    }
  }

  // 保存连接配置
  const saveConfig = async () => {
    isSaving.value = true

    try {
      const configToSave: VectorIndexConfig = {
        qdrantUrl: configForm.qdrantUrl || 'http://localhost:6334',
        qdrantApiKey: configForm.qdrantApiKey || null,
        collectionName: configForm.collectionName || 'orbitx-code-vectors',
        embeddingModelId: configForm.embeddingModelId || '',
        maxConcurrentFiles: configForm.maxConcurrentFiles || 4,
      }

      await vectorIndexApi.saveConfig(configToSave)

      // 重新初始化向量索引服务
      await vectorIndexApi.init(configToSave)

      createMessage.success(t('common.save_success'))
    } finally {
      isSaving.value = false
    }
  }

  // 测试连接
  const testConnection = async () => {
    if (!configForm.qdrantUrl.trim()) return

    isTestingConnection.value = true

    try {
      const testConfig = {
        qdrantUrl: configForm.qdrantUrl,
        qdrantApiKey: configForm.qdrantApiKey || null,
        collectionName: configForm.collectionName,
        embeddingModelId: configForm.embeddingModelId,
        maxConcurrentFiles: configForm.maxConcurrentFiles,
      }

      const result = await vectorIndexApi.testConnection(testConfig)
      createMessage.success(result)
    } catch (error) {
      console.error('测试连接失败:', error)
      createMessage.error(
        t('settings.vectorIndex.connection_test_failed', {
          error: error instanceof Error ? error.message : String(error),
        })
      )
    } finally {
      isTestingConnection.value = false
    }
  }

  // 开关变更处理
  const onEnabledChange = async (enabled: boolean) => {
    if (enabled) {
      // 开启功能时，先检查配置是否完整
      if (!isFormValid.value) {
        // 如果配置不完整，阻止开启并重置开关状态
        globalSettings.enabled = false
        createMessage.error(t('settings.vectorIndex.config_incomplete'))
        return
      }

      // 配置完整，测试连接
      isConnecting.value = true
      try {
        const testConfig = {
          qdrantUrl: configForm.qdrantUrl,
          qdrantApiKey: configForm.qdrantApiKey || null,
          collectionName: configForm.collectionName,
          embeddingModelId: configForm.embeddingModelId,
          maxConcurrentFiles: configForm.maxConcurrentFiles,
        }

        await vectorIndexApi.testConnection(testConfig)
        isConnected.value = true

        // 测试成功，保存设置
        await saveGlobalSettings()
        createMessage.success(t('settings.vectorIndex.feature_enabled_success'))
        createMessage.info(t('settings.vectorIndex.feature_enabled_restart_required'))
      } catch (error) {
        // 测试连接失败，阻止开启功能
        globalSettings.enabled = false
        isConnected.value = false
        // 只显示一个错误信息，不重复显示连接测试失败的信息
        createMessage.info(t('settings.vectorIndex.enable_failed_connection'))
      } finally {
        isConnecting.value = false
      }
    } else {
      // 关闭功能，直接保存设置
      await saveGlobalSettings()
      createMessage.info(t('settings.vectorIndex.feature_disabled_restart_required'))
      // 重置连接状态
      isConnected.value = null
    }
  }

  // 手动尝试连接数据库（仅用于手动测试）
  const tryConnect = async () => {
    isConnecting.value = true

    try {
      // 获取现有配置并尝试测试连接
      const cfg = await vectorIndexApi.getConfig()
      await vectorIndexApi.testConnection(cfg)

      // 初始化服务
      await vectorIndexApi.init(cfg)
      isConnected.value = true
    } catch (e) {
      isConnected.value = false
      console.error('连接失败:', e)
    } finally {
      isConnecting.value = false
    }
  }

  // 移除工作目录
  const removeWorkspace = async (index: number) => {
    const workspace = globalSettings.workspaces[index]

    try {
      await vectorIndexAppSettingsApi.removeWorkspace(workspace)
      globalSettings.workspaces.splice(index, 1)
      createMessage.success(t('settings.vectorIndex.workspace_removed'))
    } catch (error) {
      console.error('移除工作目录失败:', error)
      createMessage.error(
        t('settings.vectorIndex.remove_workspace_failed', {
          error: error instanceof Error ? error.message : String(error),
        })
      )
    }
  }

  // Embedding模型变化处理
  const handleEmbeddingModelChange = (modelId: string) => {
    // 向量维度将由后端根据模型自动推断，前端不再需要处理
    void modelId // 避免未使用变量警告
  }

  // 定义事件
  const emit = defineEmits<{
    navigateToSection: [section: string]
  }>()

  // 导航到AI设置页面
  const navigateToAISettings = () => {
    emit('navigateToSection', 'ai')
  }

  // 初始化方法，供外部调用
  const init = async () => {
    // 确保AI设置已加载（用于获取embedding模型列表）
    if (!aiSettingsStore.isInitialized) {
      await aiSettingsStore.loadSettings()
    }

    // 加载所有向量索引设置
    await loadAllSettings()
  }

  // 暴露初始化方法给父组件
  defineExpose({
    init,
  })
</script>

<style scoped>
  .settings-input {
    width: 100%;
    max-width: 300px;
    height: 32px;
    padding: 0 12px;
    background: var(--bg-500);
    border: none;
    border-radius: var(--border-radius);
    color: var(--text-200);
    font-size: 13px;
  }

  .settings-input:focus {
    outline: none;
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
</style>
