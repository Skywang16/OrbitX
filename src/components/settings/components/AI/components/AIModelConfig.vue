<script setup lang="ts">
  import type { AIModelConfig, AIProvider } from '@/types'
  import type { AIModelTestConnectionInput, AIModelCreateInput } from '@/api/ai/types'

  import { aiApi } from '@/api'
  import { computed, onMounted, ref, reactive, nextTick } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useAISettingsStore } from '../store'
  import { useLLMRegistry } from '@/composables/useLLMRegistry'
  import SettingsCard from '../../../SettingsCard.vue'

  const { t } = useI18n()

  const aiSettingsStore = useAISettingsStore()
  const { providers, providerOptions, getChatModelOptions, loadProviders } = useLLMRegistry()

  // State
  const models = computed(() => aiSettingsStore.chatModels)
  const loading = computed(() => aiSettingsStore.isLoading)

  // Inline editing state
  const editingId = ref<string | null>(null)
  const isAdding = ref(false)
  const isTesting = ref(false)
  const isSaving = ref(false)
  const showAdvancedOptions = ref(false)

  // Form data for inline editing
  const formData = reactive({
    provider: '' as AIProvider | '',
    apiUrl: '',
    apiKey: '',
    model: '',
    modelType: 'chat' as const,
    options: {
      maxContextTokens: 128000,
      temperature: 0.7,
      timeout: 300000,
      maxTokens: -1,
    },
    useCustomBaseUrl: false,
  })

  // Input refs for focus management
  const providerInputRef = ref<HTMLSelectElement>()
  const apiKeyInputRef = ref<HTMLInputElement>()

  onMounted(async () => {
    await Promise.all([aiSettingsStore.loadModels(), loadProviders()])
  })

  // Provider info
  const providerInfo = computed(() => {
    return providers.value.find(p => p.providerType.toLowerCase() === formData.provider.toLowerCase())
  })

  const hasPresetModels = computed(() => {
    return providerInfo.value?.presetModels && providerInfo.value.presetModels.length > 0
  })

  const availableModels = computed(() => (hasPresetModels.value ? getChatModelOptions(formData.provider) : []))

  // Reset form
  const resetForm = () => {
    formData.provider = '' as AIProvider | ''
    formData.apiUrl = ''
    formData.apiKey = ''
    formData.model = ''
    formData.useCustomBaseUrl = false
    formData.options = {
      maxContextTokens: 128000,
      temperature: 0.7,
      timeout: 300000,
      maxTokens: -1,
    }
    showAdvancedOptions.value = false
  }

  // Start adding new model
  const startAdding = () => {
    editingId.value = null
    isAdding.value = true
    resetForm()
    nextTick(() => {
      providerInputRef.value?.focus()
    })
  }

  // Cancel editing/adding
  const cancelEdit = () => {
    editingId.value = null
    isAdding.value = false
    resetForm()
  }

  // Start editing existing model
  const startEditing = (model: AIModelConfig) => {
    isAdding.value = false
    editingId.value = model.id
    formData.provider = model.provider
    formData.apiUrl = model.apiUrl
    formData.apiKey = model.apiKey
    formData.model = model.model
    formData.useCustomBaseUrl = model.useCustomBaseUrl || false
    formData.options = {
      maxContextTokens: model.options?.maxContextTokens ?? 128000,
      temperature: model.options?.temperature ?? 0.7,
      timeout: model.options?.timeout ?? 300000,
      maxTokens: model.options?.maxTokens ?? -1,
    }
    showAdvancedOptions.value = false
    nextTick(() => {
      apiKeyInputRef.value?.focus()
    })
  }

  // Handle provider change
  const handleProviderChange = (value: AIProvider | string) => {
    formData.provider = value as AIProvider
    const info = providerInfo.value
    if (!info) return

    formData.apiUrl = info.defaultApiUrl
    const models = getChatModelOptions(value)
    formData.model = models.length > 0 ? models[0].value : ''
    formData.useCustomBaseUrl = false
  }

  // Handle custom base URL toggle
  const handleCustomUrlToggle = () => {
    if (formData.useCustomBaseUrl) {
      formData.apiUrl = ''
    } else {
      formData.apiUrl = providerInfo.value?.defaultApiUrl || ''
    }
  }

  // Test connection
  const testConnection = async () => {
    if (!formData.provider || !formData.apiKey || !formData.model) return

    isTesting.value = true
    try {
      const testConfig: AIModelTestConnectionInput = {
        provider: formData.provider as AIProvider,
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

  // Save model
  const saveModel = async () => {
    if (!formData.provider || !formData.apiKey || !formData.model) return

    isSaving.value = true
    try {
      if (editingId.value) {
        await aiSettingsStore.updateModel(editingId.value, {
          provider: formData.provider as AIProvider,
          apiUrl: formData.apiUrl,
          apiKey: formData.apiKey,
          model: formData.model,
          modelType: formData.modelType,
          options: formData.options,
          useCustomBaseUrl: formData.useCustomBaseUrl,
        })
      } else {
        const createInput: AIModelCreateInput = {
          provider: formData.provider as AIProvider,
          apiUrl: formData.apiUrl,
          apiKey: formData.apiKey,
          model: formData.model,
          modelType: 'chat',
          options: formData.options,
          useCustomBaseUrl: formData.useCustomBaseUrl,
        }
        await aiSettingsStore.addModel(createInput)
      }
      cancelEdit()
    } finally {
      isSaving.value = false
    }
  }

  // Delete model
  const deleteModel = async (modelId: string) => {
    await aiSettingsStore.removeModel(modelId)
  }

  // Check if form is valid
  const isFormValid = computed(() => {
    return formData.provider && formData.apiKey.trim() && formData.model
  })
</script>

<template>
  <div class="settings-group">
    <h3 class="settings-group-title">{{ t('settings.ai.model_config') }}</h3>

    <!-- Add button -->
    <SettingsCard v-if="!isAdding && !editingId">
      <div class="settings-item clickable" @click="startAdding">
        <div class="settings-item-header">
          <div class="settings-label">{{ t('ai_model.add_new_model') }}</div>
          <div class="settings-description">{{ t('ai_model.add_model_description') }}</div>
        </div>
        <div class="settings-item-control">
          <x-button variant="primary" size="small">
            {{ t('ai_model.add_chat_model') }}
          </x-button>
        </div>
      </div>
    </SettingsCard>

    <!-- Loading state -->
    <div v-if="loading" class="settings-loading">
      <div class="settings-loading-spinner"></div>
      <span>{{ t('ai_model.loading') }}</span>
    </div>

    <!-- Add new model form -->
    <SettingsCard v-else-if="isAdding">
      <div class="inline-form">
        <!-- Provider select -->
        <div class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">{{ t('ai_model.provider') }}</div>
          </div>
          <div class="settings-item-control">
            <x-select
              ref="providerInputRef"
              v-model="formData.provider"
              :options="providerOptions.map(p => ({ value: p.value, label: p.label }))"
              :placeholder="t('ai_model.select_provider')"
              @update:modelValue="handleProviderChange"
            />
          </div>
        </div>

        <!-- Model select (if provider has presets) -->
        <div v-if="hasPresetModels" class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">{{ t('ai_model.model') }}</div>
          </div>
          <div class="settings-item-control">
            <x-select v-model="formData.model" :options="availableModels" :placeholder="t('ai_model.select_model')" />
          </div>
        </div>

        <!-- Model name input (if no presets) -->
        <div v-else-if="formData.provider" class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">{{ t('ai_model.model_name') }}</div>
          </div>
          <div class="settings-item-control">
            <input
              v-model="formData.model"
              type="text"
              class="settings-input"
              :placeholder="t('ai_model.model_name_placeholder')"
            />
          </div>
        </div>

        <!-- Custom base URL toggle -->
        <div v-if="hasPresetModels" class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">{{ t('ai_model.use_custom_base_url') }}</div>
          </div>
          <div class="settings-item-control">
            <x-switch
              :modelValue="formData.useCustomBaseUrl"
              @update:modelValue="
                (val: boolean) => {
                  formData.useCustomBaseUrl = val
                  handleCustomUrlToggle()
                }
              "
            />
          </div>
        </div>

        <!-- Custom base URL input -->
        <div v-if="formData.useCustomBaseUrl || !hasPresetModels" class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">
              {{ hasPresetModels ? t('ai_model.custom_base_url') : t('ai_model.api_url') }}
            </div>
          </div>
          <div class="settings-item-control">
            <input
              v-model="formData.apiUrl"
              type="url"
              class="settings-input mono"
              :placeholder="t('ai_model.api_url_placeholder')"
            />
          </div>
        </div>

        <!-- API Key -->
        <div v-if="formData.provider" class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">{{ t('ai_model.api_key') }}</div>
          </div>
          <div class="settings-item-control">
            <input
              ref="apiKeyInputRef"
              v-model="formData.apiKey"
              type="password"
              class="settings-input mono"
              :placeholder="t('ai_model.api_key_placeholder')"
            />
          </div>
        </div>

        <!-- Context Window -->
        <div v-if="formData.provider" class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">{{ t('ai_model.context_window') }}</div>
            <div class="settings-description">{{ t('ai_model.context_window_description') }}</div>
          </div>
          <div class="settings-item-control">
            <input
              v-model.number="formData.options.maxContextTokens"
              type="number"
              class="settings-input mono"
              placeholder="128000"
              min="1000"
              max="2000000"
            />
          </div>
        </div>

        <!-- Advanced Options Toggle -->
        <div
          v-if="formData.provider"
          class="settings-item clickable"
          @click="showAdvancedOptions = !showAdvancedOptions"
        >
          <div class="settings-item-header">
            <div class="settings-label">
              <svg
                class="toggle-icon"
                :class="{ expanded: showAdvancedOptions }"
                width="12"
                height="12"
                viewBox="0 0 12 12"
                fill="none"
              >
                <path d="M4.5 3L7.5 6L4.5 9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
              </svg>
              {{ t('ai_model.advanced_options') }}
            </div>
          </div>
        </div>

        <!-- Advanced Options Content -->
        <template v-if="showAdvancedOptions && formData.provider">
          <!-- Max Output Tokens -->
          <div class="settings-item">
            <div class="settings-item-header">
              <div class="settings-label">{{ t('ai_model.max_output_tokens') }}</div>
              <div class="settings-description">{{ t('ai_model.max_output_tokens_description') }}</div>
            </div>
            <div class="settings-item-control">
              <input
                v-model.number="formData.options.maxTokens"
                type="number"
                class="settings-input mono"
                placeholder="-1"
                min="-1"
                max="200000"
              />
            </div>
          </div>
        </template>

        <!-- Actions -->
        <div class="form-actions">
          <x-button variant="secondary" size="small" @click="cancelEdit">
            {{ t('common.cancel') }}
          </x-button>
          <x-button
            variant="secondary"
            size="small"
            :loading="isTesting"
            :disabled="!isFormValid"
            @click="testConnection"
          >
            {{ isTesting ? t('ai_model.testing') : t('ai_model.test_connection') }}
          </x-button>
          <x-button variant="primary" size="small" :loading="isSaving" :disabled="!isFormValid" @click="saveModel">
            {{ t('common.add') }}
          </x-button>
        </div>
      </div>
    </SettingsCard>

    <!-- Model list -->
    <template v-else>
      <SettingsCard v-if="models.length === 0">
        <div class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">{{ t('ai_model.no_models') }}</div>
            <div class="settings-description">{{ t('ai_model_config.empty_description') }}</div>
          </div>
        </div>
      </SettingsCard>

      <SettingsCard v-else>
        <template v-for="model in models" :key="model.id">
          <!-- Editing mode -->
          <div v-if="editingId === model.id" class="inline-form inline-form--nested">
            <!-- Provider select -->
            <div class="settings-item">
              <div class="settings-item-header">
                <div class="settings-label">{{ t('ai_model.provider') }}</div>
              </div>
              <div class="settings-item-control">
                <x-select
                  v-model="formData.provider"
                  :options="providerOptions.map(p => ({ value: p.value, label: p.label }))"
                  :placeholder="t('ai_model.select_provider')"
                  @update:modelValue="handleProviderChange"
                />
              </div>
            </div>

            <!-- Model select (if provider has presets) -->
            <div v-if="hasPresetModels" class="settings-item">
              <div class="settings-item-header">
                <div class="settings-label">{{ t('ai_model.model') }}</div>
              </div>
              <div class="settings-item-control">
                <x-select
                  v-model="formData.model"
                  :options="availableModels"
                  :placeholder="t('ai_model.select_model')"
                />
              </div>
            </div>

            <!-- Model name input (if no presets) -->
            <div v-else-if="formData.provider" class="settings-item">
              <div class="settings-item-header">
                <div class="settings-label">{{ t('ai_model.model_name') }}</div>
              </div>
              <div class="settings-item-control">
                <input
                  v-model="formData.model"
                  type="text"
                  class="settings-input"
                  :placeholder="t('ai_model.model_name_placeholder')"
                />
              </div>
            </div>

            <!-- Custom base URL toggle -->
            <div v-if="hasPresetModels" class="settings-item">
              <div class="settings-item-header">
                <div class="settings-label">{{ t('ai_model.use_custom_base_url') }}</div>
              </div>
              <div class="settings-item-control">
                <x-switch
                  :modelValue="formData.useCustomBaseUrl"
                  @update:modelValue="
                    (val: boolean) => {
                      formData.useCustomBaseUrl = val
                      handleCustomUrlToggle()
                    }
                  "
                />
              </div>
            </div>

            <!-- Custom base URL input -->
            <div v-if="formData.useCustomBaseUrl || !hasPresetModels" class="settings-item">
              <div class="settings-item-header">
                <div class="settings-label">
                  {{ hasPresetModels ? t('ai_model.custom_base_url') : t('ai_model.api_url') }}
                </div>
              </div>
              <div class="settings-item-control">
                <input
                  v-model="formData.apiUrl"
                  type="url"
                  class="settings-input mono"
                  :placeholder="t('ai_model.api_url_placeholder')"
                />
              </div>
            </div>

            <!-- API Key -->
            <div class="settings-item">
              <div class="settings-item-header">
                <div class="settings-label">{{ t('ai_model.api_key') }}</div>
              </div>
              <div class="settings-item-control">
                <input
                  ref="apiKeyInputRef"
                  v-model="formData.apiKey"
                  type="password"
                  class="settings-input mono"
                  :placeholder="t('ai_model.api_key_placeholder')"
                />
              </div>
            </div>

            <!-- Context Window -->
            <div class="settings-item">
              <div class="settings-item-header">
                <div class="settings-label">{{ t('ai_model.context_window') }}</div>
                <div class="settings-description">{{ t('ai_model.context_window_description') }}</div>
              </div>
              <div class="settings-item-control">
                <input
                  v-model.number="formData.options.maxContextTokens"
                  type="number"
                  class="settings-input mono"
                  placeholder="128000"
                  min="1000"
                  max="2000000"
                />
              </div>
            </div>

            <!-- Advanced Options Toggle -->
            <div class="settings-item clickable" @click="showAdvancedOptions = !showAdvancedOptions">
              <div class="settings-item-header">
                <div class="settings-label">
                  <svg
                    class="toggle-icon"
                    :class="{ expanded: showAdvancedOptions }"
                    width="12"
                    height="12"
                    viewBox="0 0 12 12"
                    fill="none"
                  >
                    <path d="M4.5 3L7.5 6L4.5 9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
                  </svg>
                  {{ t('ai_model.advanced_options') }}
                </div>
              </div>
            </div>

            <!-- Advanced Options Content -->
            <template v-if="showAdvancedOptions">
              <!-- Max Output Tokens -->
              <div class="settings-item">
                <div class="settings-item-header">
                  <div class="settings-label">{{ t('ai_model.max_output_tokens') }}</div>
                  <div class="settings-description">{{ t('ai_model.max_output_tokens_description') }}</div>
                </div>
                <div class="settings-item-control">
                  <input
                    v-model.number="formData.options.maxTokens"
                    type="number"
                    class="settings-input mono"
                    placeholder="-1"
                    min="-1"
                    max="200000"
                  />
                </div>
              </div>
            </template>

            <!-- Actions -->
            <div class="form-actions">
              <x-button variant="secondary" size="small" @click="cancelEdit">
                {{ t('common.cancel') }}
              </x-button>
              <x-button
                variant="secondary"
                size="small"
                :loading="isTesting"
                :disabled="!isFormValid"
                @click="testConnection"
              >
                {{ isTesting ? t('ai_model.testing') : t('ai_model.test_connection') }}
              </x-button>
              <x-button variant="primary" size="small" :loading="isSaving" :disabled="!isFormValid" @click="saveModel">
                {{ t('common.save') }}
              </x-button>
            </div>
          </div>

          <!-- View mode -->
          <div v-else class="settings-item">
            <div class="settings-item-header">
              <div class="settings-label mono">{{ model.model }}</div>
              <div class="settings-description">
                {{ model.provider }}
                <span v-if="model.useCustomBaseUrl" class="tag">custom url</span>
              </div>
            </div>
            <div class="settings-item-control">
              <x-button variant="secondary" size="small" @click="startEditing(model)">
                {{ t('ai_model.edit') }}
              </x-button>
              <x-button variant="danger" size="small" @click="deleteModel(model.id)">
                {{ t('ai_model.delete') }}
              </x-button>
            </div>
          </div>
        </template>
      </SettingsCard>
    </template>
  </div>
</template>

<style scoped>
  .inline-form {
    display: flex;
    flex-direction: column;
  }

  .inline-form--nested {
    padding: 0;
  }

  /* 覆盖全局样式：内联表单中所有 settings-item 都不显示分隔线 */
  .inline-form :deep(.settings-item)::after,
  .inline-form--nested :deep(.settings-item)::after {
    display: none !important;
  }

  .form-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px 20px;
    border-top: 1px solid var(--border-200);
  }

  .settings-input {
    width: 100%;
    height: 32px;
    padding: 0 10px;
    font-size: 13px;
    color: var(--text-100);
    background: var(--bg-400);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius-sm);
    outline: none;
    transition: border-color 0.15s ease;
  }

  .settings-input:focus {
    border-color: var(--color-primary);
  }

  .settings-input.mono,
  .settings-value.mono,
  .settings-label.mono {
    font-family: var(--font-family-mono);
  }

  .settings-value {
    font-size: 13px;
    color: var(--text-200);
  }

  .tag {
    display: inline-block;
    font-size: 10px;
    color: var(--text-400);
    background: var(--bg-500);
    padding: 1px 6px;
    border-radius: var(--border-radius-xs);
    margin-left: 8px;
  }

  .toggle-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    margin-right: 6px;
    transition: transform 0.2s ease;
    color: var(--text-400);
  }

  .toggle-icon.expanded {
    transform: rotate(90deg);
  }
</style>
