<script setup lang="ts">
  import type { AIModelConfig } from '@/types'
  import type { AIModelTestConnectionInput } from '@/api/ai/types'

  import { aiApi } from '@/api'
  import { reactive, ref, computed, onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useLLMRegistry } from '@/composables/useLLMRegistry'

  interface Props {
    model?: AIModelConfig | null
    defaultModelType?: 'chat' | 'embedding'
  }

  interface Emits {
    (e: 'submit', data: Omit<AIModelConfig, 'id'>): void
    (e: 'cancel'): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  // 使用后端LLM注册表
  const { providers, providerOptions, getChatModelOptions, loadProviders } = useLLMRegistry()

  const selectedProvider = ref<string>('')
  const errors = ref<Record<string, string>>({})
  const isSubmitting = ref(false)
  const isTesting = ref(false)

  const formData = reactive({
    provider: '' as AIModelConfig['provider'],
    apiUrl: '',
    apiKey: '',
    model: '',
    modelType: 'chat' as AIModelConfig['modelType'],
    options: {
      maxContextTokens: 128000,
      temperature: 0.7,
      timeout: 300000,
      supportsImages: false,
      supportsPromptCache: false,
      contextWindow: 128000,
      maxTokens: -1, // -1 表示使用模型默认值
    },
    useCustomBaseUrl: false,
    customBaseUrl: '',
  })

  // 高级选项展开状态
  const showAdvancedOptions = ref(false)

  // 当前服务商信息
  const providerInfo = computed(() => {
    return providers.value.find(p => p.providerType.toLowerCase() === selectedProvider.value.toLowerCase())
  })
  // 是否有预设模型
  const hasPresetModels = computed(() => {
    return providerInfo.value?.presetModels && providerInfo.value.presetModels.length > 0
  })
  // 可用模型列表
  const availableModels = computed(() => (hasPresetModels.value ? getChatModelOptions(selectedProvider.value) : []))

  onMounted(() => loadProviders())

  // 初始化
  if (props.model) {
    Object.assign(formData, props.model, {
      options: props.model.options || formData.options,
      useCustomBaseUrl: false,
      customBaseUrl: '',
    })
    selectedProvider.value = String(props.model.provider)
  }

  const handleProviderChange = (value: string) => {
    selectedProvider.value = value
    formData.provider = value as AIModelConfig['provider']

    const info = providerInfo.value
    if (!info) return

    // 设置默认 API URL
    formData.apiUrl = info.defaultApiUrl

    // 如果有预设模型，自动选择第一个
    const models = getChatModelOptions(value)
    if (models.length > 0) {
      formData.model = models[0].value
    } else {
      formData.model = ''
    }

    // 重置自定义 URL 选项
    formData.useCustomBaseUrl = false
    formData.customBaseUrl = ''
  }

  const handleModelChange = (value: string) => {
    // Model selection updated
  }

  const handleUseCustomBaseUrlChange = () => {
    if (formData.useCustomBaseUrl) {
      formData.customBaseUrl = ''
      formData.apiUrl = ''
    } else {
      formData.apiUrl = providerInfo.value?.defaultApiUrl || ''
      formData.customBaseUrl = ''
    }
  }

  const validateForm = () => {
    errors.value = {}
    if (!selectedProvider.value) errors.value.provider = t('ai_model.validation.preset_required')
    if (!formData.apiKey.trim()) errors.value.apiKey = t('ai_model.validation.api_key_required')

    if (hasPresetModels.value) {
      if (!formData.model) errors.value.model = t('ai_model.validation.model_required')
      if (formData.useCustomBaseUrl && !formData.customBaseUrl) {
        errors.value.customBaseUrl = t('ai_model.validation.custom_base_url_required')
      }
    } else {
      if (!formData.apiUrl) errors.value.apiUrl = t('ai_model.validation.api_url_required')
      if (!formData.model) errors.value.model = t('ai_model.validation.model_name_required')
    }

    return Object.keys(errors.value).length === 0
  }

  const handleSubmit = () => {
    if (!validateForm()) return

    isSubmitting.value = true
    const submitData = { ...formData }
    if (formData.useCustomBaseUrl) submitData.apiUrl = formData.customBaseUrl
    emit('submit', submitData)
    isSubmitting.value = false
  }

  const handleCancel = () => {
    emit('cancel')
  }

  const handleTestConnection = async () => {
    if (!validateForm()) return

    isTesting.value = true
    try {
      const testConfig: AIModelTestConnectionInput = {
        provider: formData.provider,
        apiUrl: formData.apiUrl,
        apiKey: formData.apiKey,
        model: formData.model,
        modelType: formData.modelType,
        options: formData.options,
      }

      await aiApi.testConnectionWithConfig(testConfig)
    } finally {
      isTesting.value = false
    }
  }
</script>

<template>
  <x-modal
    :visible="true"
    :title="props.model ? t('ai_model.edit_title') : t('ai_model.add_title')"
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
            {{ props.model ? t('common.save') : t('common.add') }}
          </x-button>
        </div>
      </div>
    </template>

    <form @submit.prevent="handleSubmit" class="ai-form">
      <input type="hidden" v-model="formData.modelType" value="chat" />

      <div class="form-row">
        <div class="form-group full-width">
          <label class="form-label">{{ t('ai_model.provider') }}</label>
          <x-select
            v-model="selectedProvider"
            :options="providerOptions.map(p => ({ value: p.value, label: p.label }))"
            :placeholder="t('ai_model.select_provider')"
            @update:modelValue="handleProviderChange"
          />
          <div v-if="errors.provider" class="error-message">{{ errors.provider }}</div>
        </div>
      </div>

      <template v-if="hasPresetModels">
        <div class="form-row">
          <div class="form-group full-width">
            <label class="form-label">{{ t('ai_model.model') }}</label>
            <x-select
              v-model="formData.model"
              :options="availableModels"
              :placeholder="t('ai_model.select_model')"
              @update:modelValue="handleModelChange"
            />
            <div v-if="errors.model" class="error-message">{{ errors.model }}</div>
          </div>
        </div>

        <div class="form-row">
          <div class="form-group full-width">
            <label class="form-label checkbox-label">
              <input
                type="checkbox"
                v-model="formData.useCustomBaseUrl"
                @change="handleUseCustomBaseUrlChange"
                class="form-checkbox"
              />
              <span>{{ t('ai_model.use_custom_base_url') }}</span>
            </label>
          </div>
        </div>

        <div v-if="formData.useCustomBaseUrl" class="form-row">
          <div class="form-group full-width">
            <label class="form-label">{{ t('ai_model.custom_base_url') }}</label>
            <input
              v-model="formData.customBaseUrl"
              type="url"
              class="form-input"
              :class="{ error: errors.customBaseUrl }"
              :placeholder="t('ai_model.custom_base_url_placeholder')"
            />
            <div v-if="errors.customBaseUrl" class="error-message">{{ errors.customBaseUrl }}</div>
          </div>
        </div>

        <div class="form-row">
          <div class="form-group full-width">
            <label class="form-label">{{ t('ai_model.api_key') }}</label>
            <input
              v-model="formData.apiKey"
              type="password"
              class="form-input"
              :class="{ error: errors.apiKey }"
              :placeholder="t('ai_model.api_key_placeholder')"
            />
            <div v-if="errors.apiKey" class="error-message">{{ errors.apiKey }}</div>
          </div>
        </div>
      </template>

      <template v-else-if="selectedProvider">
        <div class="form-row">
          <div class="form-group full-width">
            <label class="form-label">{{ t('ai_model.model_name') }} *</label>
            <input
              v-model="formData.model"
              type="text"
              class="form-input"
              :class="{ error: errors.model }"
              :placeholder="t('ai_model.model_name_placeholder')"
            />
            <div v-if="errors.model" class="error-message">{{ errors.model }}</div>
          </div>
        </div>

        <div class="form-row">
          <div class="form-group full-width">
            <label class="form-label">{{ t('ai_model.api_url') }} *</label>
            <input
              v-model="formData.apiUrl"
              type="url"
              class="form-input"
              :class="{ error: errors.apiUrl }"
              :placeholder="t('ai_model.api_url_placeholder')"
            />
            <div v-if="errors.apiUrl" class="error-message">{{ errors.apiUrl }}</div>
          </div>
        </div>

        <div class="form-row">
          <div class="form-group full-width">
            <label class="form-label">{{ t('ai_model.api_key') }} *</label>
            <input
              v-model="formData.apiKey"
              type="password"
              class="form-input"
              :class="{ error: errors.apiKey }"
              :placeholder="t('ai_model.api_key_placeholder')"
            />
            <div v-if="errors.apiKey" class="error-message">{{ errors.apiKey }}</div>
          </div>
        </div>

        <div class="form-row">
          <div class="form-group full-width">
            <label class="form-label">{{ t('ai_model.context_window') }} *</label>
            <input
              v-model.number="formData.options.contextWindow"
              type="number"
              class="form-input"
              placeholder="128000"
              min="1000"
              max="2000000"
            />
            <div class="form-description">{{ t('ai_model.context_window_description') }}</div>
          </div>
        </div>

        <div class="form-row">
          <div class="form-group full-width">
            <button type="button" class="advanced-toggle" @click="showAdvancedOptions = !showAdvancedOptions">
              <svg
                class="toggle-icon"
                :class="{ expanded: showAdvancedOptions }"
                width="12"
                height="12"
                viewBox="0 0 12 12"
                fill="none"
                xmlns="http://www.w3.org/2000/svg"
              >
                <path d="M4.5 3L7.5 6L4.5 9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
              </svg>
              <span>{{ t('ai_model.advanced_options') }}</span>
            </button>
          </div>
        </div>

        <div v-if="showAdvancedOptions" class="advanced-options">
          <div class="form-row">
            <div class="form-group full-width">
              <label class="form-label">{{ t('ai_model.max_output_tokens') }}</label>
              <input
                v-model.number="formData.options.maxTokens"
                type="number"
                class="form-input"
                placeholder="-1"
                min="-1"
                max="200000"
              />
              <div class="form-description">{{ t('ai_model.max_output_tokens_description') }}</div>
            </div>
          </div>

          <div class="form-row">
            <div class="form-group full-width">
              <label class="form-label">{{ t('ai_model.feature_support') }}</label>
              <div class="checkbox-group">
                <label class="checkbox-label">
                  <input type="checkbox" v-model="formData.options.supportsImages" class="form-checkbox" />
                  <span>{{ t('ai_model.supports_images') }}</span>
                </label>
                <label class="checkbox-label">
                  <input type="checkbox" v-model="formData.options.supportsPromptCache" class="form-checkbox" />
                  <span>{{ t('ai_model.supports_prompt_cache') }}</span>
                </label>
              </div>
            </div>
          </div>
        </div>
      </template>
    </form>
  </x-modal>
</template>

<style scoped>
  .ai-form {
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
    margin-bottom: var(--spacing-sm);
  }

  .form-input {
    width: 100%;
    height: 32px;
    padding: 0 var(--spacing-md);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius);
    background-color: var(--bg-400);
    color: var(--text-200);
    font-size: var(--font-size-md);
    font-family: var(--font-family);
    line-height: 1.5;
    transition: all var(--x-duration-normal) var(--x-ease-out);
    box-sizing: border-box;
  }

  .form-input:focus {
    outline: none;
    box-shadow: 0 0 0 2px var(--color-primary-alpha);
  }

  .form-input.error:focus {
    box-shadow: 0 0 0 2px rgba(244, 71, 71, 0.1);
  }

  .error-message {
    font-size: var(--font-size-xs);
    color: var(--color-error);
    margin-top: var(--spacing-xs);
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    line-height: 1.4;
  }

  .modal-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--spacing-lg);
  }

  .footer-right {
    display: flex;
    gap: var(--spacing-md);
  }

  .form-description {
    font-size: var(--font-size-xs);
    color: var(--text-400);
    margin-top: var(--spacing-xs);
    line-height: 1.4;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    cursor: pointer;
    user-select: none;
  }

  .checkbox-label span {
    color: var(--text-200);
    font-size: var(--font-size-sm);
  }

  .form-checkbox {
    width: 18px;
    height: 18px;
    cursor: pointer;
    accent-color: var(--color-primary);
  }

  .checkbox-group {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm) 0;
  }

  .advanced-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 0;
    background: none;
    border: none;
    color: var(--text-300);
    font-size: var(--font-size-sm);
    font-weight: 500;
    cursor: pointer;
  }

  .toggle-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
    color: var(--text-400);
  }

  .toggle-icon.expanded {
    transform: rotate(90deg);
  }

  .advanced-options {
    margin-top: 0;
    padding-left: 18px;
  }
</style>
