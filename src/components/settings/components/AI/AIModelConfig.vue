<script setup lang="ts">
  import type { AIModelConfig } from '@/types'
  import { createMessage } from '@/ui'
  import { handleError } from '@/utils/errorHandler'
  import { computed, onMounted, ref } from 'vue'
  import { useI18n } from 'vue-i18n'
  import AIModelForm from './AIModelForm.vue'
  import { useAISettingsStore } from './store'

  const { t } = useI18n()

  const aiSettingsStore = useAISettingsStore()

  const showAddForm = ref(false)
  const editingModel = ref<AIModelConfig | null>(null)

  const models = computed(() => aiSettingsStore.models)
  const loading = computed(() => aiSettingsStore.isLoading)

  onMounted(async () => {
    if (!aiSettingsStore.isInitialized) {
      await aiSettingsStore.loadSettings()
    }
  })

  const handleAddModel = () => {
    editingModel.value = null
    showAddForm.value = true
  }

  const handleEditModel = (model: AIModelConfig) => {
    editingModel.value = { ...model }
    showAddForm.value = true
  }

  const handleDeleteModel = async (modelId: string) => {
    try {
      await aiSettingsStore.removeModel(modelId)
      createMessage.success(t('ai_model.delete_success'))
    } catch (error) {
      createMessage.error(handleError(error, t('ai_model.delete_failed')))
    }
  }

  const handleFormSubmit = async (modelData: Omit<AIModelConfig, 'id'>) => {
    try {
      if (editingModel.value) {
        await aiSettingsStore.updateModel(editingModel.value.id, modelData)
        createMessage.success(t('ai_model.update_success'))
      } else {
        const newModel: AIModelConfig = {
          ...modelData,
          id: Date.now().toString(),
        }

        await aiSettingsStore.addModel(newModel)
        createMessage.success(t('ai_model.add_success'))
      }
      showAddForm.value = false
      editingModel.value = null
    } catch (error) {
      createMessage.error(handleError(error, t('ai_model.operation_failed')))
    }
  }

  const handleFormCancel = () => {
    showAddForm.value = false
    editingModel.value = null
  }
</script>

<template>
  <div class="settings-group">
    <h3 class="settings-group-title">{{ t('settings.ai.model_config') }}</h3>
    <div class="settings-item">
      <div class="settings-item-header">
        <div class="settings-label">{{ t('ai_model.add_new_model') }}</div>
        <div class="settings-description">{{ t('ai_model.add_model_description') }}</div>
      </div>
      <div class="settings-item-control">
        <x-button variant="primary" @click="handleAddModel">
          {{ t('ai_model.add_model') }}
        </x-button>
      </div>
    </div>

    <div v-if="loading" class="settings-loading">
      <div class="settings-loading-spinner"></div>
      <span>{{ t('ai_model.loading') }}</span>
    </div>

    <div v-else-if="models.length === 0" class="settings-item">
      <div class="settings-item-header">
        <div class="settings-label">{{ t('ai_model.no_models') }}</div>
        <div class="settings-description">{{ t('ai_model_config.empty_description') }}</div>
      </div>
    </div>
    <div v-else>
      <div v-for="model in models" :key="model.id" class="settings-item">
        <div class="settings-item-header">
          <div class="settings-label">{{ model.name }}</div>
          <div class="settings-description">{{ model.provider || t('ai_model.default_provider') }}</div>
        </div>
        <div class="settings-item-control">
          <x-button variant="secondary" size="small" @click="handleEditModel(model)">
            {{ t('ai_model.edit') }}
          </x-button>
          <x-popconfirm
            :title="t('ai_model.delete_confirm')"
            :description="t('ai_model.delete_description', { name: model.name })"
            type="danger"
            :confirm-text="t('ai_model.delete_confirm_text')"
            :cancel-text="t('ai_model.cancel')"
            placement="top"
            @confirm="handleDeleteModel(model.id)"
          >
            <template #trigger>
              <x-button variant="danger" size="small">
                {{ t('ai_model.delete') }}
              </x-button>
            </template>
          </x-popconfirm>
        </div>
      </div>
    </div>

    <AIModelForm v-if="showAddForm" :model="editingModel" @submit="handleFormSubmit" @cancel="handleFormCancel" />
  </div>
</template>

<style scoped>
  .ai-model-config {
    width: 100%;
  }

  .action-header {
    margin-bottom: 20px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--border-300);
  }

  .action-header :deep(.x-button) {
    background: var(--color-primary);
    color: white;
    border-radius: 4px;
    padding: 8px 12px;
    font-size: 13px;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .action-header :deep(.x-button:hover) {
    background: var(--color-primary-hover);
  }

  .section-title {
    font-size: 20px;
    color: var(--text-100);
    margin-bottom: 8px;
  }

  .section-description {
    font-size: 13px;
    color: var(--text-400);
  }

  .empty-state {
    text-align: center;
    padding: 48px 24px;
    color: var(--text-400);
    background: var(--bg-500);
    border-radius: 4px;
  }

  .empty-icon {
    margin-bottom: 16px;
    color: var(--text-300);
  }

  .empty-title {
    font-size: 16px;
    color: var(--text-200);
    margin-bottom: 8px;
  }

  .empty-description {
    font-size: 13px;
  }

  .model-cards {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .model-card {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--bg-500);
    border-radius: 4px;
    padding: 12px 16px;
  }

  .model-card:hover {
    background: var(--bg-400);
  }

  .model-name {
    font-size: 14px;
    color: var(--text-200);
  }

  .model-actions {
    display: flex;
    gap: 8px;
  }

  .model-actions :deep(.x-button) {
    background: transparent;
    border: 1px solid var(--border-200);
    color: var(--text-300);
    border-radius: 4px;
    padding: 4px 8px;
    font-size: 12px;
  }

  .model-actions :deep(.x-button:hover) {
    background: var(--bg-300);
    color: var(--text-200);
  }

  .model-actions :deep(.x-button[variant='danger']) {
    border-color: var(--error-border);
    color: var(--error-text);
  }

  .model-actions :deep(.x-button[variant='danger']:hover) {
    background: var(--error-bg);
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 48px 24px;
    color: var(--text-400);
    background: var(--bg-500);
    border-radius: 4px;
  }

  .loading-state p {
    font-size: 13px;
  }

  .loading-spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border-300);
    border-top: 2px solid var(--color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 12px;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
