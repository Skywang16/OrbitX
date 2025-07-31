<script setup lang="ts">
  import type { AIModelConfig, AIProvider } from '@/types'

  import { computed, reactive, ref, watch } from 'vue'

  interface Props {
    model?: AIModelConfig | null
  }

  interface Emits {
    (e: 'submit', data: Omit<AIModelConfig, 'id'>): void
    (e: 'cancel'): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()

  // Modal 显示状态
  const modalVisible = ref(true)

  // 表单数据
  const formData = reactive({
    name: '',
    provider: 'openAI' as AIModelConfig['provider'],
    apiUrl: '',
    apiKey: '',
    model: '',
    isDefault: false,
    options: {
      maxTokens: 4096,
      temperature: 0.7,
      timeout: 30000,
    },
  })

  // 表单验证状态
  const errors = ref<Record<string, string>>({})
  const isSubmitting = ref(false)

  // 提供商选项
  const providerOptions = [
    {
      value: 'openAI',
      label: 'OpenAI',
      icon: '🤖',
      description: 'OpenAI GPT 模型',
      defaultUrl: 'https://api.openai.com/v1/chat/completions',
      models: ['gpt-3.5-turbo', 'gpt-4', 'gpt-4-turbo'],
    },
    {
      value: 'claude',
      label: 'Claude',
      icon: '🧠',
      description: 'Anthropic Claude 模型',
      defaultUrl: 'https://api.anthropic.com/v1',
      models: ['claude-3-sonnet', 'claude-3-opus', 'claude-3-haiku'],
    },
    {
      value: 'local',
      label: 'Local (本地模型)',
      icon: '💻',
      description: '本地部署模型 (LM Studio, Ollama, etc.)',
      defaultUrl: 'http://127.0.0.1:1234/v1/chat/completions',
      models: ['gemma-2b-it', 'llama-3.1-8b', 'qwen2.5', 'codellama'],
    },
    {
      value: 'custom',
      label: '自定义',
      icon: '⚙️',
      description: '自定义API端点',
      defaultUrl: '',
      models: [],
    },
  ]

  // 当前选中的提供商
  const selectedProvider = computed(() => providerOptions.find(p => p.value === formData.provider))

  // 监听提供商变化，自动填充默认URL
  watch(
    () => formData.provider,
    newProvider => {
      const provider = providerOptions.find(p => p.value === newProvider)
      if (provider && provider.defaultUrl && !formData.apiUrl) {
        formData.apiUrl = provider.defaultUrl
      }
    }
  )

  // 初始化表单数据
  if (props.model) {
    Object.assign(formData, {
      name: props.model.name,
      provider: props.model.provider,
      apiUrl: props.model.apiUrl,
      apiKey: props.model.apiKey,
      model: props.model.model,
      isDefault: props.model.isDefault || false,
      options: {
        maxTokens: props.model.options?.maxTokens || 4096,
        temperature: props.model.options?.temperature || 0.7,
        timeout: props.model.options?.timeout || 30000,
      },
    })
  }

  // 表单验证
  const validateForm = () => {
    errors.value = {}

    if (!formData.name.trim()) {
      errors.value.name = '请输入模型名称'
    }

    if (!formData.apiUrl.trim()) {
      errors.value.apiUrl = '请输入API地址'
    } else {
      try {
        new URL(formData.apiUrl)
      } catch {
        errors.value.apiUrl = '请输入有效的URL地址'
      }
    }

    if (!formData.apiKey.trim()) {
      errors.value.apiKey = '请输入API密钥'
    }

    if (!formData.model.trim()) {
      errors.value.model = '请输入模型名称'
    }

    if (formData.options.maxTokens < 1 || formData.options.maxTokens > 32000) {
      errors.value.maxTokens = '最大令牌数应在1-32000之间'
    }

    if (formData.options.temperature < 0 || formData.options.temperature > 2) {
      errors.value.temperature = '温度值应在0-2之间'
    }

    return Object.keys(errors.value).length === 0
  }

  // 处理提交
  const handleSubmit = async () => {
    if (!validateForm()) {
      return
    }

    isSubmitting.value = true
    try {
      // 数据已经是camelCase格式，直接提交
      const submitData = {
        name: formData.name,
        provider: formData.provider,
        apiUrl: formData.apiUrl,
        apiKey: formData.apiKey,
        model: formData.model,
        isDefault: formData.isDefault,
        options: {
          maxTokens: formData.options.maxTokens,
          temperature: formData.options.temperature,
          timeout: formData.options.timeout,
        },
      }
      emit('submit', submitData)
    } finally {
      isSubmitting.value = false
    }
  }

  // 处理取消
  const handleCancel = () => {
    modalVisible.value = false
    emit('cancel')
  }

  // 处理Modal关闭
  const handleModalClose = () => {
    modalVisible.value = false
    emit('cancel')
  }

  // 处理模型选择
  const handleModelSelect = (modelName: string) => {
    formData.model = modelName
  }
</script>

<template>
  <x-modal
    v-model:visible="modalVisible"
    :title="props.model ? '编辑AI模型' : '添加AI模型'"
    size="large"
    show-footer
    :show-cancel-button="true"
    :show-confirm-button="true"
    cancel-text="取消"
    :confirm-text="props.model ? '保存' : '添加'"
    :loading="isSubmitting"
    loading-text="保存中..."
    @cancel="handleCancel"
    @confirm="handleSubmit"
    @close="handleModalClose"
  >
    <form @submit.prevent="handleSubmit">
      <!-- 基本信息 -->
      <div class="form-section">
        <h4 class="section-title">基本信息</h4>

        <div class="form-group">
          <label class="form-label">模型名称</label>
          <input
            v-model="formData.name"
            type="text"
            class="form-input"
            :class="{ error: errors.name }"
            placeholder="例如：GPT-4 生产环境"
          />
          <div v-if="errors.name" class="error-message">{{ errors.name }}</div>
        </div>

        <div class="form-group">
          <label class="form-label">提供商</label>
          <div class="provider-options">
            <div
              v-for="option in providerOptions"
              :key="option.value"
              class="provider-option"
              :class="{ selected: formData.provider === option.value }"
              @click="formData.provider = option.value as AIProvider"
            >
              <div class="option-header">
                <div class="option-content">
                  <div class="option-label">{{ option.label }}</div>
                  <div class="option-description">{{ option.description }}</div>
                </div>
                <div class="option-radio">
                  <div class="radio-button" :class="{ checked: formData.provider === option.value }">
                    <div class="radio-dot"></div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 连接配置 -->
      <div class="form-section">
        <h4 class="section-title">连接配置</h4>

        <div class="form-group">
          <label class="form-label">API地址</label>
          <input
            v-model="formData.apiUrl"
            type="url"
            class="form-input"
            :class="{ error: errors.apiUrl }"
            placeholder="https://api.openai.com/v1"
          />
          <div v-if="errors.apiUrl" class="error-message">{{ errors.apiUrl }}</div>
        </div>

        <div class="form-group">
          <label class="form-label">API密钥</label>
          <input
            v-model="formData.apiKey"
            type="password"
            class="form-input"
            :class="{ error: errors.apiKey }"
            placeholder="sk-..."
          />
          <div v-if="errors.apiKey" class="error-message">{{ errors.apiKey }}</div>
        </div>

        <div class="form-group">
          <label class="form-label">模型名称</label>
          <div class="model-input-group">
            <input
              v-model="formData.model"
              type="text"
              class="form-input"
              :class="{ error: errors.model }"
              placeholder="gpt-4"
            />
            <div v-if="selectedProvider?.models.length" class="model-suggestions">
              <div class="suggestions-label">常用模型：</div>
              <div class="suggestion-tags">
                <button
                  v-for="model in selectedProvider.models"
                  :key="model"
                  type="button"
                  class="suggestion-tag"
                  @click="handleModelSelect(model)"
                >
                  {{ model }}
                </button>
              </div>
            </div>
          </div>
          <div v-if="errors.model" class="error-message">{{ errors.model }}</div>
        </div>
      </div>

      <!-- 高级设置 -->
      <div class="form-section">
        <h4 class="section-title">高级设置</h4>

        <div class="form-row">
          <div class="form-group">
            <label class="form-label">最大令牌数</label>
            <input
              v-model.number="formData.options.maxTokens"
              type="number"
              class="form-input"
              :class="{ error: errors.maxTokens }"
              min="1"
              max="32000"
            />
            <div v-if="errors.maxTokens" class="error-message">{{ errors.maxTokens }}</div>
          </div>

          <div class="form-group">
            <label class="form-label">温度值</label>
            <input
              v-model.number="formData.options.temperature"
              type="number"
              class="form-input"
              :class="{ error: errors.temperature }"
              min="0"
              max="2"
              step="0.1"
            />
            <div v-if="errors.temperature" class="error-message">{{ errors.temperature }}</div>
          </div>
        </div>
      </div>
    </form>
  </x-modal>
</template>

<style scoped>
  .form-section {
    margin-bottom: var(--spacing-xl);
  }

  .section-title {
    font-size: var(--font-size-md);
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 var(--spacing-md) 0;
  }

  .form-group {
    margin-bottom: var(--spacing-md);
  }

  .form-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-md);
  }

  .form-label {
    display: block;
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: var(--spacing-xs);
  }

  .form-input {
    width: 100%;
    padding: var(--spacing-sm);
    border: 1px solid var(--border-color);
    border-radius: var(--border-radius);
    background-color: var(--color-background);
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    transition: all 0.2s ease;
  }

  .form-input:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px var(--color-primary-alpha);
  }

  .form-input.error {
    border-color: var(--color-danger);
  }

  .error-message {
    font-size: var(--font-size-xs);
    color: var(--color-danger);
    margin-top: var(--spacing-xs);
  }

  .provider-options {
    display: grid;
    gap: var(--spacing-xs);
  }

  .provider-option {
    border: 2px solid var(--border-color);
    border-radius: var(--border-radius-lg);
    padding: var(--spacing-md);
    cursor: pointer;
    transition: all 0.2s ease;
    background-color: var(--color-background);
  }

  .option-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .option-content {
    flex: 1;
  }

  .provider-option.selected {
    border-color: var(--color-primary);
    background-color: var(--color-primary-alpha);
  }

  .option-icon {
    flex-shrink: 0;
    font-size: 20px;
    color: var(--text-secondary);
    transition: color 0.2s ease;
  }

  .provider-option.selected .option-icon {
    color: var(--color-primary);
  }

  .option-content {
    flex: 1;
  }

  .option-radio {
    flex-shrink: 0;
  }

  .radio-button {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border-color);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s ease;
  }

  .radio-button.checked {
    border-color: var(--color-primary);
  }

  .radio-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: var(--color-primary);
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .radio-button.checked .radio-dot {
    opacity: 1;
  }

  .option-label {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 2px;
  }

  .option-description {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
  }

  .model-input-group {
    position: relative;
  }

  .model-suggestions {
    margin-top: var(--spacing-sm);
  }

  .suggestions-label {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    margin-bottom: var(--spacing-xs);
  }

  .suggestion-tags {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-xs);
  }

  .suggestion-tag {
    background-color: var(--color-background-secondary);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    padding: 2px var(--spacing-xs);
    border-radius: var(--border-radius);
    font-size: var(--font-size-xs);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .suggestion-tag:hover {
    border-color: var(--color-primary);
    background-color: var(--color-primary-alpha);
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    cursor: pointer;
    font-size: var(--font-size-sm);
  }

  .checkbox-input {
    width: 16px;
    height: 16px;
  }

  .switch-group {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
  }

  .switch-text {
    font-size: var(--font-size-sm);
    color: var(--text-primary);
  }
</style>
