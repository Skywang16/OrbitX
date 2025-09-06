<script setup lang="ts">
  import type { AIModelConfig } from '@/types'
  import { createMessage } from '@/ui'
  import { handleError } from '@/utils/errorHandler'
  import { aiApi } from '@/api'
  import { reactive, ref, computed, onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useLLMRegistry } from '@/composables/useLLMRegistry'

  interface Props {
    model?: AIModelConfig | null
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

  // 配置模式：preset（预设）或 custom（自定义）
  const configMode = ref<'preset' | 'custom'>('preset')
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
    provider: 'openai' as AIModelConfig['provider'],
    apiUrl: '',
    apiKey: '',
    model: '',
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
      options: {
        maxTokens: props.model.options?.maxTokens || 4096,
        temperature: props.model.options?.temperature || 0.7,
        timeout: props.model.options?.timeout || 300000,
      },
    })

    // 判断是预设还是自定义
    const preset = providerOptions.value.find(p => p.apiUrl === props.model?.apiUrl)
    if (preset) {
      configMode.value = 'preset'
      selectedPreset.value = preset.value
    } else {
      configMode.value = 'custom'
    }
  }

  // 监听配置模式变化
  const handleConfigModeChange = (mode: 'preset' | 'custom') => {
    configMode.value = mode
    if (mode === 'preset') {
      selectedPreset.value = ''
      formData.apiUrl = ''
      formData.model = ''
      formData.name = ''
    } else {
      selectedPreset.value = ''
      formData.name = ''
    }
  }

  // 监听预设选择变化
  const handlePresetChange = (presetValue: string) => {
    selectedPreset.value = presetValue
    const preset = providerOptions.value.find(p => p.value === presetValue)
    if (preset) {
      formData.provider = presetValue as AIModelConfig['provider']
      formData.apiUrl = preset.apiUrl
      const models = getModelOptions(presetValue)
      if (models.length > 0) {
        formData.model = models[0].value // 默认选择第一个模型
        formData.name = `${preset.label} - ${models[0].label}`
      }
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
      emit('submit', { ...formData })
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
    if (!validateForm()) {
      createMessage.warning(t('ai_model.test.fill_config_first'))
      return
    }

    isTesting.value = true
    try {
      const testConfig: AIModelConfig = {
        id: 'test-' + Date.now(),
        name: formData.name || 'Test Model',
        provider: formData.provider,
        apiUrl: formData.apiUrl,
        apiKey: formData.apiKey,
        model: formData.model,
        options: formData.options,
      }

      const isConnected = await aiApi.testConnectionWithConfig(testConfig)

      if (isConnected) {
        createMessage.success(t('ai_model.test.success'))
      } else {
        createMessage.error(t('ai_model.test.failed'))
      }
    } catch (error) {
      createMessage.error(handleError(error, t('ai_model.test.error')))
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
            @update:value="handlePresetChange"
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

  /* 响应式设计 */
  @media (max-width: 768px) {
    .ai-form {
      max-width: 100%;
      padding: 0 var(--spacing-md);
    }

    .form-row {
      flex-direction: column;
      gap: var(--spacing-md);
    }

    .modal-footer {
      flex-direction: column;
      gap: var(--spacing-md);
      padding: var(--spacing-md);
    }

    .footer-right {
      width: 100%;
      justify-content: flex-end;
    }

    .tab-switcher {
      flex-direction: column;
      border-radius: var(--border-radius);
      padding: var(--spacing-xs);
    }

    .tab-button {
      border-radius: var(--border-radius-sm);
      padding: var(--spacing-lg);
    }

    .tab-indicator {
      width: calc(100% - 8px);
      height: calc(50% - 4px);
      border-radius: var(--border-radius-sm);
      transform: translateY(0);
    }

    .tab-indicator.move-right {
      transform: translateY(100%);
    }

    .form-input {
      height: 40px;
      font-size: var(--font-size-lg);
    }
  }

  /* 触摸设备优化 */
  @media (hover: none) and (pointer: coarse) {
    .form-input {
      min-height: 44px;
    }

    .tab-button {
      min-height: 48px;
      padding: var(--spacing-lg) var(--spacing-xl);
    }
  }

  /* 高对比度模式支持 */
  @media (prefers-contrast: high) {
    .form-input {
      border-width: 2px;
    }

    .tab-button {
      border: 1px solid var(--border-400);
    }

    .tab-button.active {
      border-color: var(--color-primary);
      border-width: 2px;
    }
  }

  /* 减少动画模式支持 */
  @media (prefers-reduced-motion: reduce) {
    .form-input,
    .tab-button,
    .tab-indicator {
      transition: none !important;
    }
  }
</style>
