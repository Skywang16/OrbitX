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
  import Button from '@/ui/components/Button.vue'

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

  // OAuth 订阅提供商配置（可扩展）
  const oauthProviders = [
    {
      id: OAuthProvider.OpenAiCodex,
      name: 'ChatGPT Plus/Pro',
      icon: 'openai',
      color: '#10a37f',
      models: [
        { value: 'gpt-4o', label: 'GPT-4o' },
        { value: 'gpt-4o-mini', label: 'GPT-4o Mini' },
        { value: 'o1', label: 'o1' },
        { value: 'o1-mini', label: 'o1 Mini' },
        { value: 'o1-pro', label: 'o1 Pro (Pro 订阅)' },
        { value: 'gpt-4', label: 'GPT-4' },
        { value: 'gpt-4-turbo', label: 'GPT-4 Turbo' },
      ],
      available: true,
    },
    {
      id: OAuthProvider.ClaudePro,
      name: 'Claude Pro',
      icon: 'anthropic',
      color: '#d97706',
      models: [
        { value: 'claude-3-5-sonnet', label: 'Claude 3.5 Sonnet' },
        { value: 'claude-3-opus', label: 'Claude 3 Opus' },
      ],
      available: false, // 暂未支持
    },
    {
      id: OAuthProvider.GeminiAdvanced,
      name: 'Gemini Advanced',
      icon: 'google',
      color: '#4285f4',
      models: [
        { value: 'gemini-ultra', label: 'Gemini Ultra' },
        { value: 'gemini-pro', label: 'Gemini Pro' },
      ],
      available: false, // 暂未支持
    },
  ]

  const formData = reactive({
    authType: 'apikey' as 'apikey' | 'oauth',
    // API Key 模式
    provider: '' as string,
    apiUrl: '',
    apiKey: '',
    model: '',
    useCustomBaseUrl: false,
    // OAuth 模式
    oauthProvider: '' as string,
    oauthConfig: undefined as OAuthConfig | undefined,
    // 通用
    options: { maxContextTokens: 128000, temperature: 0.5, timeout: 300000, maxTokens: -1 },
  })

  // 当前选中的 OAuth 提供商配置
  const currentOAuthProvider = computed(() => {
    return oauthProviders.find(p => p.id === formData.oauthProvider)
  })

  // OAuth 提供商下拉选项
  const oauthProviderOptions = computed(() => {
    return oauthProviders.map(p => ({
      value: p.id,
      label: p.name + (p.available ? '' : ' (即将支持)'),
      disabled: !p.available,
    }))
  })

  // 当前可用模型列表
  const availableModels = computed(() => {
    if (formData.authType === 'oauth') {
      return currentOAuthProvider.value?.models || []
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
  }

  const startEditing = (model: AIModelConfig) => {
    editingId.value = model.id
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
    showAdvancedOptions.value = false
  }

  const switchAuthType = (type: 'apikey' | 'oauth') => {
    formData.authType = type
    formData.model = ''
    if (type === 'oauth') {
      formData.provider = ''
      formData.apiKey = ''
      formData.apiUrl = ''
      // 默认选择第一个可用的 OAuth 提供商
      const firstAvailable = oauthProviders.find(p => p.available)
      if (firstAvailable) {
        formData.oauthProvider = firstAvailable.id
        formData.model = firstAvailable.models[0]?.value || ''
      }
    } else {
      formData.oauthConfig = undefined
      formData.oauthProvider = ''
    }
  }

  const handleOAuthProviderChange = (value: string) => {
    formData.oauthProvider = value
    formData.oauthConfig = undefined
    const provider = oauthProviders.find(p => p.id === value)
    formData.model = provider?.models[0]?.value || ''
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
              provider: 'openai_compatible' as AIProvider, // TODO: 根据 oauthProvider 动态设置
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

  // 获取模型显示信息
  const getModelBadge = (model: AIModelConfig) => {
    if (model.authType === AuthType.OAuth) {
      const provider = oauthProviders.find(p => p.id === model.oauthConfig?.provider)
      return { name: provider?.name || 'OAuth', color: provider?.color || '#10a37f' }
    }
    return null
  }
</script>

<template>
  <div class="ai-model-config">
    <!-- Loading State -->
    <div v-if="loading" class="settings-loading">
      <div class="settings-loading-spinner"></div>
      <span>{{ t('ai_model.loading') }}</span>
    </div>

    <template v-else>
      <!-- Add/Edit Model Form -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          {{ editingId ? t('ai_model.edit_model') || 'Edit Model' : t('ai_model.add_model') || 'Add Model' }}
        </h3>

        <SettingsCard>
          <div class="model-form">
            <!-- Auth Type Selection -->
            <div class="form-section">
              <div class="form-label">{{ t('ai_model.connection_type') || '连接方式' }}</div>
              <div class="auth-type-selector">
                <button
                  type="button"
                  class="auth-type-btn"
                  :class="{ active: formData.authType === 'apikey' }"
                  @click="switchAuthType('apikey')"
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="auth-type-icon">
                    <path
                      d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"
                    />
                  </svg>
                  <span>API Key</span>
                </button>
                <button
                  type="button"
                  class="auth-type-btn"
                  :class="{ active: formData.authType === 'oauth' }"
                  @click="switchAuthType('oauth')"
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="auth-type-icon">
                    <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
                    <circle cx="12" cy="7" r="4" />
                  </svg>
                  <span>{{ t('ai_model.subscription') || '订阅账号' }}</span>
                </button>
              </div>
            </div>

            <!-- API Key Mode -->
            <template v-if="formData.authType === 'apikey'">
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

              <div v-if="formData.provider" class="settings-item">
                <div class="settings-item-header">
                  <div class="settings-label">{{ t('ai_model.api_key') }}</div>
                </div>
                <div class="settings-item-control">
                  <input
                    v-model="formData.apiKey"
                    type="password"
                    class="settings-input mono"
                    :placeholder="t('ai_model.api_key_placeholder')"
                  />
                </div>
              </div>

              <div v-if="formData.provider && availableModels.length > 0" class="settings-item">
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

              <div v-if="formData.provider && hasPresetModels" class="settings-item">
                <div class="settings-item-header">
                  <div class="settings-label">{{ t('ai_model.use_custom_base_url') }}</div>
                </div>
                <div class="settings-item-control">
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
              </div>

              <div v-if="formData.provider && (formData.useCustomBaseUrl || !hasPresetModels)" class="settings-item">
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
            </template>

            <!-- OAuth Mode -->
            <template v-else>
              <div class="settings-item">
                <div class="settings-item-header">
                  <div class="settings-label">{{ t('ai_model.subscription_service') || '订阅服务' }}</div>
                </div>
                <div class="settings-item-control">
                  <x-select
                    v-model="formData.oauthProvider"
                    :options="oauthProviderOptions"
                    :placeholder="t('ai_model.select_subscription') || '选择订阅服务'"
                    @update:modelValue="handleOAuthProviderChange"
                  />
                </div>
              </div>

              <div v-if="formData.oauthProvider && currentOAuthProvider?.available" class="settings-item">
                <div class="settings-item-header">
                  <div class="settings-label">{{ t('ai_model.authorization') || '账号授权' }}</div>
                </div>
                <div class="settings-item-control">
                  <div class="oauth-status-container">
                    <div class="oauth-status" :class="{ authorized: formData.oauthConfig }">
                      <svg
                        v-if="formData.oauthConfig"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2.5"
                        class="oauth-status-icon success"
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
                        class="oauth-status-icon"
                      >
                        <circle cx="12" cy="12" r="10" />
                        <line x1="12" y1="8" x2="12" y2="12" />
                        <line x1="12" y1="16" x2="12.01" y2="16" />
                      </svg>
                      <span>
                        {{
                          formData.oauthConfig
                            ? t('ai_model.authorized') || '已授权'
                            : t('ai_model.not_authorized') || '未授权'
                        }}
                      </span>
                    </div>
                    <Button
                      :variant="formData.oauthConfig ? 'secondary' : 'primary'"
                      size="small"
                      @click="showOAuthDialog = true"
                    >
                      {{
                        formData.oauthConfig
                          ? t('ai_model.reauthorize') || '重新授权'
                          : t('ai_model.start_authorization') || '开始授权'
                      }}
                    </Button>
                  </div>
                </div>
              </div>

              <div v-if="formData.oauthProvider && currentOAuthProvider?.available" class="settings-item">
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
            </template>

            <!-- Advanced Options Toggle -->
            <div
              v-if="
                (formData.authType === 'apikey' && formData.provider) ||
                (formData.authType === 'oauth' && formData.oauthProvider)
              "
              class="settings-item clickable advanced-toggle"
              @click="showAdvancedOptions = !showAdvancedOptions"
            >
              <div class="settings-item-header">
                <div class="settings-label">
                  <svg
                    class="toggle-chevron"
                    :class="{ expanded: showAdvancedOptions }"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                  >
                    <polyline points="9 18 15 12 9 6" />
                  </svg>
                  {{ t('ai_model.advanced_options') }}
                </div>
              </div>
            </div>

            <!-- Advanced Options -->
            <template v-if="showAdvancedOptions">
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
              <div v-if="formData.authType === 'apikey'" class="settings-item">
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

            <!-- Form Actions -->
            <div class="form-actions">
              <Button v-if="editingId" variant="ghost" size="small" @click="resetForm">
                {{ t('common.cancel') }}
              </Button>
              <Button
                v-if="formData.authType === 'apikey'"
                variant="secondary"
                size="small"
                :loading="isTesting"
                :disabled="!isFormValid"
                @click="testConnection"
              >
                {{ isTesting ? t('ai_model.testing') : t('ai_model.test_connection') }}
              </Button>
              <Button variant="primary" size="small" :loading="isSaving" :disabled="!isFormValid" @click="saveModel">
                {{ editingId ? t('common.save') : t('common.add') }}
              </Button>
            </div>
          </div>
        </SettingsCard>
      </div>

      <!-- Configured Models List -->
      <div v-if="models.length > 0" class="settings-section">
        <h3 class="settings-section-title">{{ t('ai_model.configured_models') || 'Configured Models' }}</h3>

        <SettingsCard>
          <div v-for="model in models" :key="model.id" class="settings-item model-item">
            <div class="settings-item-header">
              <div class="model-info">
                <div class="model-name">{{ model.model }}</div>
                <div class="model-meta">
                  <template v-if="getModelBadge(model)">
                    <span class="model-badge oauth" :style="{ '--badge-color': getModelBadge(model)?.color }">
                      {{ getModelBadge(model)?.name }}
                    </span>
                  </template>
                  <template v-else>
                    <span class="model-provider">{{ model.provider }}</span>
                    <span class="model-badge apikey">API Key</span>
                  </template>
                  <span v-if="model.useCustomBaseUrl" class="model-tag">custom url</span>
                </div>
              </div>
            </div>
            <div class="settings-item-control">
              <Button variant="ghost" size="small" @click="startEditing(model)">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="btn-icon">
                  <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
                  <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
                </svg>
              </Button>
              <Button variant="ghost" size="small" class="danger-btn" @click="deleteModel(model.id)">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="btn-icon">
                  <polyline points="3 6 5 6 21 6" />
                  <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                </svg>
              </Button>
            </div>
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

  /* Form Section */
  .model-form {
    display: flex;
    flex-direction: column;
  }

  .model-form :deep(.settings-item)::after {
    display: none !important;
  }

  .form-section {
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-100);
  }

  .form-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-400);
    text-transform: uppercase;
    letter-spacing: 0.03em;
    margin-bottom: 12px;
  }

  /* Auth Type Selector */
  .auth-type-selector {
    display: flex;
    gap: 8px;
  }

  .auth-type-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 10px 16px;
    background: var(--bg-300);
    border: 1px solid var(--border-200);
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s ease;
    color: var(--text-300);
    font-size: 13px;
    font-weight: 500;
  }

  .auth-type-btn:hover {
    border-color: var(--border-300);
    color: var(--text-100);
  }

  .auth-type-btn.active {
    border-color: var(--color-primary);
    background: color-mix(in srgb, var(--color-primary) 8%, var(--bg-300));
    color: var(--color-primary);
  }

  .auth-type-icon {
    width: 18px;
    height: 18px;
  }

  /* OAuth Status */
  .oauth-status-container {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .oauth-status {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--text-400);
  }

  .oauth-status.authorized {
    color: var(--color-success, #22c55e);
  }

  .oauth-status-icon {
    width: 16px;
    height: 16px;
  }

  .oauth-status-icon.success {
    color: var(--color-success, #22c55e);
  }

  /* Advanced Toggle */
  .advanced-toggle .settings-label {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-400);
  }

  .toggle-chevron {
    width: 14px;
    height: 14px;
    transition: transform 0.2s ease;
  }

  .toggle-chevron.expanded {
    transform: rotate(90deg);
  }

  /* Form Actions */
  .form-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px 20px;
    border-top: 1px solid var(--border-100);
    background: var(--bg-250, color-mix(in srgb, var(--bg-300) 30%, var(--bg-200)));
  }

  /* Model List */
  .model-item {
    padding: 14px 20px !important;
  }

  .model-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .model-name {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-100);
    font-family: var(--font-family-mono);
  }

  .model-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .model-provider {
    font-size: 12px;
    color: var(--text-400);
  }

  .model-badge {
    display: inline-flex;
    align-items: center;
    font-size: 10px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 4px;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .model-badge.apikey {
    color: var(--text-400);
    background: var(--bg-400);
  }

  .model-badge.oauth {
    color: var(--badge-color, #10a37f);
    background: color-mix(in srgb, var(--badge-color, #10a37f) 15%, var(--bg-400));
  }

  .model-tag {
    font-size: 10px;
    color: var(--text-500);
    background: var(--bg-400);
    padding: 2px 6px;
    border-radius: 4px;
  }

  /* Button Icons */
  .btn-icon {
    width: 16px;
    height: 16px;
  }

  .danger-btn {
    color: var(--text-400);
  }

  .danger-btn:hover {
    color: var(--color-error, #ef4444);
    background: color-mix(in srgb, var(--color-error, #ef4444) 10%, transparent);
  }
</style>
