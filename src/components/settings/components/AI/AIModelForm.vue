<script setup lang="ts">
  import type { AIModelConfig } from '@/types'
  import { reactive, ref } from 'vue'
  import { createMessage } from '@/ui'
  import { handleError } from '@/utils/errorHandler'
  import { aiApi } from '@/api'
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
      timeout: 300000,
    },
  })

  // 表单验证状态
  const errors = ref<Record<string, string>>({})
  const isSubmitting = ref(false)
  const isTesting = ref(false)

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
        timeout: props.model.options?.timeout || 300000,
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

  // 测试连接
  const handleTestConnection = async () => {
    // 验证必填字段
    if (!formData.apiUrl || !formData.apiKey || !formData.model) {
      createMessage.warning('请先填写API地址、API密钥和模型名称')
      return
    }

    isTesting.value = true
    try {
      // 构造临时的模型配置用于测试
      const testConfig: AIModelConfig = {
        id: 'test-' + Date.now(), // 临时ID
        name: formData.name || 'Test Model',
        provider: formData.provider,
        apiUrl: formData.apiUrl,
        apiKey: formData.apiKey,
        model: formData.model,
        isDefault: false,
        options: formData.options,
      }

      // 调用连接测试 API
      const isConnected = await aiApi.testConnectionWithConfig(testConfig)

      if (isConnected) {
        createMessage.success('连接测试成功！')
      } else {
        createMessage.error('连接测试失败，请检查配置')
      }
    } catch (error) {
      createMessage.error(handleError(error, '连接测试失败'))
    } finally {
      isTesting.value = false
    }
  }
</script>

<template>
  <x-modal
    :visible="true"
    :title="props.model ? '编辑AI模型' : '添加AI模型'"
    size="large"
    show-footer
    :show-cancel-button="false"
    :show-confirm-button="false"
    @close="handleCancel"
  >
    <template #footer>
      <div class="modal-footer">
        <x-button variant="secondary" :loading="isTesting" @click="handleTestConnection">
          {{ isTesting ? '测试中...' : '测试连接' }}
        </x-button>
        <div class="footer-right">
          <x-button variant="secondary" @click="handleCancel">取消</x-button>
          <x-button variant="primary" :loading="isSubmitting" @click="handleSubmit">
            {{ props.model ? '保存' : '添加' }}
          </x-button>
        </div>
      </div>
    </template>
    <form @submit.prevent="handleSubmit">
      <!-- 基本信息 -->
      <div class="form-section">
        <h4 class="section-title">基本信息</h4>

        <div class="form-group">
          <label class="form-label">配置名称</label>
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

        <div class="form-row">
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
      </div>
    </form>
  </x-modal>
</template>

<style scoped>
  .form-section {
    margin-bottom: 1.5rem;
    padding: 1.25rem;
    background: var(--bg-500);
    border: 1px solid var(--border-300);
    border-radius: 8px;
  }

  .form-section:last-of-type {
    margin-bottom: 0;
  }

  .section-title {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-200);
    margin: 0 0 1rem 0;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--border-300);
  }

  .form-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .form-group {
    margin-bottom: 1rem;
  }

  .form-group:last-child {
    margin-bottom: 0;
  }

  .form-label {
    display: block;
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--text-200);
    margin-bottom: 0.5rem;
  }

  .form-input {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid var(--border-300);
    border-radius: 6px;
    background-color: var(--bg-400);
    color: var(--text-200);
    font-size: 0.875rem;
    transition: border-color 0.2s ease;
  }

  .form-input:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  .form-input.error {
    border-color: var(--color-danger);
  }

  .form-hint {
    font-size: 0.75rem;
    color: var(--text-400);
    margin-top: 0.25rem;
  }

  .error-message {
    font-size: 0.75rem;
    color: var(--color-danger);
    margin-top: 0.25rem;
  }

  .modal-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }

  .footer-right {
    display: flex;
    gap: 0.75rem;
  }

  /* 响应式设计 */
  @media (max-width: 768px) {
    .form-row {
      grid-template-columns: 1fr;
      gap: 1rem;
    }

    .modal-footer {
      flex-direction: column;
      gap: 0.75rem;
    }

    .footer-right {
      width: 100%;
      justify-content: flex-end;
    }
  }
</style>
