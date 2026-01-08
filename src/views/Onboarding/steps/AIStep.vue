<template>
  <div class="ai-step">
    <Transition name="fade-slide">
      <div v-if="!selectedProvider" class="step-header">
        <h2 class="step-title">{{ t('onboarding.ai.title') }}</h2>
        <p class="step-description">{{ t('onboarding.ai.description') }}</p>
      </div>
    </Transition>

    <div class="ai-options">
      <TransitionGroup name="provider-list" tag="div" class="provider-container">
        <div
          v-for="provider in visibleProviders"
          :key="provider.id"
          class="ai-option"
          :class="{
            selected: selectedProvider === provider.id,
            expanded: selectedProvider === provider.id,
            'move-to-top': selectedProvider === provider.id,
          }"
        >
          <div class="ai-option-header" @click="selectProvider(provider.id)">
            <div class="ai-info">
              <div class="ai-name">{{ provider.name }}</div>
            </div>
          </div>

          <Transition name="dropdown" appear>
            <div v-if="selectedProvider === provider.id" class="ai-config-dropdown">
              <div class="config-divider"></div>

              <template v-if="hasPresetModels">
                <div class="form-group">
                  <label class="form-label">{{ t('ai_model.model') }}</label>
                  <x-select
                    v-model="formData.model"
                    :options="availableModels"
                    :placeholder="t('ai_model.select_model')"
                    :class="{ error: errors.model }"
                  />
                  <div v-if="errors.model" class="error-message">{{ errors.model }}</div>
                </div>

                <div class="form-group">
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

                <div v-if="formData.useCustomBaseUrl" class="form-group">
                  <label class="form-label">{{ t('ai_model.custom_base_url') }}</label>
                  <input
                    v-model="formData.apiUrl"
                    type="url"
                    class="form-input"
                    :class="{ error: errors.apiUrl }"
                    :placeholder="t('ai_model.custom_base_url_placeholder')"
                  />
                  <div v-if="errors.apiUrl" class="error-message">{{ errors.apiUrl }}</div>
                </div>

                <div class="form-group">
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
              </template>

              <template v-else-if="selectedProvider">
                <div class="form-group">
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

                <div class="form-group">
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

                <div class="form-group">
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

                <div class="form-group">
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

                <div class="form-group">
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

                <div v-if="showAdvancedOptions" class="advanced-options">
                  <div class="form-group">
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
              </template>
            </div>
          </Transition>
        </div>
      </TransitionGroup>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, reactive, computed, onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { v4 as uuidv4 } from 'uuid'
  import { XSelect } from '@/ui'

  import type { AIModelConfig } from '@/types'
  import { useAISettingsStore } from '@/components/settings/components/AI/store'
  import { useLLMRegistry } from '@/composables/useLLMRegistry'

  const { t } = useI18n()
  const aiSettingsStore = useAISettingsStore()
  const { providers, providerOptions, getChatModelOptions, loadProviders } = useLLMRegistry()
  const selectedProvider = ref('')

  const availableProviders = computed(() => {
    return providerOptions.value.map(provider => ({
      id: provider.value,
      name: provider.label,
    }))
  })

  onMounted(async () => {
    if (providerOptions.value.length === 0) {
      await loadProviders()
    }
  })

  const formData = reactive({
    provider: 'anthropic' as AIModelConfig['provider'],
    apiKey: '',
    apiUrl: '',
    model: '',
    options: {
      maxContextTokens: 128000,
      temperature: 0.7,
      timeout: 300000,
      contextWindow: 128000,
      maxTokens: -1,
    },
    useCustomBaseUrl: false,
  })

  const errors = ref<Record<string, string>>({})
  const isSubmitting = ref(false)
  const showAdvancedOptions = ref(false)

  const visibleProviders = computed(() => {
    if (!selectedProvider.value) {
      return availableProviders.value
    }
    return availableProviders.value.filter(provider => provider.id === selectedProvider.value)
  })

  const providerInfo = computed(() => {
    return providers.value.find(p => p.providerType.toLowerCase() === selectedProvider.value.toLowerCase())
  })

  const hasPresetModels = computed(() => {
    return providerInfo.value?.presetModels && providerInfo.value.presetModels.length > 0
  })

  const availableModels = computed(() => {
    return hasPresetModels.value ? getChatModelOptions(selectedProvider.value) : []
  })

  const selectProvider = (providerId: string) => {
    if (selectedProvider.value === providerId) {
      selectedProvider.value = ''
    } else {
      selectedProvider.value = providerId
      formData.provider = providerId as AIModelConfig['provider']
      const info = providerInfo.value
      if (info) {
        formData.apiUrl = info.defaultApiUrl
        const models = getChatModelOptions(providerId)
        if (models.length > 0) {
          formData.model = models[0].value
        } else {
          formData.model = ''
        }
      }
    }
    formData.apiKey = ''
    formData.useCustomBaseUrl = false
    errors.value = {}
  }

  const handleUseCustomBaseUrlChange = () => {
    if (formData.useCustomBaseUrl) {
      formData.apiUrl = ''
    } else {
      formData.apiUrl = providerInfo.value?.defaultApiUrl || ''
    }
    delete errors.value.apiUrl
  }

  const validateForm = () => {
    errors.value = {}
    if (!selectedProvider.value) errors.value.provider = t('ai_model.validation.preset_required')
    if (!formData.apiKey.trim()) errors.value.apiKey = t('ai_model.validation.api_key_required')

    if (hasPresetModels.value) {
      if (!formData.model) errors.value.model = t('ai_model.validation.model_required')
      if (formData.useCustomBaseUrl && !formData.apiUrl) {
        errors.value.apiUrl = t('ai_model.validation.custom_base_url_required')
      }
    } else {
      if (!formData.apiUrl) errors.value.apiUrl = t('ai_model.validation.api_url_required')
      if (!formData.model) errors.value.model = t('ai_model.validation.model_name_required')
    }

    return Object.keys(errors.value).length === 0
  }

  const getDefaultApiUrl = () => {
    return formData.apiUrl
  }

  const handleSaveConfig = async (): Promise<boolean> => {
    if (!selectedProvider.value) return false

    if (!validateForm()) return false

    isSubmitting.value = true

    const newModel: AIModelConfig = {
      id: uuidv4(),
      provider: formData.provider,
      apiUrl: getDefaultApiUrl(),
      apiKey: formData.apiKey,
      model: formData.model,
      modelType: 'chat',
      options: formData.options,
      useCustomBaseUrl: formData.useCustomBaseUrl,
    }

    await aiSettingsStore.addModel(newModel)

    selectedProvider.value = ''
    Object.assign(formData, {
      provider: 'anthropic' as AIModelConfig['provider'],
      apiKey: '',
      apiUrl: '',
      model: '',
      options: {
        maxContextTokens: 128000,
        temperature: 0.7,
        timeout: 300000,
        contextWindow: 128000,
        maxTokens: -1,
      },
      useCustomBaseUrl: false,
    })
    errors.value = {}
    isSubmitting.value = false
    return true
  }

  const handleSkip = () => {
    return true
  }

  defineExpose({
    selectedProvider,
    handleSaveConfig,
    handleSkip,
  })
</script>

<style scoped>
  .ai-step {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 100%;
    max-width: 500px;
    height: 100%;
    overflow: hidden;
  }

  .step-header {
    text-align: center;
    margin-bottom: 40px;
    flex-shrink: 0;
    overflow: hidden;
  }

  .step-title {
    font-size: 32px;
    font-weight: 600;
    color: var(--text-100);
    margin: 0 0 12px 0;
  }

  .step-description {
    font-size: 16px;
    color: var(--text-400);
    margin: 0;
    line-height: 1.5;
  }

  .ai-options {
    width: 100%;
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .provider-container {
    display: flex;
    flex-direction: column;
    gap: 16px;
    flex: 1;
    overflow: hidden;
  }

  .provider-container::-webkit-scrollbar {
    width: 6px;
  }

  .provider-container::-webkit-scrollbar-track {
    background: transparent;
    border-radius: var(--border-radius-xs);
  }

  .provider-container::-webkit-scrollbar-thumb {
    background: var(--border-200);
    border-radius: var(--border-radius-xs);
  }

  .provider-container::-webkit-scrollbar-thumb:hover {
    background: var(--border-300);
  }

  .ai-option {
    background: var(--bg-200);
    border: 2px solid var(--border-100);
    border-radius: var(--border-radius-xl);
    cursor: pointer;
    transition:
      border-color 0.15s cubic-bezier(0.4, 0, 0.2, 1),
      background-color 0.15s cubic-bezier(0.4, 0, 0.2, 1);
    position: relative;
    overflow: hidden;
    will-change: border-color, background-color;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    max-height: 100%;
  }

  .ai-option-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px;
    transition: background-color 0.15s cubic-bezier(0.4, 0, 0.2, 1);
    will-change: background-color;
    flex-shrink: 0;
  }

  .ai-option:not(.selected):hover .ai-option-header {
    background: var(--bg-300);
  }

  .ai-option:not(.selected):hover {
    border-color: var(--color-primary);
  }

  .ai-option.selected {
    border-color: var(--color-primary);
  }

  .ai-option.selected .ai-option-header {
    background: var(--bg-300);
  }

  .ai-info {
    flex: 1;
    text-align: left;
  }

  .ai-name {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-200);
    margin: 0 0 4px 0;
  }

  .ai-config-dropdown {
    padding: 0 20px 24px 20px;
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  .ai-config-dropdown::-webkit-scrollbar {
    width: 6px;
  }

  .ai-config-dropdown::-webkit-scrollbar-track {
    background: var(--bg-300);
    border-radius: var(--border-radius-xs);
  }

  .ai-config-dropdown::-webkit-scrollbar-thumb {
    background: var(--border-200);
    border-radius: var(--border-radius-xs);
  }

  .ai-config-dropdown::-webkit-scrollbar-thumb:hover {
    background: var(--border-300);
  }

  .config-divider {
    height: 1px;
    background: var(--border-100);
    margin: 0 0 20px 0;
  }

  .form-group {
    margin-bottom: 20px;
  }

  .form-label {
    display: block;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-200);
    margin-bottom: 8px;
  }

  .form-input {
    width: 100%;
    padding: 12px 16px;
    font-size: 14px;
    color: var(--text-100);
    background: var(--bg-200);
    border: 2px solid var(--border-100);
    border-radius: var(--border-radius-lg);
  }

  .form-input:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  .form-input::placeholder {
    color: var(--text-400);
  }

  .form-input.error {
    border-color: var(--color-danger, #ef4444);
  }

  .error-message {
    font-size: 12px;
    color: var(--color-danger, #ef4444);
    margin-top: 4px;
  }

  .form-description {
    font-size: 12px;
    color: var(--text-400);
    margin-top: 4px;
    line-height: 1.4;
  }

  .form-group :deep(.x-select) {
    width: 100%;
  }

  .form-group :deep(.x-select__input-wrapper) {
    width: 100%;
  }

  .form-group :deep(.x-select__input) {
    width: 100%;
    padding: 12px 16px;
    font-size: 14px;
    color: var(--text-100);
    background: var(--bg-200);
    border: 2px solid var(--border-100);
    border-radius: var(--border-radius-lg);

    min-height: auto;
  }

  .form-group :deep(.x-select__input:focus-within) {
    border-color: var(--color-primary);
  }

  .form-group :deep(.x-select.error .x-select__input) {
    border-color: var(--color-danger, #ef4444);
  }

  .form-group :deep(.x-select__placeholder) {
    color: var(--text-400);
  }

  .form-group :deep(.x-select__value) {
    color: var(--text-100);
  }

  .provider-list-enter-active {
    transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .provider-list-leave-active {
    transition: all 0.2s cubic-bezier(0.4, 0, 1, 1);
  }

  .provider-list-enter-from {
    opacity: 0;
    transform: translateY(-15px) scale(0.95);
  }

  .provider-list-leave-to {
    opacity: 0;
    transform: translateY(-15px) scale(0.95);
  }

  .provider-list-move {
    transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .dropdown-enter-active {
    transition:
      max-height 0.3s cubic-bezier(0.4, 0, 0.2, 1) 0.1s,
      opacity 0.2s cubic-bezier(0.4, 0, 0.2, 1) 0.15s;
    will-change: max-height, opacity;
    overflow: hidden;
  }

  .dropdown-leave-active {
    transition:
      max-height 0.25s cubic-bezier(0.4, 0, 1, 1),
      opacity 0.15s cubic-bezier(0.4, 0, 1, 1);
    will-change: max-height, opacity;
    overflow: hidden;
  }

  .dropdown-enter-from {
    opacity: 0;
    max-height: 0;
  }

  .dropdown-leave-to {
    opacity: 0;
    max-height: 0;
  }

  .dropdown-enter-to,
  .dropdown-leave-from {
    opacity: 1;
    max-height: 350px;
  }

  .ai-option.expanded {
    border-color: var(--color-primary);
    background: var(--bg-250);
  }

  .ai-option.move-to-top {
    order: -1;
    will-change: transform, order;
    transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .ai-option:not(.move-to-top) {
    transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    user-select: none;
  }

  .checkbox-label span {
    color: var(--text-200);
    font-size: 14px;
  }

  .form-checkbox {
    width: 18px;
    height: 18px;
    cursor: pointer;
    accent-color: var(--color-primary);
  }

  .form-checkbox:focus {
    outline: 2px solid var(--color-primary-alpha);
    outline-offset: 2px;
  }

  .checkbox-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 4px 0;
  }

  .advanced-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 0;
    background: none;
    border: none;
    color: var(--text-300);
    font-size: 14px;
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

  .fade-slide-enter-active,
  .fade-slide-leave-active {
    transition:
      opacity 0.25s cubic-bezier(0.4, 0, 0.2, 1),
      max-height 0.3s cubic-bezier(0.4, 0, 0.2, 1),
      margin 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .fade-slide-enter-from,
  .fade-slide-leave-to {
    opacity: 0;
    max-height: 0;
    margin-bottom: 0;
  }

  .fade-slide-enter-to,
  .fade-slide-leave-from {
    opacity: 1;
    max-height: 200px;
    margin-bottom: 40px;
  }
</style>
