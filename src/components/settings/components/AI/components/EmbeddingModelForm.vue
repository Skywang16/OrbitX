<script setup lang="ts">
  import { reactive, ref } from 'vue'
  import { useI18n } from 'vue-i18n'

  export interface EmbeddingModelConfig {
    id?: string
    apiUrl: string
    apiKey: string
    modelName: string
    dimension: number
  }

  interface Props {
    config?: EmbeddingModelConfig | null
  }

  interface Emits {
    (e: 'submit', data: EmbeddingModelConfig): void
    (e: 'cancel'): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const errors = ref<Record<string, string>>({})
  const isSubmitting = ref(false)
  const isTesting = ref(false)

  // 常用预设
  const presets = [
    { name: 'OpenAI small', model: 'text-embedding-3-small', dim: 1536, url: 'https://api.openai.com/v1' },
    { name: 'OpenAI large', model: 'text-embedding-3-large', dim: 3072, url: 'https://api.openai.com/v1' },
    { name: '智谱 embedding-2', model: 'embedding-2', dim: 1024, url: 'https://open.bigmodel.cn/api/paas/v4' },
  ]

  const formData = reactive<EmbeddingModelConfig>({
    apiUrl: props.config?.apiUrl || 'https://api.openai.com/v1',
    apiKey: props.config?.apiKey || '',
    modelName: props.config?.modelName || 'text-embedding-3-small',
    dimension: props.config?.dimension || 1536,
  })

  const applyPreset = (preset: (typeof presets)[0]) => {
    formData.modelName = preset.model
    formData.dimension = preset.dim
    formData.apiUrl = preset.url
  }

  const validateForm = () => {
    errors.value = {}
    if (!formData.apiUrl.trim()) errors.value.apiUrl = t('embedding_model.validation.api_url_required')
    if (!formData.apiKey.trim()) errors.value.apiKey = t('embedding_model.validation.api_key_required')
    if (!formData.modelName.trim()) errors.value.modelName = t('embedding_model.validation.model_name_required')
    if (!formData.dimension || formData.dimension < 1)
      errors.value.dimension = t('embedding_model.validation.dimension_required')
    return Object.keys(errors.value).length === 0
  }

  const handleSubmit = () => {
    if (!validateForm()) return
    isSubmitting.value = true
    emit('submit', { ...formData })
    isSubmitting.value = false
  }

  const handleCancel = () => {
    emit('cancel')
  }

  const handleTestConnection = async () => {
    if (!validateForm()) return
    isTesting.value = true
    try {
      // TODO: 调用后端测试连接
      // await vectorDbApi.testEmbeddingConnection(formData)
      await new Promise(resolve => setTimeout(resolve, 1000))
    } finally {
      isTesting.value = false
    }
  }
</script>

<template>
  <x-modal
    :visible="true"
    :title="props.config ? t('embedding_model.edit_title') : t('embedding_model.add_title')"
    size="medium"
    show-footer
    :show-cancel-button="false"
    :show-confirm-button="false"
    @close="handleCancel"
  >
    <template #footer>
      <div class="modal-footer">
        <x-button variant="secondary" :loading="isTesting" @click="handleTestConnection">
          {{ isTesting ? t('ai_model.testing') : t('ai_model.test_connection') }}
        </x-button>
        <div class="footer-right">
          <x-button variant="secondary" @click="handleCancel">{{ t('common.cancel') }}</x-button>
          <x-button variant="primary" :loading="isSubmitting" @click="handleSubmit">
            {{ t('common.save') }}
          </x-button>
        </div>
      </div>
    </template>

    <form @submit.prevent="handleSubmit" class="embedding-form">
      <!-- 快速预设 -->
      <div class="form-row">
        <div class="form-group full-width">
          <label class="form-label">{{ t('embedding_model.presets') }}</label>
          <div class="preset-buttons">
            <x-button
              v-for="preset in presets"
              :key="preset.model"
              variant="secondary"
              size="small"
              @click="applyPreset(preset)"
            >
              {{ preset.name }}
            </x-button>
          </div>
        </div>
      </div>

      <!-- API URL -->
      <div class="form-row">
        <div class="form-group full-width">
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
        <div class="form-group full-width">
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
        <div class="form-group full-width">
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
        <div class="form-group full-width">
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
    </form>
  </x-modal>
</template>

<style scoped>
  .embedding-form {
    max-width: 500px;
    margin: 0 auto;
  }

  .form-row {
    margin-top: 16px;
    display: flex;
    gap: var(--spacing-lg);
    margin-bottom: var(--spacing-lg);
  }

  .form-group {
    flex: 1;
    min-width: 0;
  }

  .form-group.full-width {
    flex: 1 1 100%;
  }

  .form-label {
    display: block;
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-200);
    margin-bottom: 6px;
  }

  .form-input {
    width: 100%;
    padding: 8px 12px;
    font-size: var(--font-size-sm);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius-sm);
    background: var(--bg-400);
    color: var(--text-100);
    transition: border-color 0.2s;
  }

  .form-input:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  .form-input.error {
    border-color: var(--color-danger);
  }

  .form-description {
    font-size: 12px;
    color: var(--text-400);
    margin-top: 4px;
  }

  .error-message {
    font-size: 12px;
    color: var(--color-danger);
    margin-top: 4px;
  }

  .preset-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .modal-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
  }

  .footer-right {
    display: flex;
    gap: 8px;
  }
</style>
