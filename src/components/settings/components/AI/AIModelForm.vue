<script setup lang="ts">
  import type { AIModelConfig } from '@/types'
  import { reactive, ref, watch } from 'vue'

  interface Props {
    model?: AIModelConfig | null
  }

  interface Emits {
    (e: 'submit', data: Omit<AIModelConfig, 'id'>): void
    (e: 'cancel'): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()

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
      description: 'OpenAI GPT 模型',
    },
    {
      value: 'claude',
      label: 'Claude',
      description: 'Anthropic Claude 模型',
    },
    {
      value: 'custom',
      label: '自定义',
      description: '自定义API端点',
    },
  ]

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

  // 简化的表单验证
  const validateForm = () => {
    errors.value = {}

    if (!formData.name.trim()) errors.value.name = '请输入模型名称'
    if (!formData.apiUrl.trim()) errors.value.apiUrl = '请输入API地址'
    if (!formData.apiKey.trim()) errors.value.apiKey = '请输入API密钥'
    if (!formData.model.trim()) errors.value.model = '请输入模型名称'

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
</script>

<template>
  <x-modal
    :visible="true"
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
    @close="handleCancel"
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
          <x-select v-model="formData.provider" :options="providerOptions" placeholder="选择AI提供商" />
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
          <input
            v-model="formData.model"
            type="text"
            class="form-input"
            :class="{ error: errors.model }"
            placeholder="gpt-4"
          />
          <div v-if="errors.model" class="error-message">{{ errors.model }}</div>
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
  }

  .form-input.error {
    border-color: var(--color-danger);
  }

  .error-message {
    font-size: var(--font-size-xs);
    color: var(--color-danger);
    margin-top: var(--spacing-xs);
  }
</style>
