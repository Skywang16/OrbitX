<script setup lang="ts">
  import { ref, onMounted, computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import SettingsCard from '../../../SettingsCard.vue'
  import EmbeddingModelForm from './EmbeddingModelForm.vue'

  export interface EmbeddingModelConfig {
    id?: string
    apiUrl: string
    apiKey: string
    modelName: string
    dimension: number
  }

  const { t } = useI18n()

  const showForm = ref(false)
  const currentConfig = ref<EmbeddingModelConfig | null>(null)
  const loading = ref(false)

  // 是否已配置
  const isConfigured = computed(() => {
    return currentConfig.value && currentConfig.value.apiKey && currentConfig.value.modelName
  })

  const handleConfigure = () => {
    showForm.value = true
  }

  const handleFormSubmit = async (data: EmbeddingModelConfig) => {
    // TODO: 调用后端保存配置
    currentConfig.value = data
    showForm.value = false
  }

  const handleFormCancel = () => {
    showForm.value = false
  }

  onMounted(async () => {
    loading.value = true
    try {
      // TODO: 加载现有配置
      // currentConfig.value = await vectorDbApi.getEmbeddingConfig()
    } finally {
      loading.value = false
    }
  })
</script>

<template>
  <div class="settings-group">
    <h3 class="settings-group-title">{{ t('embedding_model.title') }}</h3>

    <SettingsCard>
      <!-- 添加/配置按钮 -->
      <div class="settings-item">
        <div class="settings-item-header">
          <div class="settings-label">{{ t('embedding_model.config_label') }}</div>
          <div class="settings-description">{{ t('embedding_model.description') }}</div>
        </div>
        <div class="settings-item-control">
          <x-button variant="primary" @click="handleConfigure">
            {{ isConfigured ? t('embedding_model.edit') : t('embedding_model.configure') }}
          </x-button>
        </div>
      </div>

      <!-- 已配置的模型信息 -->
      <div v-if="isConfigured && currentConfig" class="settings-item">
        <div class="settings-item-header">
          <div class="model-info">
            <div class="settings-label">{{ currentConfig.modelName }}</div>
            <div class="settings-description">
              {{ currentConfig.apiUrl }} · {{ t('embedding_model.dimension') }}: {{ currentConfig.dimension }}
            </div>
          </div>
        </div>
      </div>

      <!-- 未配置提示 -->
      <div v-else-if="!loading" class="settings-item">
        <div class="settings-item-header">
          <div class="settings-description">{{ t('embedding_model.not_configured') }}</div>
        </div>
      </div>
    </SettingsCard>

    <!-- 弹窗表单 -->
    <EmbeddingModelForm v-if="showForm" :config="currentConfig" @submit="handleFormSubmit" @cancel="handleFormCancel" />
  </div>
</template>

<style scoped>
  .model-info {
    flex: 1;
  }

  .settings-item-control {
    display: flex;
    gap: 8px;
  }
</style>
