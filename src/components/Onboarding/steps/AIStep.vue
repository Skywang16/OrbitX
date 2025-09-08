<template>
  <div class="ai-step">
    <div class="step-header">
      <h2 class="step-title">{{ t('onboarding.ai.title') }}</h2>
      <p class="step-description">{{ t('onboarding.ai.description') }}</p>
    </div>

    <div class="ai-options">
      <TransitionGroup name="provider-list" tag="div" class="provider-container">
        <!-- 选择提供商 -->
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
          <!-- 卡片头部 -->
          <div class="ai-option-header" @click="selectProvider(provider.id)">
            <div class="ai-info">
              <div class="ai-name">{{ provider.name }}</div>
            </div>
            <div class="ai-badge" v-if="provider.recommended">
              {{ t('onboarding.ai.recommended') }}
            </div>
          </div>

          <!-- 展开的配置表单 -->
          <Transition name="dropdown" appear>
            <div v-if="selectedProvider === provider.id" class="ai-config-dropdown">
              <div class="config-divider"></div>

              <div class="form-group">
                <label class="form-label">{{ t('onboarding.ai.config_name') }}</label>
                <input
                  v-model="formData.name"
                  type="text"
                  class="form-input"
                  :class="{ error: errors.name }"
                  :placeholder="t('onboarding.ai.config_name_placeholder')"
                />
                <div v-if="errors.name" class="error-message">{{ errors.name }}</div>
              </div>

              <div class="form-group">
                <label class="form-label">{{ t('ai_model_form.api_key') }}</label>
                <input
                  v-model="formData.apiKey"
                  type="password"
                  class="form-input"
                  :class="{ error: errors.apiKey }"
                  :placeholder="t('onboarding.ai.api_key_placeholder')"
                />
                <div v-if="errors.apiKey" class="error-message">{{ errors.apiKey }}</div>
              </div>

              <div class="form-group" v-if="selectedProvider === 'custom'">
                <label class="form-label">{{ t('ai_model_form.api_url') }}</label>
                <input
                  v-model="formData.apiUrl"
                  type="url"
                  class="form-input"
                  :class="{ error: errors.apiUrl }"
                  :placeholder="t('onboarding.ai.api_url_placeholder')"
                />
                <div v-if="errors.apiUrl" class="error-message">{{ errors.apiUrl }}</div>
              </div>

              <div class="form-group">
                <label class="form-label">{{ t('onboarding.ai.model_name') }}</label>
                <!-- 预设provider使用下拉选择 -->
                <x-select
                  v-if="selectedProvider !== 'custom' && availableModels.length > 0"
                  v-model="formData.model"
                  :options="availableModels"
                  :placeholder="t('ai_model.select_model')"
                  :class="{ error: errors.model }"
                />
                <!-- 自定义provider使用文本输入 -->
                <input
                  v-else
                  v-model="formData.model"
                  type="text"
                  class="form-input"
                  :class="{ error: errors.model }"
                  :placeholder="getModelPlaceholder()"
                />
                <div v-if="errors.model" class="error-message">{{ errors.model }}</div>
              </div>
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
  import { createMessage, XSelect } from '@/ui'
  import { handleError, handleErrorWithMessage } from '@/utils/errorHandler'

  import type { AIModelConfig } from '@/types'
  import { useAISettingsStore } from '@/components/settings/components/AI/store'
  import { useLLMRegistry } from '@/composables/useLLMRegistry'

  const { t } = useI18n()
  const aiSettingsStore = useAISettingsStore()

  // 使用后端LLM注册表
  const { providerOptions, getModelOptions, loadProviders } = useLLMRegistry()

  const selectedProvider = ref('')

  // 从后端注册表生成可用供应商列表
  const availableProviders = computed(() => {
    const providers = providerOptions.value.map((provider, index) => ({
      id: provider.value,
      name: provider.label,
      recommended: index === 0, // 第一个供应商设为推荐
    }))

    // 添加自定义选项
    providers.push({
      id: 'custom',
      name: t('onboarding.ai.models.custom.name'),
      recommended: false,
    })

    return providers
  })

  // 组件挂载时确保数据已加载
  onMounted(async () => {
    if (providerOptions.value.length === 0) {
      await loadProviders()
    }
  })

  const formData = reactive({
    name: '',
    provider: 'anthropic' as AIModelConfig['provider'],
    apiKey: '',
    apiUrl: '',
    model: '',
    options: {
      maxTokens: 4096,
      temperature: 0.7,
      timeout: 300000,
    },
  })

  const errors = ref<Record<string, string>>({})
  const isSubmitting = ref(false)

  // 计算可见的提供商列表
  const visibleProviders = computed(() => {
    if (!selectedProvider.value) {
      return availableProviders.value
    }
    // 只显示选中的提供商
    return availableProviders.value.filter(provider => provider.id === selectedProvider.value)
  })

  // 计算当前选中provider的可用模型
  const availableModels = computed(() => {
    if (!selectedProvider.value || selectedProvider.value === 'custom') {
      return []
    }
    return getModelOptions(selectedProvider.value)
  })

  const selectProvider = (providerId: string) => {
    // 如果点击的是已选中的提供商，则取消选中
    if (selectedProvider.value === providerId) {
      selectedProvider.value = ''
    } else {
      selectedProvider.value = providerId
      formData.provider = providerId as AIModelConfig['provider']

      // 如果不是自定义provider，自动设置API URL和默认模型
      if (providerId !== 'custom') {
        const provider = providerOptions.value.find(p => p.value === providerId)
        if (provider) {
          formData.apiUrl = provider.apiUrl
          const models = getModelOptions(providerId)
          if (models.length > 0) {
            formData.model = models[0].value // 默认选择第一个模型
          }
        }
      }
    }
    // 重置表单数据
    formData.name = ''
    formData.apiKey = ''
    if (providerId === 'custom') {
      formData.apiUrl = ''
      formData.model = ''
    }
    errors.value = {}
  }

  const getModelPlaceholder = () => {
    if (selectedProvider.value === 'custom') {
      return t('onboarding.ai.model_name_placeholder')
    }

    // 从后端注册表获取第一个可用模型作为占位符
    const models = getModelOptions(selectedProvider.value)
    return models.length > 0 ? models[0].value : ''
  }

  // 简化的表单验证
  const validateForm = () => {
    errors.value = {}

    if (!formData.name.trim()) errors.value.name = t('onboarding.ai.config_name_required')
    if (!formData.apiKey.trim()) errors.value.apiKey = t('onboarding.ai.api_key_required')
    if (!formData.model.trim()) errors.value.model = t('onboarding.ai.model_name_required')

    // 自定义提供商需要API URL
    if (selectedProvider.value === 'custom' && !formData.apiUrl.trim()) {
      errors.value.apiUrl = t('onboarding.ai.api_url_required')
    }

    return Object.keys(errors.value).length === 0
  }

  // 获取默认API URL
  const getDefaultApiUrl = () => {
    if (selectedProvider.value === 'custom') {
      return formData.apiUrl
    }

    // 从后端注册表获取默认API URL
    const provider = providerOptions.value.find(p => p.value === selectedProvider.value)
    return provider?.apiUrl || formData.apiUrl
  }

  // 保存配置
  const handleSaveConfig = async (): Promise<boolean> => {
    // 如果没有选择提供商，提示用户选择或跳过
    if (!selectedProvider.value) {
      createMessage.warning(t('onboarding.ai.select_provider_first'))
      return false
    }

    if (!validateForm()) return false

    isSubmitting.value = true
    try {
      const newModel: AIModelConfig = {
        id: Date.now().toString(),
        name: formData.name,
        provider: formData.provider,
        apiUrl: getDefaultApiUrl(),
        apiKey: formData.apiKey,
        model: formData.model,
        modelType: 'chat', // 默认为聊天模型
        options: formData.options,
      }

      // 调用AI设置store来保存配置
      await aiSettingsStore.addModel(newModel)
      createMessage.success(t('onboarding.ai.save_config_success'))

      // 重置表单
      selectedProvider.value = ''
      Object.assign(formData, {
        name: '',
        provider: 'anthropic' as AIModelConfig['provider'],
        apiKey: '',
        apiUrl: '',
        model: '',
        options: { maxTokens: 4096, temperature: 0.7, timeout: 300000 },
      })
      errors.value = {}
      return true
    } catch (error) {
      handleErrorWithMessage(error, t('onboarding.ai.save_config_failed'))
      return false
    } finally {
      isSubmitting.value = false
    }
  }

  // 暂时跳过
  const handleSkip = () => {
    createMessage.info(t('onboarding.ai.skip_config_message'))
    return true // 返回true表示可以继续
  }

  // 暴露给父组件
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
  }

  .step-header {
    text-align: center;
    margin-bottom: 40px;
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
    margin-bottom: 40px;
  }

  .provider-container {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .ai-option {
    background: var(--bg-200);
    border: 2px solid var(--border-100);
    border-radius: 12px;
    cursor: pointer;
    transition:
      border-color 0.15s cubic-bezier(0.4, 0, 0.2, 1),
      background-color 0.15s cubic-bezier(0.4, 0, 0.2, 1);
    position: relative;
    overflow: hidden;
    will-change: border-color, background-color;
  }

  .ai-option-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px;
    transition: background-color 0.15s cubic-bezier(0.4, 0, 0.2, 1);
    will-change: background-color;
  }

  /* 只有未选中时才显示hover效果 */
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

  .ai-badge {
    font-size: 11px;
    font-weight: 600;
    color: var(--color-primary);
    background: var(--color-primary-alpha);
    padding: 4px 8px;
    border-radius: 6px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .ai-config-dropdown {
    padding: 0 20px 24px 20px;
    height: 40vh;
    overflow-y: auto;
  }

  /* 自定义滚动条样式 */
  .ai-config-dropdown::-webkit-scrollbar {
    width: 6px;
  }

  .ai-config-dropdown::-webkit-scrollbar-track {
    background: var(--bg-300);
    border-radius: 3px;
  }

  .ai-config-dropdown::-webkit-scrollbar-thumb {
    background: var(--border-200);
    border-radius: 3px;
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
    border-radius: 8px;
    transition: border-color 0.2s ease;
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

  /* XSelect样式调整，使其与表单输入框保持一致 */
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
    border-radius: 8px;
    transition: border-color 0.2s ease;
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

  /* 动画效果优化 */
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

  /* 下拉展开动画优化 */
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

  /* 选中状态的卡片样式调整 */
  .ai-option.expanded {
    border-color: var(--color-primary);
    background: var(--bg-250);
  }

  /* 移动到顶部的动画优化 */
  .ai-option.move-to-top {
    order: -1;
    will-change: transform, order;
    transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  }

  /* 为未选中的卡片添加位移动画 */
  .ai-option:not(.move-to-top) {
    transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  }
</style>
