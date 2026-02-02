<script setup lang="ts">
  import type { AIModelConfig, AIProvider } from '@/types'
  import type { AIModelTestConnectionInput, AIModelCreateInput } from '@/api/ai/types'
  import { AuthType, OAuthProvider, type OAuthConfig } from '@/types/oauth'

  import { aiApi } from '@/api'
  import { computed, onMounted, ref, reactive } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useAISettingsStore } from '../store'
  import { useLLMRegistry } from '@/composables/useLLMRegistry'
  import SettingsCard from '../../../SettingsCard.vue'
  import OAuthAuthorizationDialog from '../../../OAuthAuthorizationDialog.vue'

  const { t } = useI18n()
  const aiSettingsStore = useAISettingsStore()
  const { providers, providerOptions, getChatModelOptions, loadProviders } = useLLMRegistry()

  const models = computed(() => aiSettingsStore.chatModels)
  const loading = computed(() => aiSettingsStore.isLoading)

  const editingId = ref<string | null>(null)
  const isTesting = ref(false)
  const isSaving = ref(false)
  const showAdvancedOptions = ref(false)
  const showOAuthDialog = ref(false)
  const showAddForm = ref(false)

  const formData = reactive({
    authType: 'apikey' as 'apikey' | 'oauth',
    provider: '' as string,
    apiUrl: '',
    apiKey: '',
    model: '',
    useCustomBaseUrl: false,
    oauthProvider: '' as string,
    oauthConfig: undefined as OAuthConfig | undefined,
    options: { maxContextTokens: 128000, temperature: 0.5, timeout: 300000, maxTokens: -1 },
  })


  const availableModels = computed(() => {
    if (formData.authType === 'oauth') {
      return [
        { value: 'gpt-4o', label: 'GPT-4o' },
        { value: 'gpt-4o-mini', label: 'GPT-4o Mini' },
        { value: 'o1', label: 'o1' },
        { value: 'o1-mini', label: 'o1 Mini' },
      ]
    }
    return hasPresetModels.value ? getChatModelOptions(formData.provider) : []
  })

  const providerInfo = computed(() => {
    if (formData.authType === 'oauth') return null
    return providers.value.find(p => p.providerType.toLowerCase() === formData.provider.toLowerCase())
  })

  const hasPresetModels = computed(() => providerInfo.value?.presetModels && providerInfo.value.presetModels.length > 0)

  onMounted(async () => {
    await Promise.all([aiSettingsStore.loadModels(), loadProviders()])
  })

  const resetForm = () => {
    formData.authType = 'apikey'
    formData.provider = ''
    formData.apiUrl = ''
    formData.apiKey = ''
    formData.model = ''
    formData.useCustomBaseUrl = false
    formData.oauthProvider = ''
    formData.oauthConfig = undefined
    formData.options = { maxContextTokens: 128000, temperature: 0.7, timeout: 300000, maxTokens: -1 }
    showAdvancedOptions.value = false
    editingId.value = null
    showAddForm.value = false
  }

  const startAdding = () => {
    resetForm()
    showAddForm.value = true
  }

  const startEditing = (model: AIModelConfig) => {
    editingId.value = model.id
    showAddForm.value = true
    if (model.authType === AuthType.OAuth) {
      formData.authType = 'oauth'
      formData.oauthProvider = model.oauthConfig?.provider || OAuthProvider.OpenAiCodex
      formData.oauthConfig = model.oauthConfig
      formData.model = model.model
      formData.provider = ''
    } else {
      formData.authType = 'apikey'
      formData.provider = model.provider
      formData.apiUrl = model.apiUrl || ''
      formData.apiKey = model.apiKey || ''
      formData.model = model.model
      formData.useCustomBaseUrl = model.useCustomBaseUrl || false
      formData.oauthProvider = ''
    }
    formData.options = {
      maxContextTokens: model.options?.maxContextTokens ?? 128000,
      temperature: model.options?.temperature ?? 0.5,
      timeout: model.options?.timeout ?? 300000,
      maxTokens: model.options?.maxTokens ?? -1,
    }
  }

  const switchAuthType = (type: 'apikey' | 'oauth') => {
    formData.authType = type
    formData.model = ''
    if (type === 'oauth') {
      formData.provider = ''
      formData.apiKey = ''
      formData.apiUrl = ''
      formData.oauthProvider = OAuthProvider.OpenAiCodex
      formData.model = 'gpt-4o'
    } else {
      formData.oauthConfig = undefined
      formData.oauthProvider = ''
    }
  }

  const handleProviderChange = (value: string) => {
    formData.provider = value
    const info = providerInfo.value
    if (info) {
      formData.apiUrl = info.defaultApiUrl
      const models = getChatModelOptions(value)
      formData.model = models.length > 0 ? models[0].value : ''
    }
    formData.useCustomBaseUrl = false
  }

  const handleCustomUrlToggle = () => {
    formData.apiUrl = formData.useCustomBaseUrl ? '' : providerInfo.value?.defaultApiUrl || ''
  }

  const handleOAuthSuccess = (config: OAuthConfig) => {
    formData.oauthConfig = config
    showOAuthDialog.value = false
  }

  const testConnection = async () => {
    if (formData.authType === 'oauth' || !formData.provider || !formData.model || !formData.apiKey) return
    isTesting.value = true
    try {
      await aiApi.testConnectionWithConfig({
        provider: formData.provider as AIProvider,
        authType: AuthType.ApiKey,
        apiUrl: formData.apiUrl,
        apiKey: formData.apiKey,
        model: formData.model,
        modelType: 'chat',
        options: formData.options,
      } as AIModelTestConnectionInput)
    } finally {
      isTesting.value = false
    }
  }

  const saveModel = async () => {
    if (formData.authType === 'oauth') {
      if (!formData.oauthConfig || !formData.model || !formData.oauthProvider) return
    } else {
      if (!formData.provider || !formData.model || !formData.apiKey) return
    }

    isSaving.value = true
    try {
      const modelData =
        formData.authType === 'oauth'
          ? {
              provider: 'openai_compatible' as AIProvider,
              authType: AuthType.OAuth,
              apiUrl: '',
              apiKey: '',
              model: formData.model,
              modelType: 'chat' as const,
              options: formData.options,
              oauthConfig: formData.oauthConfig,
              useCustomBaseUrl: false,
            }
          : {
              provider: formData.provider as AIProvider,
              authType: AuthType.ApiKey,
              apiUrl: formData.apiUrl,
              apiKey: formData.apiKey,
              model: formData.model,
              modelType: 'chat' as const,
              options: formData.options,
              oauthConfig: undefined,
              useCustomBaseUrl: formData.useCustomBaseUrl,
            }

      if (editingId.value) {
        await aiSettingsStore.updateModel(editingId.value, modelData)
      } else {
        await aiSettingsStore.addModel(modelData as AIModelCreateInput)
      }
      resetForm()
    } finally {
      isSaving.value = false
    }
  }

  const deleteModel = async (modelId: string) => {
    await aiSettingsStore.removeModel(modelId)
  }

  const isFormValid = computed(() => {
    if (formData.authType === 'oauth') {
      return !!formData.oauthConfig && !!formData.model && !!formData.oauthProvider
    }
    return !!formData.provider && !!formData.model && !!formData.apiKey?.trim()
  })
</script>

<template>
  <div class="ai-model-config">
    <!-- Loading State -->
    <div v-if="loading" class="loading-state">
      <div class="loading-spinner"></div>
      <span>{{ t('ai_model.loading') }}</span>
    </div>

    <template v-else>
      <!-- Configured Models List -->
      <div class="settings-section">
        <div class="section-header">
          <h3 class="settings-section-title">{{ t('ai_model.configured_models') || 'Configured Models' }}</h3>
          <button v-if="!showAddForm && models.length > 0" class="add-btn" @click="startAdding">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="12" y1="5" x2="12" y2="19" />
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
            {{ t('ai_model.add_model') || 'Add Model' }}
          </button>
        </div>

        <!-- Empty State -->
        <div v-if="models.length === 0 && !showAddForm" class="empty-state">
          <div class="empty-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path
                d="M9.5 2A2.5 2.5 0 0 1 12 4.5v15a2.5 2.5 0 0 1-4.96.44 2.5 2.5 0 0 1-2.96-3.08 3 3 0 0 1-.34-5.58 2.5 2.5 0 0 1 1.32-4.24 2.5 2.5 0 0 1 1.98-3A2.5 2.5 0 0 1 9.5 2Z"
              />
              <path
                d="M14.5 2A2.5 2.5 0 0 0 12 4.5v15a2.5 2.5 0 0 0 4.96.44 2.5 2.5 0 0 0 2.96-3.08 3 3 0 0 0 .34-5.58 2.5 2.5 0 0 0-1.32-4.24 2.5 2.5 0 0 0-1.98-3A2.5 2.5 0 0 0 14.5 2Z"
              />
            </svg>
          </div>
          <div class="empty-text">
            <h4>{{ t('ai_model.no_models') || 'No AI models configured' }}</h4>
            <p>{{ t('ai_model.no_models_description') || 'Add a model to start using AI features' }}</p>
          </div>
          <button class="primary-btn" @click="startAdding">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="12" y1="5" x2="12" y2="19" />
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
            {{ t('ai_model.add_first_model') || 'Add Your First Model' }}
          </button>
        </div>

        <!-- Models List -->
        <div v-if="models.length > 0" class="models-list">
          <div v-for="model in models" :key="model.id" class="model-card">
            <div class="model-main">
              <div class="model-icon">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path
                    d="M9.5 2A2.5 2.5 0 0 1 12 4.5v15a2.5 2.5 0 0 1-4.96.44 2.5 2.5 0 0 1-2.96-3.08 3 3 0 0 1-.34-5.58 2.5 2.5 0 0 1 1.32-4.24 2.5 2.5 0 0 1 1.98-3A2.5 2.5 0 0 1 9.5 2Z"
                  />
                  <path
                    d="M14.5 2A2.5 2.5 0 0 0 12 4.5v15a2.5 2.5 0 0 0 4.96.44 2.5 2.5 0 0 0 2.96-3.08 3 3 0 0 0 .34-5.58 2.5 2.5 0 0 0-1.32-4.24 2.5 2.5 0 0 0-1.98-3A2.5 2.5 0 0 0 14.5 2Z"
                  />
                </svg>
              </div>
              <div class="model-info">
                <div class="model-name">{{ model.model }}</div>
                <div class="model-meta">
                  <span class="model-provider">{{ model.provider }}</span>
                  <span class="model-badge" :class="model.authType === AuthType.OAuth ? 'oauth' : 'apikey'">
                    {{ model.authType === AuthType.OAuth ? 'OAuth' : 'API Key' }}
                  </span>
                </div>
              </div>
            </div>
            <div class="model-actions">
              <button class="icon-btn" @click="startEditing(model)" :title="t('common.edit')">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
                  <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
                </svg>
              </button>
              <button class="icon-btn danger" @click="deleteModel(model.id)" :title="t('common.delete')">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="3 6 5 6 21 6" />
                  <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                </svg>
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Add/Edit Form -->
      <div v-if="showAddForm" class="settings-section">
        <h3 class="settings-section-title">
          {{ editingId ? t('ai_model.edit_model') || 'Edit Model' : t('ai_model.add_model') || 'Add Model' }}
        </h3>

        <SettingsCard>
          <!-- Auth Type Tabs -->
          <div class="auth-tabs">
            <button
              class="auth-tab"
              :class="{ active: formData.authType === 'apikey' }"
              @click="switchAuthType('apikey')"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path
                  d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"
                />
              </svg>
              API Key
            </button>
            <button
              class="auth-tab"
              :class="{ active: formData.authType === 'oauth' }"
              @click="switchAuthType('oauth')"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
                <circle cx="12" cy="7" r="4" />
              </svg>
              {{ t('ai_model.subscription') || 'Subscription' }}
            </button>
          </div>

          <!-- API Key Form -->
          <div v-if="formData.authType === 'apikey'" class="form-body">
            <div class="form-group">
              <label class="form-label">{{ t('ai_model.provider') }}</label>
              <x-select
                v-model="formData.provider"
                :options="providerOptions.map(p => ({ value: p.value, label: p.label }))"
                :placeholder="t('ai_model.select_provider')"
                @update:modelValue="handleProviderChange"
              />
            </div>

            <div v-if="formData.provider" class="form-group">
              <label class="form-label">{{ t('ai_model.api_key') }}</label>
              <input
                v-model="formData.apiKey"
                type="password"
                class="form-input mono"
                :placeholder="t('ai_model.api_key_placeholder')"
              />
            </div>

            <div v-if="formData.provider && availableModels.length > 0" class="form-group">
              <label class="form-label">{{ t('ai_model.model') }}</label>
              <x-select v-model="formData.model" :options="availableModels" :placeholder="t('ai_model.select_model')" />
            </div>

            <div v-else-if="formData.provider" class="form-group">
              <label class="form-label">{{ t('ai_model.model_name') }}</label>
              <input
                v-model="formData.model"
                type="text"
                class="form-input"
                :placeholder="t('ai_model.model_name_placeholder')"
              />
            </div>

            <div v-if="formData.provider && hasPresetModels" class="form-group inline">
              <label class="form-label">{{ t('ai_model.use_custom_base_url') }}</label>
              <x-switch
                :modelValue="formData.useCustomBaseUrl"
                @update:modelValue="
                  (v: boolean) => {
                    formData.useCustomBaseUrl = v
                    handleCustomUrlToggle()
                  }
                "
              />
            </div>

            <div v-if="formData.provider && (formData.useCustomBaseUrl || !hasPresetModels)" class="form-group">
              <label class="form-label">{{ t('ai_model.api_url') }}</label>
              <input
                v-model="formData.apiUrl"
                type="url"
                class="form-input mono"
                :placeholder="t('ai_model.api_url_placeholder')"
              />
            </div>
          </div>

          <!-- OAuth Form -->
          <div v-else class="form-body">
            <div class="oauth-info">
              <div class="oauth-provider-card">
                <div class="oauth-provider-icon">
                  <svg viewBox="0 0 24 24" fill="currentColor">
                    <path
                      d="M22.2819 9.8211a5.9847 5.9847 0 0 0-.5157-4.9108 6.0462 6.0462 0 0 0-6.5098-2.9A6.0651 6.0651 0 0 0 4.9807 4.1818a5.9847 5.9847 0 0 0-3.9977 2.9 6.0462 6.0462 0 0 0 .7427 7.0966 5.98 5.98 0 0 0 .511 4.9107 6.051 6.051 0 0 0 6.5146 2.9001A5.9847 5.9847 0 0 0 13.2599 24a6.0557 6.0557 0 0 0 5.7718-4.2058 5.9894 5.9894 0 0 0 3.9977-2.9001 6.0557 6.0557 0 0 0-.7475-7.0729zm-9.022 12.6081a4.4755 4.4755 0 0 1-2.8764-1.0408l.1419-.0804 4.7783-2.7582a.7948.7948 0 0 0 .3927-.6813v-6.7369l2.02 1.1686a.071.071 0 0 1 .038.052v5.5826a4.504 4.504 0 0 1-4.4945 4.4944zm-9.6607-4.1254a4.4708 4.4708 0 0 1-.5346-3.0137l.142.0852 4.783 2.7582a.7712.7712 0 0 0 .7806 0l5.8428-3.3685v2.3324a.0804.0804 0 0 1-.0332.0615L9.74 19.9502a4.4992 4.4992 0 0 1-6.1408-1.6464zM2.3408 7.8956a4.485 4.485 0 0 1 2.3655-1.9728V11.6a.7664.7664 0 0 0 .3879.6765l5.8144 3.3543-2.0201 1.1685a.0757.0757 0 0 1-.071 0l-4.8303-2.7865A4.504 4.504 0 0 1 2.3408 7.8956zm16.5963 3.8558L13.1038 8.364 15.1192 7.2a.0757.0757 0 0 1 .071 0l4.8303 2.7913a4.4944 4.4944 0 0 1-.6765 8.1042v-5.6772a.79.79 0 0 0-.407-.667zm2.0107-3.0231l-.142-.0852-4.7735-2.7818a.7759.7759 0 0 0-.7854 0L9.409 9.2297V6.8974a.0662.0662 0 0 1 .0284-.0615l4.8303-2.7866a4.4992 4.4992 0 0 1 6.6802 4.66zM8.3065 12.863l-2.02-1.1638a.0804.0804 0 0 1-.038-.0567V6.0742a4.4992 4.4992 0 0 1 7.3757-3.4537l-.142.0805L8.704 5.459a.7948.7948 0 0 0-.3927.6813zm1.0976-2.3654l2.602-1.4998 2.6069 1.4998v2.9994l-2.5974 1.4997-2.6067-1.4997Z"
                    />
                  </svg>
                </div>
                <div class="oauth-provider-info">
                  <div class="oauth-provider-name">ChatGPT Plus/Pro</div>
                  <div class="oauth-provider-desc">
                    {{ t('ai_model.oauth_description') || 'Use your ChatGPT subscription' }}
                  </div>
                </div>
              </div>

              <div class="oauth-status-row">
                <div class="oauth-status" :class="{ authorized: formData.oauthConfig }">
                  <svg
                    v-if="formData.oauthConfig"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2.5"
                    class="status-icon success"
                  >
                    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
                    <polyline points="22 4 12 14.01 9 11.01" />
                  </svg>
                  <svg
                    v-else
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    class="status-icon"
                  >
                    <circle cx="12" cy="12" r="10" />
                  </svg>
                  <span>
                    {{
                      formData.oauthConfig
                        ? t('ai_model.authorized') || 'Authorized'
                        : t('ai_model.not_authorized') || 'Not authorized'
                    }}
                  </span>
                </div>
                <button class="auth-btn" :class="{ secondary: formData.oauthConfig }" @click="showOAuthDialog = true">
                  {{
                    formData.oauthConfig
                      ? t('ai_model.reauthorize') || 'Re-authorize'
                      : t('ai_model.start_authorization') || 'Authorize'
                  }}
                </button>
              </div>
            </div>

            <div v-if="formData.oauthConfig" class="form-group">
              <label class="form-label">{{ t('ai_model.model') }}</label>
              <x-select v-model="formData.model" :options="availableModels" :placeholder="t('ai_model.select_model')" />
            </div>
          </div>

          <!-- Form Actions -->
          <div class="form-actions">
            <button class="cancel-btn" @click="resetForm">{{ t('common.cancel') }}</button>
            <button
              v-if="formData.authType === 'apikey'"
              class="secondary-btn"
              :disabled="!isFormValid || isTesting"
              @click="testConnection"
            >
              {{ isTesting ? t('ai_model.testing') : t('ai_model.test_connection') }}
            </button>
            <button class="primary-btn" :disabled="!isFormValid || isSaving" @click="saveModel">
              {{ editingId ? t('common.save') : t('common.add') }}
            </button>
          </div>
        </SettingsCard>
      </div>
    </template>

    <OAuthAuthorizationDialog
      :visible="showOAuthDialog"
      :initial-provider="(formData.oauthProvider as OAuthProvider) || OAuthProvider.OpenAiCodex"
      @update:visible="showOAuthDialog = $event"
      @success="handleOAuthSuccess"
      @cancel="showOAuthDialog = false"
    />
  </div>
</template>

<style scoped>
  .ai-model-config {
    display: flex;
    flex-direction: column;
    gap: 32px;
  }

  .settings-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  /* Loading State */
  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
    padding: 64px 32px;
    color: var(--text-400);
  }

  .loading-spinner {
    width: 32px;
    height: 32px;
    border: 2px solid var(--border-200);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* Empty State */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 20px;
    padding: 48px 32px;
    background: var(--bg-200);
    border: 1px solid var(--border-100);
    border-radius: 12px;
    text-align: center;
  }

  .empty-icon {
    width: 56px;
    height: 56px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-300);
    border-radius: 14px;
    color: var(--text-400);
  }

  .empty-icon svg {
    width: 28px;
    height: 28px;
  }

  .empty-text h4 {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-100);
    margin: 0 0 4px;
  }

  .empty-text p {
    font-size: 13px;
    color: var(--text-400);
    margin: 0;
  }

  /* Models List */
  .models-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .model-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px;
    background: var(--bg-200);
    border: 1px solid var(--border-100);
    border-radius: 10px;
    transition: border-color 0.15s ease;
  }

  .model-card:hover {
    border-color: var(--border-200);
  }

  .model-main {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .model-icon {
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-300);
    border-radius: 8px;
    color: var(--text-300);
  }

  .model-icon svg {
    width: 18px;
    height: 18px;
  }

  .model-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .model-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-100);
    font-family: var(--font-family-mono);
  }

  .model-meta {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .model-provider {
    font-size: 12px;
    color: var(--text-400);
  }

  .model-badge {
    font-size: 10px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 4px;
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }

  .model-badge.apikey {
    color: var(--text-400);
    background: var(--bg-400);
  }

  .model-badge.oauth {
    color: #10a37f;
    background: color-mix(in srgb, #10a37f 15%, var(--bg-400));
  }

  .model-actions {
    display: flex;
    gap: 4px;
  }

  /* Buttons */
  .add-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: 13px;
    font-weight: 500;
    color: var(--color-primary);
    background: transparent;
    border: 1px solid var(--color-primary);
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .add-btn:hover {
    background: color-mix(in srgb, var(--color-primary) 10%, transparent);
  }

  .add-btn svg {
    width: 14px;
    height: 14px;
  }

  .icon-btn {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: var(--text-400);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .icon-btn:hover {
    background: var(--bg-300);
    color: var(--text-100);
  }

  .icon-btn.danger:hover {
    background: color-mix(in srgb, var(--color-error) 15%, transparent);
    color: var(--color-error);
  }

  .icon-btn svg {
    width: 16px;
    height: 16px;
  }

  .primary-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px 16px;
    font-size: 13px;
    font-weight: 500;
    color: white;
    background: var(--color-primary);
    border: none;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .primary-btn:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .primary-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .primary-btn svg {
    width: 16px;
    height: 16px;
  }

  .secondary-btn {
    padding: 8px 16px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-200);
    background: var(--bg-300);
    border: 1px solid var(--border-200);
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .secondary-btn:hover:not(:disabled) {
    background: var(--bg-400);
  }

  .secondary-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .cancel-btn {
    padding: 8px 16px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-300);
    background: transparent;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .cancel-btn:hover {
    color: var(--text-100);
    background: var(--bg-300);
  }

  /* Auth Tabs */
  .auth-tabs {
    display: flex;
    border-bottom: 1px solid var(--border-100);
  }

  .auth-tab {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 14px 16px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-400);
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .auth-tab:hover {
    color: var(--text-200);
    background: var(--bg-250);
  }

  .auth-tab.active {
    color: var(--color-primary);
    border-bottom-color: var(--color-primary);
  }

  .auth-tab svg {
    width: 18px;
    height: 18px;
  }

  /* Form */
  .form-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-group.inline {
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
  }

  .form-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-200);
  }

  .form-input {
    padding: 10px 12px;
    font-size: 13px;
    color: var(--text-100);
    background: var(--bg-300);
    border: 1px solid var(--border-200);
    border-radius: 6px;
    outline: none;
    transition: all 0.15s ease;
  }

  .form-input:focus {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 15%, transparent);
  }

  .form-input.mono {
    font-family: var(--font-family-mono);
  }

  .form-input::placeholder {
    color: var(--text-500);
  }

  /* OAuth */
  .oauth-info {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .oauth-provider-card {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px;
    background: var(--bg-300);
    border-radius: 10px;
  }

  .oauth-provider-icon {
    width: 44px;
    height: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #10a37f;
    border-radius: 10px;
    color: white;
  }

  .oauth-provider-icon svg {
    width: 24px;
    height: 24px;
  }

  .oauth-provider-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-100);
  }

  .oauth-provider-desc {
    font-size: 12px;
    color: var(--text-400);
  }

  .oauth-status-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    background: var(--bg-250);
    border-radius: 8px;
  }

  .oauth-status {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-400);
  }

  .oauth-status.authorized {
    color: var(--color-success, #22c55e);
  }

  .status-icon {
    width: 18px;
    height: 18px;
  }

  .status-icon.success {
    color: var(--color-success, #22c55e);
  }

  .auth-btn {
    padding: 8px 14px;
    font-size: 13px;
    font-weight: 500;
    color: white;
    background: var(--color-primary);
    border: none;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .auth-btn:hover {
    filter: brightness(1.1);
  }

  .auth-btn.secondary {
    color: var(--text-200);
    background: var(--bg-400);
  }

  .auth-btn.secondary:hover {
    background: var(--bg-500);
  }

  /* Form Actions */
  .form-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px 20px;
    border-top: 1px solid var(--border-100);
    background: var(--bg-250);
  }
</style>
