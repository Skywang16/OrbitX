<script setup lang="ts">
  import { ai } from '@/api/ai'
  import type { AIModelConfig } from '@/types'
  import { confirm, createMessage } from '@/ui'
  import { computed, onMounted, ref } from 'vue'
  import AIModelForm from './AIModelForm.vue'
  import { useAISettingsStore } from './store'

  // 使用AI设置store
  const aiSettingsStore = useAISettingsStore()

  // 响应式数据
  const showAddForm = ref(false)
  const editingModel = ref<AIModelConfig | null>(null)
  const testingModels = ref<Set<string>>(new Set())

  // 使用store中的数据和状态
  const models = computed(() => aiSettingsStore.models)
  const loading = computed(() => aiSettingsStore.isLoading)

  // 生命周期
  onMounted(async () => {
    // 确保数据已加载
    if (!aiSettingsStore.isInitialized) {
      await aiSettingsStore.loadSettings()
    }
  })

  // 处理添加模型
  const handleAddModel = () => {
    editingModel.value = null
    showAddForm.value = true
  }

  // 处理编辑模型
  const handleEditModel = (model: AIModelConfig) => {
    editingModel.value = { ...model }
    showAddForm.value = true
  }

  // 处理删除模型
  const handleDeleteModel = async (modelId: string) => {
    const confirmed = await confirm('确定要删除这个AI模型配置吗？')

    if (confirmed) {
      try {
        await aiSettingsStore.removeModel(modelId)
        createMessage.success('模型删除成功')
      } catch (error) {
        createMessage.error('删除失败')
      }
    }
  }

  // 处理设置默认模型
  const handleSetDefault = async (modelId: string, isDefault: boolean) => {
    if (isDefault) {
      return
    }
    try {
      await aiSettingsStore.setDefaultModel(modelId)
      createMessage.success('默认模型设置成功')
    } catch (error) {
      createMessage.error('设置默认模型失败')
    }
  }

  // 处理测试连接
  const handleTestConnection = async (modelId: string) => {
    testingModels.value.add(modelId)
    try {
      const result = await ai.testConnection(modelId)
      if (result) {
        createMessage.success('连接测试成功')
      } else {
        createMessage.error('连接测试失败')
      }
    } catch (error) {
      createMessage.error('连接测试失败')
    } finally {
      testingModels.value.delete(modelId)
    }
  }

  // 处理表单提交
  const handleFormSubmit = async (modelData: Omit<AIModelConfig, 'id'>) => {
    try {
      if (editingModel.value) {
        // 编辑模式
        await aiSettingsStore.updateModel(editingModel.value.id, modelData)
        createMessage.success('模型更新成功')
      } else {
        // 添加模式
        const newModel: AIModelConfig = {
          ...modelData,
          id: Date.now().toString(),
        }
        await aiSettingsStore.addModel(newModel)
        createMessage.success('模型添加成功')
      }
      showAddForm.value = false
      editingModel.value = null
    } catch (error) {
      createMessage.error('操作失败')
    }
  }

  // 处理表单取消
  const handleFormCancel = () => {
    showAddForm.value = false
    editingModel.value = null
  }

  // 获取提供商显示名称
  const getProviderName = (provider: string) => {
    const names: Record<string, string> = {
      openAI: 'OpenAI',
      claude: 'Claude',
      custom: '自定义',
    }
    return names[provider] || provider
  }
</script>

<template>
  <div class="ai-model-config">
    <!-- 页面标题和操作 -->
    <div class="section-header">
      <x-button variant="primary" @click="handleAddModel">
        <template #icon>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="12" y1="5" x2="12" y2="19" />
            <line x1="5" y1="12" x2="19" y2="12" />
          </svg>
        </template>
        添加模型
      </x-button>
    </div>

    <!-- 模型列表 -->
    <div class="models-list">
      <!-- 加载状态 -->
      <div v-if="loading" class="loading-state">
        <div class="loading-spinner"></div>
        <p>加载中...</p>
      </div>

      <!-- 空状态 -->
      <div v-else-if="models.length === 0" class="empty-state">
        <div class="empty-icon">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path
              d="M9.5 2A2.5 2.5 0 0 1 12 4.5v15a2.5 2.5 0 0 1-4.96.44 2.5 2.5 0 0 1-2.96-3.08 3 3 0 0 1-.34-5.58 2.5 2.5 0 0 1 1.32-4.24 2.5 2.5 0 0 1 1.98-3A2.5 2.5 0 0 1 9.5 2Z"
            />
            <path
              d="M14.5 2A2.5 2.5 0 0 0 12 4.5v15a2.5 2.5 0 0 0 4.96.44 2.5 2.5 0 0 0 2.96-3.08 3 3 0 0 0 .34-5.58 2.5 2.5 0 0 0-1.32-4.24 2.5 2.5 0 0 0-1.98-3A2.5 2.5 0 0 0 14.5 2Z"
            />
          </svg>
        </div>
        <h3 class="empty-title">暂无AI模型</h3>
        <p class="empty-description">点击"添加模型"按钮开始配置您的第一个AI模型</p>
      </div>

      <div v-else class="model-cards">
        <div
          v-for="model in models"
          :key="model.id"
          class="model-card"
          :class="{
            default: model.isDefault,
          }"
          @click="handleSetDefault(model.id, model.isDefault)"
        >
          <!-- 模型头部 -->
          <div class="model-header">
            <div class="model-info">
              <div class="model-name">{{ model.name }}</div>
              <div class="model-provider">{{ getProviderName(model.provider) }}</div>
            </div>
            <div class="option-radio">
              <div class="radio-button" :class="{ checked: model.isDefault }">
                <div class="radio-dot"></div>
              </div>
            </div>
          </div>

          <!-- 模型详情 -->
          <div class="model-details">
            <div class="detail-item">
              <span class="detail-label">模型:</span>
              <span class="detail-value">{{ model.model }}</span>
            </div>
            <div class="detail-item">
              <span class="detail-label">API地址:</span>
              <span class="detail-value">{{ model.apiUrl }}</span>
            </div>
          </div>

          <!-- 模型操作 -->
          <div class="model-actions">
            <x-button
              variant="secondary"
              size="small"
              :loading="testingModels.has(model.id)"
              @click.stop="handleTestConnection(model.id)"
            >
              {{ testingModels.has(model.id) ? '测试中...' : '测试连接' }}
            </x-button>
            <x-button variant="secondary" size="small" @click.stop="handleEditModel(model)">编辑</x-button>
            <x-button variant="danger" size="small" @click.stop="handleDeleteModel(model.id)">删除</x-button>
          </div>
        </div>
      </div>
    </div>

    <!-- 模型表单弹窗 -->
    <AIModelForm v-if="showAddForm" :model="editingModel" @submit="handleFormSubmit" @cancel="handleFormCancel" />
  </div>
</template>

<style scoped>
  .ai-model-config {
    width: 100%;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: var(--spacing-xl);
    gap: var(--spacing-lg);
  }

  .header-content {
    flex: 1;
  }

  .section-title {
    font-size: var(--font-size-lg);
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 var(--spacing-xs) 0;
  }

  .section-description {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.5;
  }

  .empty-state {
    text-align: center;
    padding: var(--spacing-xl) var(--spacing-lg);
    color: var(--text-secondary);
  }

  .empty-icon {
    margin-bottom: var(--spacing-md);
    opacity: 0.5;
  }

  .empty-title {
    font-size: var(--font-size-md);
    font-weight: 500;
    color: var(--text-primary);
    margin: 0 0 var(--spacing-xs) 0;
  }

  .empty-description {
    font-size: var(--font-size-sm);
    margin: 0;
    line-height: 1.5;
  }

  .model-cards {
    display: grid;
    gap: var(--spacing-md);
  }

  .model-card {
    background-color: var(--color-background);
    border: 2px solid var(--border-color);
    border-radius: var(--border-radius-lg);
    padding: var(--spacing-sm) var(--spacing-md);
    transition: all 0.2s ease;
  }

  .model-card:hover {
    border-color: var(--border-color-hover);
    background-color: var(--color-background-hover);
  }

  .model-card.default {
    border-color: var(--color-primary);
    background-color: var(--color-primary-alpha);
  }

  .model-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: var(--spacing-xs);
  }

  .model-name {
    font-size: var(--font-size-md);
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 1px;
  }

  .model-provider {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .model-details {
    margin-bottom: var(--spacing-sm);
    position: relative;
  }

  .detail-item {
    display: flex;
    gap: var(--spacing-xs);
    margin-bottom: 2px;
    font-size: var(--font-size-xs);
  }

  .detail-label {
    color: var(--text-secondary);
    min-width: 60px;
  }

  .detail-value {
    color: var(--text-primary);
    word-break: break-all;
  }

  .default-badge {
    position: absolute;
    top: 0;
    right: 0;
    background-color: var(--color-primary);
    color: white;
    font-size: var(--font-size-xs);
    padding: 2px var(--spacing-xs);
    border-radius: var(--border-radius);
    font-weight: 500;
  }

  .model-actions {
    display: flex;
    gap: var(--spacing-xs);
    flex-wrap: wrap;
    margin-top: var(--spacing-xs);
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

  /* 加载状态 */
  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--spacing-xl);
    color: var(--text-secondary);
  }

  .loading-spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border-color);
    border-top: 2px solid var(--color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: var(--spacing-md);
  }

  @keyframes spin {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }
</style>
