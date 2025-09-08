<script setup lang="ts">
  import type { AIModelConfig } from '@/types'
  
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
  const { providerOptions, getModelOptions, loadProviders } = useLLMRegistry()

  // 配置模式：preset（预设）或 custom（自定义）——默认自定义，避免误判
  const configMode = ref<'preset' | 'custom'>('custom')
  const selectedPreset = ref<string>('')

  // 组件挂载时确保数据已加载
  onMounted(async () => {
    if (providerOptions.value.length === 0) {
      await loadProviders()
    }
  })

  // 表单数据
  const formData = reactive({
    name: '',
    provider: 'custom' as AIModelConfig['provider'],
    apiUrl: '',
    apiKey: '',
    model: '',
    modelType: (props.defaultModelType || 'chat') as AIModelConfig['modelType'],
    options: {
      maxTokens: 4096,
      temperature: 0.7,
      timeout: 300000,
    },
  })

  // 表单验证状态
  const errors = ref<Record<string, string>>({})
  const isSubmitting = ref(false)
  const isTesting = ref(false)

  // 计算属性：是否为预设模式
  const isPresetMode = computed(() => configMode.value === 'preset')

  // 计算属性：当前预设的可用模型
  const availableModels = computed(() => {
    if (isPresetMode.value && selectedPreset.value) {
      return getModelOptions(selectedPreset.value)
    }
    return []
  })

  // 初始化表单数据
  if (props.model) {
    Object.assign(formData, {
      name: props.model.name,
      provider: props.model.provider,
      apiUrl: props.model.apiUrl,
      apiKey: props.model.apiKey,
      model: props.model.model,
      modelType: props.model.modelType || 'chat',
      options: {
        maxTokens: props.model.options?.maxTokens || 4096,
        temperature: props.model.options?.temperature || 0.7,
        timeout: props.model.options?.timeout || 300000,
      },
    })

    // 简化：仅依据 provider 判断模式
    if (props.model.provider === 'custom') {
      configMode.value = 'custom'
      selectedPreset.value = ''
    } else {
      configMode.value = 'preset'
      selectedPreset.value = String(props.model.provider)
    }
  }

  // 监听配置模式变化
  const handleConfigModeChange = (mode: 'preset' | 'custom') => {
    configMode.value = mode
    if (mode === 'preset') {
      selectedPreset.value = ''
      formData.apiUrl = ''
      formData.model = ''
      // provider 将在选择预设时由 handlePresetChange 设置
    } else {
      selectedPreset.value = ''
      formData.name = ''
      // 自定义模式下，provider 固定为 custom
      formData.provider = 'custom'
    }
  }

  // 监听预设选择变化
  const handlePresetChange = (presetValue: string) => {
    selectedPreset.value = presetValue
    const preset = providerOptions.value.find(p => p.value === presetValue)
    if (preset) {
      // 预设模式下，provider = 选中的预设值
      formData.provider = presetValue as AIModelConfig['provider']
      formData.apiUrl = preset.apiUrl
      const models = getModelOptions(presetValue)
      if (models.length > 0) {
        formData.model = models[0].value // 默认选择第一个模型
        // 名称直接等于选中模型的名字
        formData.name = models[0].label
      }
    }
  }

  // 更换模型时，若处于预设模式，名称=模型名称
  const handleModelChange = (modelValue: string) => {
    if (configMode.value !== 'preset') return
    const models = getModelOptions(selectedPreset.value)
    const modelInfo = models.find(m => m.value === modelValue)
    if (modelInfo) {
      formData.name = modelInfo.label
    }
  }

  // 表单验证
  const validateForm = () => {
    errors.value = {}

    if (!formData.apiKey.trim()) {
      errors.value.apiKey = t('ai_model.validation.api_key_required')
    }

    if (isPresetMode.value) {
      if (!selectedPreset.value) {
        errors.value.preset = t('ai_model.validation.preset_required')
      }
      if (!formData.model.trim()) {
        errors.value.model = t('ai_model.validation.model_required')
      }
    } else {
      if (!formData.name.trim()) {
        errors.value.name = t('ai_model.validation.config_name_required')
      }
      if (!formData.apiUrl.trim()) {
        errors.value.apiUrl = t('ai_model.validation.api_url_required')
      }
      if (!formData.model.trim()) {
        errors.value.model = t('ai_model.validation.model_name_required')
      }
    }

    return Object.keys(errors.value).length === 0
  }

  // 处理提交
  const handleSubmit = () => {
    if (!validateForm()) return

    isSubmitting.value = true
    try {
      // 最终提交前的兜底修正
      const submitData = { ...formData }
      if (isPresetMode.value) {
        // 预设模式下，确保 provider = 选中的预设
        if (selectedPreset.value) {
          submitData.provider = selectedPreset.value as AIModelConfig['provider']
        }
        // 若名称为空，使用当前模型的label
        if (!submitData.name.trim()) {
          const models = getModelOptions(selectedPreset.value)
          const modelInfo = models.find(m => m.value === submitData.model)
          if (modelInfo) submitData.name = modelInfo.label
        }
      } else {
        // 自定义模式
        submitData.provider = 'custom'
      }
      emit('submit', submitData)
    } finally {
      isSubmitting.value = false
    }
  }

  // 处理取消
  const handleCancel = () => {
    emit('cancel')
  }

  // 测试连接
  const handleTestConnection = async () => {
    if (!validateForm()) return

    isTesting.value = true
    const testConfig: AIModelConfig = {
      id: 'test-' + Date.now(),
      name: formData.name || 'Test Model',
      provider: formData.provider,
      apiUrl: formData.apiUrl,
      apiKey: formData.apiKey,
      model: formData.model,
      modelType: formData.modelType,
      options: formData.options,
    }

    await aiApi.testConnectionWithConfig(testConfig)
    isTesting.value = false
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
      <!-- 模型类型选择 -->
      <div class="form-row">
        <div class="form-group full-width">
          <label class="form-label">{{ t('ai_model.model_type') }}</label>
          <x-select
            v-model="formData.modelType"
            :options="[
              { value: 'chat', label: t('ai_model.model_type_chat') },
              { value: 'embedding', label: t('ai_model.model_type_embedding') },
            ]"
            :placeholder="t('ai_model.select_model_type')"
          />
          <div class="form-description">
            {{
              formData.modelType === 'chat'
                ? t('ai_model.model_type_chat_description')
                : t('ai_model.model_type_embedding_description')
            }}
          </div>
        </div>
      </div>

      <!-- 配置类型选择 -->
      <div class="form-row">
        <div class="form-group full-width">
          <label class="form-label">{{ t('ai_model.config_type') }}</label>
          <div class="tab-switcher">
            <button
              type="button"
              class="tab-button"
              :class="{ active: configMode === 'preset' }"
              @click="handleConfigModeChange('preset')"
            >
              {{ t('ai_model.preset_provider') }}
            </button>
            <button
              type="button"
              class="tab-button"
              :class="{ active: configMode === 'custom' }"
              @click="handleConfigModeChange('custom')"
            >
              {{ t('ai_model.custom_config') }}
            </button>
            <div class="tab-indicator" :class="{ 'move-right': configMode === 'custom' }"></div>
          </div>
        </div>
      </div>

      <!-- 预设模式 -->
      <div v-if="isPresetMode" class="form-row">
        <div class="form-group">
          <label class="form-label">{{ t('ai_model.provider') }}</label>
          <x-select
            v-model="selectedPreset"
            :options="providerOptions.map(p => ({ value: p.value, label: p.label }))"
            :placeholder="t('ai_model.select_provider')"
            @update:modelValue="handlePresetChange"
          />
          <div v-if="errors.preset" class="error-message">{{ errors.preset }}</div>
        </div>
        <div class="form-group">
          <label class="form-label">{{ t('ai_model.model') }}</label>
          <x-select
            v-if="availableModels.length > 0"
            v-model="formData.model"
            :options="availableModels"
            :placeholder="t('ai_model.select_model')"
            @update:modelValue="handleModelChange"
          />
          <input
            v-else
            type="text"
            class="form-input disabled"
            :placeholder="t('ai_model.select_provider_first')"
            disabled
          />
          <div v-if="errors.model" class="error-message">{{ errors.model }}</div>
        </div>
      </div>

      <!-- 自定义模式 -->
      <template v-if="!isPresetMode">
        <div class="form-row">
          <div class="form-group">
            <label class="form-label">{{ t('ai_model.config_name') }}</label>
            <input
              v-model="formData.name"
              type="text"
              class="form-input"
              :class="{ error: errors.name }"
              :placeholder="t('ai_model.config_name_placeholder')"
            />
            <div v-if="errors.name" class="error-message">{{ errors.name }}</div>
          </div>
          <div class="form-group">
            <label class="form-label">{{ t('ai_model.model_name') }}</label>
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
            <label class="form-label">{{ t('ai_model.api_url') }}</label>
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
      </template>

      <!-- API Key -->
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

  .form-input:hover {
    border-color: var(--border-400);
  }

  .form-input:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px var(--color-primary-alpha);
  }

  .form-input.error {
    border-color: var(--color-error);
  }

  .form-input.error:focus {
    box-shadow: 0 0 0 2px rgba(244, 71, 71, 0.1);
  }

  .form-input.disabled {
    background-color: var(--bg-500);
    color: var(--text-400);
    cursor: not-allowed;
    opacity: 0.6;
  }

  .form-input::placeholder {
    color: var(--text-400);
  }

  .tab-switcher {
    position: relative;
    display: flex;
    background-color: var(--bg-500);
    border-radius: var(--border-radius);
    padding: 4px;
    border: 1px solid var(--border-300);
  }

  .tab-button {
    flex: 1;
    position: relative;
    z-index: 2;
    padding: var(--spacing-md) var(--spacing-lg);
    border: none;
    background: transparent;
    color: var(--text-400);
    font-size: var(--font-size-sm);
    font-family: var(--font-family);
    font-weight: 500;
    cursor: pointer;
    transition: color var(--x-duration-normal) var(--x-ease-out);
    user-select: none;
    border-radius: var(--border-radius-sm);
    text-align: center;
  }

  .tab-button:hover {
    color: var(--text-300);
  }

  .tab-button.active {
    color: var(--text-100);
  }

  .tab-button:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  .tab-indicator {
    position: absolute;
    top: 4px;
    left: 4px;
    width: calc(50% - 4px);
    height: calc(100% - 8px);
    background-color: var(--bg-300);
    border-radius: var(--border-radius-sm);
    transition: transform var(--x-duration-normal) var(--x-ease-out);
    z-index: 1;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
  }

  .tab-indicator.move-right {
    transform: translateX(100%);
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

  .error-message::before {
    content: '⚠';
    font-size: 12px;
    flex-shrink: 0;
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
</style>
