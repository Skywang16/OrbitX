<script setup lang="ts">
  import type { AIModelConfig } from '@/types'

  import { computed, onMounted, ref } from 'vue'
  import { useI18n } from 'vue-i18n'
  import AIModelForm from './AIModelForm.vue'
  import { useAISettingsStore } from '../store'
  import SettingsCard from '../../../SettingsCard.vue'

  const { t } = useI18n()

  const aiSettingsStore = useAISettingsStore()

  const showAddForm = ref(false)
  const editingModel = ref<AIModelConfig | null>(null)

  // 只显示 Chat 模型
  const models = computed(() => aiSettingsStore.chatModels)
  const loading = computed(() => aiSettingsStore.isLoading)

  onMounted(async () => {
    await aiSettingsStore.loadModels()
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
    await aiSettingsStore.removeModel(modelId)
  }

  const handleFormSubmit = async (modelData: Omit<AIModelConfig, 'id'>) => {
    if (editingModel.value) {
      await aiSettingsStore.updateModel(editingModel.value.id, modelData)
    } else {
      await aiSettingsStore.addModel({ ...modelData, modelType: 'chat' })
    }
    showAddForm.value = false
    editingModel.value = null
  }

  const handleFormCancel = () => {
    showAddForm.value = false
    editingModel.value = null
  }
</script>

<template>
  <div class="settings-group">
    <h3 class="settings-group-title">{{ t('settings.ai.model_config') }}</h3>

    <SettingsCard>
      <div class="settings-item">
        <div class="settings-item-header">
          <div class="settings-label">{{ t('ai_model.add_new_model') }}</div>
          <div class="settings-description">{{ t('ai_model.add_model_description') }}</div>
        </div>
        <div class="settings-item-control">
          <x-button variant="primary" @click="handleAddModel">
            {{ t('ai_model.add_chat_model') }}
          </x-button>
        </div>
      </div>
    </SettingsCard>

    <div v-if="loading" class="settings-loading">
      <div class="settings-loading-spinner"></div>
      <span>{{ t('ai_model.loading') }}</span>
    </div>

    <SettingsCard v-else-if="models.length === 0">
      <div class="settings-item">
        <div class="settings-item-header">
          <div class="settings-label">{{ t('ai_model.no_models') }}</div>
          <div class="settings-description">{{ t('ai_model_config.empty_description') }}</div>
        </div>
      </div>
    </SettingsCard>

    <div v-else>
      <div class="model-section">
        <h4 class="settings-group-title">{{ t('ai_model.chat') }}</h4>

        <SettingsCard>
          <div v-for="model in models" :key="model.id" class="settings-item">
            <div class="settings-item-header">
              <div class="model-info">
                <div class="settings-label">{{ model.model }}</div>
                <div class="settings-description">{{ model.provider }}</div>
              </div>
            </div>
            <div class="settings-item-control">
              <x-button variant="primary" size="small" @click="handleEditModel(model)">
                {{ t('ai_model.edit') }}
              </x-button>
              <x-popconfirm
                :title="t('ai_model.delete_confirm')"
                :description="t('ai_model.delete_description', { name: model.model })"
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
        </SettingsCard>
      </div>
    </div>

    <AIModelForm v-if="showAddForm" :model="editingModel" @submit="handleFormSubmit" @cancel="handleFormCancel" />
  </div>
</template>

<style scoped>
  .ai-model-config {
    width: 100%;
  }

  .model-section {
    margin-bottom: 24px;
  }

  .settings-item-control {
    display: flex;
    gap: 8px;
  }

  .model-info {
    flex: 1;
  }

  .action-header {
    margin-bottom: 20px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--border-200);
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
    border-radius: var(--border-radius-sm);
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
    border-radius: var(--border-radius-sm);
    padding: 12px 16px;
  }

  .model-card:hover {
    background: var(--bg-500);
  }

  .model-name {
    font-size: 14px;
    color: var(--text-200);
  }

  .model-actions {
    display: flex;
    gap: 8px;
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 48px 24px;
    color: var(--text-400);
    background: var(--bg-500);
    border-radius: var(--border-radius-sm);
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
