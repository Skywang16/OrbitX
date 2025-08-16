<template>
  <div class="shortcut-statistics">
    <div class="stats-grid">
      <div class="stat-item">
        <div class="stat-value">{{ statistics?.total_count || 0 }}</div>
        <div class="stat-label">总快捷键</div>
      </div>

      <div class="stat-item">
        <div class="stat-value">{{ statistics?.global_count || 0 }}</div>
        <div class="stat-label">全局</div>
      </div>

      <div class="stat-item">
        <div class="stat-value">{{ statistics?.terminal_count || 0 }}</div>
        <div class="stat-label">终端</div>
      </div>

      <div class="stat-item">
        <div class="stat-value">{{ statistics?.custom_count || 0 }}</div>
        <div class="stat-label">自定义</div>
      </div>

      <div class="stat-item" :class="{ 'stat-warning': conflictCount > 0 }">
        <div class="stat-value">{{ conflictCount }}</div>
        <div class="stat-label">冲突</div>
      </div>

      <div class="stat-item" :class="{ 'stat-error': errorCount > 0 }">
        <div class="stat-value">{{ errorCount }}</div>
        <div class="stat-label">错误</div>
      </div>
    </div>

    <div v-if="loading" class="stats-loading">
      <div class="spinner"></div>
      <span>加载统计信息...</span>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { computed } from 'vue'
  import { useShortcuts } from '@/composables/useShortcuts'
  import { useShortcutStore } from '@/stores/shortcuts'

  // 组合式API
  const { statistics, loading } = useShortcuts()
  const store = useShortcutStore()
  const lastConflictDetection = computed(() => store.state.lastConflictDetection)
  const lastValidation = computed(() => store.state.lastValidation)

  // 计算属性
  const conflictCount = computed(() => {
    return lastConflictDetection.value?.conflicts.length || 0
  })

  const errorCount = computed(() => {
    return lastValidation.value?.errors.length || 0
  })
</script>

<style scoped>
  .shortcut-statistics {
    position: relative;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(80px, 1fr));
    gap: 16px;
  }

  .stat-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 12px;
    background: var(--bg-secondary);
    border-radius: 6px;
    border: 1px solid var(--border);
    transition: all 0.2s;
  }

  .stat-item:hover {
    background: var(--bg-hover);
  }

  .stat-item.stat-warning {
    border-color: var(--warning);
    background: var(--warning-bg);
  }

  .stat-item.stat-error {
    border-color: var(--error);
    background: var(--error-bg);
  }

  .stat-value {
    font-size: 24px;
    font-weight: 600;
    color: var(--text-200);
    line-height: 1;
  }

  .stat-warning .stat-value {
    color: var(--warning);
  }

  .stat-error .stat-value {
    color: var(--error);
  }

  .stat-label {
    font-size: 12px;
    color: var(--text-400);
    margin-top: 4px;
    text-align: center;
  }

  .stats-loading {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    background: var(--bg-500);
    border-radius: 6px;
    font-size: 14px;
    color: var(--text-400);
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid var(--border);
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

  @media (max-width: 768px) {
    .stats-grid {
      grid-template-columns: repeat(3, 1fr);
      gap: 12px;
    }

    .stat-item {
      padding: 8px;
    }

    .stat-value {
      font-size: 20px;
    }

    .stat-label {
      font-size: 11px;
    }
  }
</style>
