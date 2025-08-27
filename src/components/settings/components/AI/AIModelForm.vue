<script setup lang="ts">
  import type { AIModelConfig } from '@/types'
  import { reactive, ref } from 'vue'
  import { createMessage } from '@/ui'
  import { handleError } from '@/utils/errorHandler'
  import { aiApi } from '@/api'
  import { useI18n } from 'vue-i18n'
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

  // 表单数据
  const formData = reactive({
    name: '',
    provider: 'openAI' as AIModelConfig['provider'],
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

  // 提供商选项
  const providerOptions = [
    {
      value: 'openAI',
      label: 'OpenAI',
      description: t('ai_model.providers.openai_description'),
    },
    {
      value: 'claude',
      label: 'Claude',
      description: t('ai_model.providers.claude_description'),
    },
    {
      value: 'custom',
      label: 'Custom',
      description: t('ai_model.providers.custom_description'),
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

    if (!formData.name.trim()) errors.value.name = t('ai_model_form.model_name_required')
    if (!formData.apiUrl.trim()) errors.value.apiUrl = t('ai_model_form.api_url_required')
    if (!formData.apiKey.trim()) errors.value.apiKey = t('ai_model_form.api_key_required')
    if (!formData.model.trim()) errors.value.model = t('ai_model_form.model_name_required')

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
      createMessage.warning(t('ai_model.validation.required_fields'))
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
        options: formData.options,
      }

      // 调用连接测试 API
      const isConnected = await aiApi.testConnectionWithConfig(testConfig)

      if (isConnected) {
        createMessage.success(t('ai_model.connection_test.success'))
      } else {
        createMessage.error(t('ai_model.connection_test.failed'))
      }
    } catch (error) {
      createMessage.error(handleError(error, t('ai_model.connection_test.error')))
    } finally {
      isTesting.value = false
    }
  }
</script>

<template>
  <x-modal
    :visible="true"
    :title="props.model ? t('ai_model_form.edit_title') : t('ai_model_form.add_title')"
    size="large"
    show-footer
    :show-cancel-button="false"
    :show-confirm-button="false"
    @close="handleCancel"
  >
    <template #footer>
      <div class="modal-footer">
        <x-button variant="secondary" :loading="isTesting" @click="handleTestConnection">
          {{ isTesting ? $t('ai_model.testing') : $t('ai_model.test_connection') }}
        </x-button>
        <div class="footer-right">
          <x-button variant="secondary" @click="handleCancel">{{ $t('dialog.cancel') }}</x-button>
          <x-button variant="primary" :loading="isSubmitting" @click="handleSubmit">
            {{ props.model ? $t('ai_model.save') : $t('ai_model.add') }}
          </x-button>
        </div>
      </div>
    </template>
    <form @submit.prevent="handleSubmit">
      <!-- 基本信息 -->
      <div class="form-section">
        <h4 class="section-title">{{ t('ai_model_form.basic_info') }}</h4>

        <div class="form-group">
          <label class="form-label">{{ t('ai_model_form.config_name') }}</label>
          <input
            v-model="formData.name"
            type="text"
            class="form-input"
            :class="{ error: errors.name }"
            :placeholder="t('ai_model_form.config_name_placeholder')"
          />
          <div v-if="errors.name" class="error-message">{{ errors.name }}</div>
        </div>

        <div class="form-group">
          <label class="form-label">{{ t('ai_model_form.provider') }}</label>
          <x-select
            v-model="formData.provider"
            :options="providerOptions"
            :placeholder="t('ai_model_form.provider_placeholder')"
          />
        </div>
      </div>

      <!-- 连接配置 -->
      <div class="form-section">
        <h4 class="section-title">{{ t('ai_model_form.connection_config') }}</h4>

        <div class="form-row">
          <div class="form-group">
            <label class="form-label">{{ t('ai_model_form.api_url') }}</label>
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
            <label class="form-label">{{ t('ai_model_form.model_name') }}</label>
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
          <label class="form-label">{{ t('ai_model_form.api_key') }}</label>
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
    margin-top: 1rem;
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
