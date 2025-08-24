<script setup lang="ts">
  import type { AIModelConfig } from '@/types'
  import { confirm, createMessage } from '@/ui'
  import { handleError } from '@/utils/errorHandler'
  import { computed, onMounted, ref } from 'vue'
  import AIModelForm from './AIModelForm.vue'
  import { useAISettingsStore } from './store'

  // 使用AI设置store
  const aiSettingsStore = useAISettingsStore()

  // 响应式数据
  const showAddForm = ref(false)
  const editingModel = ref<AIModelConfig | null>(null)

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
        createMessage.error(handleError(error, '删除失败'))
      }
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
      createMessage.error(handleError(error, '操作失败'))
    }
  }

  // 处理表单取消
  const handleFormCancel = () => {
    showAddForm.value = false
    editingModel.value = null
  }
</script>

<template>
  <div class="ai-model-config">
    <!-- 操作按钮 -->
    <div class="action-header">
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
        <div v-for="model in models" :key="model.id" class="model-card">
          <div class="model-left">
            <div class="model-name">{{ model.name }}</div>
          </div>

          <div class="model-actions">
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

  .action-header {
    margin-bottom: 20px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--border-300);
  }

  .action-header :deep(.x-button) {
    background: var(--color-primary);
    color: white;
    border-radius: 4px;
    padding: 8px 12px;
    font-size: 13px;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .action-header :deep(.x-button:hover) {
    background: var(--color-primary-hover);
  }

  .section-title {
    font-size: 20px;
    color: var(--text-100);
    margin-bottom: 8px;
  }

  .section-description {
    font-size: 13px;
    color: var(--text-400);
  }

  .empty-state {
    text-align: center;
    padding: 48px 24px;
    color: var(--text-400);
    background: var(--bg-500);
    border-radius: 4px;
  }

  .empty-icon {
    margin-bottom: 16px;
    color: var(--text-300);
  }

  .empty-title {
    font-size: 16px;
    color: var(--text-200);
    margin-bottom: 8px;
  }

  .empty-description {
    font-size: 13px;
  }

  .model-cards {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .model-card {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--bg-500);
    border-radius: 4px;
    padding: 12px 16px;
  }

  .model-card:hover {
    background: var(--bg-400);
  }

  .model-name {
    font-size: 14px;
    color: var(--text-200);
  }

  .model-actions {
    display: flex;
    gap: 8px;
  }

  .model-actions :deep(.x-button) {
    background: transparent;
    border: 1px solid var(--border-200);
    color: var(--text-300);
    border-radius: 4px;
    padding: 4px 8px;
    font-size: 12px;
  }

  .model-actions :deep(.x-button:hover) {
    background: var(--bg-300);
    color: var(--text-200);
  }

  .model-actions :deep(.x-button[variant='danger']) {
    border-color: var(--error-border);
    color: var(--error-text);
  }

  .model-actions :deep(.x-button[variant='danger']:hover) {
    background: var(--error-bg);
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 48px 24px;
    color: var(--text-400);
    background: var(--bg-500);
    border-radius: 4px;
  }

  .loading-state p {
    font-size: 13px;
  }

  .loading-spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border-300);
    border-top: 2px solid var(--color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 12px;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
