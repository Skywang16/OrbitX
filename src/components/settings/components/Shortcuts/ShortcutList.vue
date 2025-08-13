<template>
  <div class="shortcut-list">
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>加载快捷键配置...</span>
    </div>

    <div v-else-if="items.length === 0" class="empty-state">
      <i class="icon-keyboard"></i>
      <h3>暂无快捷键</h3>
      <p>点击"添加快捷键"按钮来创建您的第一个快捷键</p>
    </div>

    <div v-else class="shortcut-table">
      <div class="table-header">
        <div class="col-category">类别</div>
        <div class="col-shortcut">快捷键</div>
        <div class="col-action">动作</div>
        <div class="col-status">状态</div>
        <div class="col-operations">操作</div>
      </div>

      <div
        v-for="item in items"
        :key="`${item.category}-${item.index}`"
        class="table-row"
        :class="{
          'has-conflict': item.hasConflict,
          'has-error': item.hasError,
        }"
      >
        <div class="col-category">
          <span class="category-badge" :class="`category-${item.category.toLowerCase()}`">
            {{ getCategoryLabel(item.category) }}
          </span>
        </div>

        <div class="col-shortcut">
          <code class="shortcut-display">
            {{ formatShortcut(item.binding) }}
          </code>
        </div>

        <div class="col-action">
          <span class="action-text">
            {{ getActionText(item.binding.action) }}
          </span>
        </div>

        <div class="col-status">
          <div class="status-indicators">
            <span v-if="item.hasConflict" class="status-badge status-conflict" title="存在冲突">
              <i class="icon-warning"></i>
              冲突
            </span>
            <span v-if="item.hasError" class="status-badge status-error" title="验证错误">
              <i class="icon-error"></i>
              错误
            </span>
            <span v-if="!item.hasConflict && !item.hasError" class="status-badge status-ok" title="正常">
              <i class="icon-check"></i>
              正常
            </span>
          </div>
        </div>

        <div class="col-operations">
          <div class="operation-buttons">
            <button class="btn-icon" title="编辑" @click="handleEdit(item)">
              <i class="icon-edit"></i>
            </button>
            <button class="btn-icon" title="复制" @click="handleDuplicate(item)">
              <i class="icon-copy"></i>
            </button>
            <button class="btn-icon btn-danger" title="删除" @click="handleDelete(item)">
              <i class="icon-delete"></i>
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { useShortcutFormatter } from '@/composables/useShortcuts'
  import { ShortcutListItem, ShortcutActionEvent, ShortcutActionType } from './types'
  import type { ShortcutAction } from '@/api/shortcuts/types'

  interface Props {
    items: ShortcutListItem[]
    loading?: boolean
  }

  interface Emits {
    (e: 'action', event: ShortcutActionEvent): void
  }

  withDefaults(defineProps<Props>(), {
    loading: false,
  })

  const emit = defineEmits<Emits>()

  const { formatShortcut } = useShortcutFormatter()

  // 方法
  const getCategoryLabel = (category: string): string => {
    const labels: Record<string, string> = {
      Global: '全局',
      Terminal: '终端',
      Custom: '自定义',
    }
    return labels[category] || category
  }

  const getActionText = (action: ShortcutAction): string => {
    if (typeof action === 'string') {
      return action
    }

    const text = action.text ? ` (${action.text})` : ''
    return `${action.action_type}${text}`
  }

  const handleEdit = (item: ShortcutListItem) => {
    emit('action', {
      type: ShortcutActionType.Edit,
      item,
    })
  }

  const handleDuplicate = (item: ShortcutListItem) => {
    emit('action', {
      type: ShortcutActionType.Duplicate,
      item,
    })
  }

  const handleDelete = (item: ShortcutListItem) => {
    emit('action', {
      type: ShortcutActionType.Delete,
      item,
    })
  }
</script>

<style scoped>
  .shortcut-list {
    width: 100%;
  }

  .loading-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 20px;
    color: var(--text-400);
  }

  .loading-state {
    gap: 12px;
  }

  .empty-state {
    gap: 16px;
  }

  .empty-state i {
    font-size: 48px;
    opacity: 0.5;
  }

  .empty-state h3 {
    margin: 0;
    font-size: 18px;
    font-weight: 500;
  }

  .empty-state p {
    margin: 0;
    font-size: 14px;
    opacity: 0.8;
  }

  .spinner {
    width: 24px;
    height: 24px;
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

  .shortcut-table {
    width: 100%;
    border-radius: 8px;
    overflow: hidden;
    border: 1px solid var(--border);
  }

  .table-header,
  .table-row {
    display: grid;
    grid-template-columns: 100px 200px 1fr 120px 120px;
    gap: 16px;
    padding: 12px 16px;
    align-items: center;
  }

  .table-header {
    background: var(--bg-tertiary);
    font-weight: 500;
    font-size: 14px;
    color: var(--text-400);
    border-bottom: 1px solid var(--border);
  }

  .table-row {
    background: var(--bg-400);
    border-bottom: 1px solid var(--border);
    transition: background-color 0.2s;
  }

  .table-row:hover {
    background: var(--bg-hover);
  }

  .table-row:last-child {
    border-bottom: none;
  }

  .table-row.has-conflict {
    border-left: 3px solid var(--warning);
  }

  .table-row.has-error {
    border-left: 3px solid var(--error);
  }

  .category-badge {
    display: inline-block;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 12px;
    font-weight: 500;
    text-transform: uppercase;
  }

  .category-global {
    background: var(--primary-bg);
    color: var(--primary);
  }

  .category-terminal {
    background: var(--success-bg);
    color: var(--success);
  }

  .category-custom {
    background: var(--info-bg);
    color: var(--info);
  }

  .shortcut-display {
    background: var(--bg-tertiary);
    padding: 4px 8px;
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 13px;
    border: 1px solid var(--border);
  }

  .action-text {
    font-size: 14px;
    color: var(--text-200);
  }

  .status-indicators {
    display: flex;
    gap: 4px;
  }

  .status-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 11px;
    font-weight: 500;
  }

  .status-ok {
    background: var(--success-bg);
    color: var(--success);
  }

  .status-conflict {
    background: var(--warning-bg);
    color: var(--warning);
  }

  .status-error {
    background: var(--error-bg);
    color: var(--error);
  }

  .operation-buttons {
    display: flex;
    gap: 4px;
  }

  .btn-icon {
    width: 32px;
    height: 32px;
    border: none;
    background: none;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-400);
    transition: all 0.2s;
  }

  .btn-icon:hover {
    background: var(--bg-tertiary);
    color: var(--text-200);
  }

  .btn-icon.btn-danger:hover {
    background: var(--error-bg);
    color: var(--error);
  }

  @media (max-width: 768px) {
    .table-header,
    .table-row {
      grid-template-columns: 1fr;
      gap: 8px;
    }

    .col-category,
    .col-shortcut,
    .col-action,
    .col-status,
    .col-operations {
      display: flex;
      justify-content: space-between;
      align-items: center;
    }

    .col-category::before {
      content: '类别: ';
    }
    .col-shortcut::before {
      content: '快捷键: ';
    }
    .col-action::before {
      content: '动作: ';
    }
    .col-status::before {
      content: '状态: ';
    }
    .col-operations::before {
      content: '操作: ';
    }

    .table-header {
      display: none;
    }
  }
</style>
