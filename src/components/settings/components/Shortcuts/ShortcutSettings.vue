<template>
  <div class="shortcut-settings">
    <!-- 错误状态 -->
    <div v-if="error" class="error-state">
      <div class="error-icon">⚠️</div>
      <p>加载快捷键设置失败: {{ error }}</p>
      <x-button variant="primary" @click="initialize()">重试</x-button>
    </div>

    <!-- 正常内容 -->
    <template v-else>
      <div class="settings-card">
        <div class="card-header">
          <h3>快捷键设置</h3>
          <p class="card-description">配置和管理应用程序的快捷键</p>
        </div>

        <div class="card-content">
          <!-- 统计信息 -->
          <div v-if="config" class="stats-section">
            <div class="stat-item">
              <span class="stat-label">全局快捷键</span>
              <span class="stat-value">{{ config.global.length }}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">终端快捷键</span>
              <span class="stat-value">{{ config.terminal.length }}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">自定义快捷键</span>
              <span class="stat-value">{{ config.custom.length }}</span>
            </div>
          </div>

          <!-- 冲突警告 -->
          <div v-if="hasConflicts" class="alert alert-warning">
            <span class="alert-icon">⚠️</span>
            <span>检测到 {{ conflictCount }} 个快捷键冲突</span>
            <x-button size="sm" variant="warning" @click="showConflictDialog = true">查看详情</x-button>
          </div>

          <!-- 快捷键列表 -->
          <div class="shortcuts-list">
            <div v-if="loading" class="loading-state">
              <div class="loading-spinner"></div>
              <span>加载中...</span>
            </div>
            <div v-else-if="!config" class="empty-state">
              <span>暂无快捷键配置</span>
            </div>
            <div v-else>
              <!-- 全局快捷键 -->
              <div v-if="config.global.length > 0" class="shortcut-category">
                <h4>全局快捷键</h4>
                <div class="shortcut-items">
                  <div v-for="(shortcut, index) in config.global" :key="`global-${index}`" class="shortcut-item">
                    <div class="shortcut-key">
                      <span v-for="modifier in shortcut.modifiers" :key="modifier" class="modifier">
                        {{ modifier }}
                      </span>
                      <span class="key">{{ shortcut.key }}</span>
                    </div>
                    <div class="shortcut-action">{{ getActionName(shortcut.action) }}</div>
                  </div>
                </div>
              </div>

              <!-- 终端快捷键 -->
              <div v-if="config.terminal.length > 0" class="shortcut-category">
                <h4>终端快捷键</h4>
                <div class="shortcut-items">
                  <div v-for="(shortcut, index) in config.terminal" :key="`terminal-${index}`" class="shortcut-item">
                    <div class="shortcut-key">
                      <span v-for="modifier in shortcut.modifiers" :key="modifier" class="modifier">
                        {{ modifier }}
                      </span>
                      <span class="key">{{ shortcut.key }}</span>
                    </div>
                    <div class="shortcut-action">{{ getActionName(shortcut.action) }}</div>
                  </div>
                </div>
              </div>

              <!-- 自定义快捷键 -->
              <div v-if="config.custom.length > 0" class="shortcut-category">
                <h4>自定义快捷键</h4>
                <div class="shortcut-items">
                  <div v-for="(shortcut, index) in config.custom" :key="`custom-${index}`" class="shortcut-item">
                    <div class="shortcut-key">
                      <span v-for="modifier in shortcut.modifiers" :key="modifier" class="modifier">
                        {{ modifier }}
                      </span>
                      <span class="key">{{ shortcut.key }}</span>
                    </div>
                    <div class="shortcut-action">{{ getActionName(shortcut.action) }}</div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- 操作按钮 -->
          <div class="actions-section">
            <x-button variant="outline" @click="handleReset" :disabled="loading">重置到默认</x-button>
          </div>
        </div>
      </div>
    </template>

    <!-- 冲突详情对话框 -->
    <ShortcutConflictDialog
      v-if="showConflictDialog"
      :conflicts="conflicts"
      @resolve="handleResolveConflict"
      @close="showConflictDialog = false"
    />
  </div>
</template>

<script setup lang="ts">
  import { ref, computed, onMounted } from 'vue'
  import { handleErrorWithMessage } from '@/utils/errorHandler'
  import { useShortcuts } from '@/composables/useShortcuts'
  import { useNotificationStore } from '@/stores/Notification'
  import { useShortcutStore } from '@/stores/shortcuts'
  import { getActionDisplayName } from '@/shortcuts/constants'
  import ShortcutEditor from './ShortcutEditor.vue'
  import ShortcutConflictDialog from './ShortcutConflictDialog.vue'
  import type { ShortcutBinding } from '@/api/shortcuts/types'
  import { ShortcutCategory } from '@/api/shortcuts/types'

  // 组合式API
  const {
    config,
    loading,
    error,
    hasConflicts,

    initialize,
    addShortcut,
    removeShortcut,
    updateShortcut,
    resetToDefaults,
    clearError,
  } = useShortcuts()

  const store = useShortcutStore()
  const lastConflictDetection = computed(() => store.lastConflictDetection)

  // 响应式状态
  const showConflictDialog = ref(false)

  // 计算属性
  const conflicts = computed(() => lastConflictDetection.value?.conflicts || [])
  const conflictCount = computed(() => conflicts.value.length)

  // 方法
  const getActionName = (action: string | { action_type: string }): string => {
    if (typeof action === 'string') {
      return getActionDisplayName(action)
    }
    if (typeof action === 'object' && action.action_type) {
      return getActionDisplayName(action.action_type)
    }
    return 'unknown'
  }

  const handleReset = async () => {
    if (await confirm('确定要重置所有快捷键到默认配置吗？此操作不可撤销。')) {
      try {
        await resetToDefaults()
      } catch (error) {
        handleErrorWithMessage(error, '重置配置失败')
      }
    }
  }

  const handleResolveConflict = (_conflictIndex: number) => {
    // 处理冲突解决逻辑
    showConflictDialog.value = false
  }

  // 生命周期
  onMounted(async () => {
    // 使用新的初始化检查机制
    if (!store.initialized && !loading.value) {
      try {
        await initialize()
      } catch (err) {
        handleErrorWithMessage(err, '快捷键设置初始化失败')
      }
    }
  })
</script>

<style scoped>
  .shortcut-settings {
    padding: var(--spacing-lg);
  }

  .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--spacing-md);
    padding: var(--spacing-xl);
    text-align: center;
  }

  .error-icon {
    font-size: 2rem;
  }

  .settings-card {
    background: var(--bg-300);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-300);
    overflow: hidden;
  }

  .card-header {
    padding: var(--spacing-lg);
    border-bottom: 1px solid var(--border-300);
  }

  .card-header h3 {
    margin: 0 0 var(--spacing-xs) 0;
    font-size: var(--font-size-lg);
    font-weight: 600;
    color: var(--text-100);
  }

  .card-description {
    margin: 0;
    color: var(--text-400);
    font-size: var(--font-size-sm);
  }

  .card-content {
    padding: var(--spacing-lg);
  }

  .stats-section {
    display: flex;
    gap: var(--spacing-lg);
    margin-bottom: var(--spacing-lg);
    padding: var(--spacing-md);
    background: var(--bg-400);
    border-radius: var(--radius-md);
  }

  .stat-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--spacing-xs);
  }

  .stat-label {
    font-size: var(--font-size-xs);
    color: var(--text-400);
  }

  .stat-value {
    font-size: var(--font-size-lg);
    font-weight: 600;
    color: var(--text-100);
  }

  .alert {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-md);
    border-radius: var(--radius-md);
    margin-bottom: var(--spacing-md);
  }

  .alert-warning {
    background: var(--warning-bg);
    color: var(--warning-text);
    border: 1px solid var(--warning-border);
  }

  .alert-icon {
    font-size: var(--font-size-md);
  }

  .shortcuts-list {
    margin-bottom: var(--spacing-lg);
  }

  .loading-state,
  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-xl);
    color: var(--text-400);
  }

  .loading-spinner {
    width: 16px;
    height: 16px;
    border: 2px solid var(--border-300);
    border-top: 2px solid var(--primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }

  .shortcut-category {
    margin-bottom: var(--spacing-lg);
  }

  .shortcut-category h4 {
    margin: 0 0 var(--spacing-md) 0;
    font-size: var(--font-size-md);
    font-weight: 600;
    color: var(--text-200);
  }

  .shortcut-items {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
  }

  .shortcut-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--spacing-md);
    background: var(--bg-400);
    border-radius: var(--radius-md);
    border: 1px solid var(--border-300);
  }

  .shortcut-key {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
  }

  .modifier,
  .key {
    padding: 2px 6px;
    background: var(--bg-500);
    border: 1px solid var(--border-300);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-xs);
    font-family: var(--font-mono);
    color: var(--text-200);
  }

  .key {
    background: var(--primary-alpha);
    color: var(--primary);
    border-color: var(--primary-alpha);
  }

  .shortcut-action {
    color: var(--text-300);
    font-size: var(--font-size-sm);
  }

  .actions-section {
    display: flex;
    justify-content: flex-end;
    gap: var(--spacing-md);
    padding-top: var(--spacing-md);
    border-top: 1px solid var(--border-300);
  }
</style>
