<template>
  <div class="conflict-dialog-overlay" @click.self="$emit('close')">
    <div class="conflict-dialog">
      <div class="dialog-header">
        <h3>快捷键冲突</h3>
        <button class="btn-close" @click="$emit('close')">
          <i class="icon-close"></i>
        </button>
      </div>

      <div class="dialog-content">
        <p class="conflict-description">检测到 {{ conflicts.length }} 个快捷键冲突，请选择要保留的快捷键：</p>

        <div v-for="(conflict, index) in conflicts" :key="index" class="conflict-item">
          <div class="conflict-header">
            <h4>
              冲突组合:
              <code>{{ conflict.key_combination }}</code>
            </h4>
          </div>

          <div class="conflicting-shortcuts">
            <div
              v-for="(shortcut, shortcutIndex) in conflict.conflicting_shortcuts"
              :key="shortcutIndex"
              class="shortcut-option"
            >
              <label class="shortcut-radio">
                <input
                  type="radio"
                  :name="`conflict-${index}`"
                  :value="shortcutIndex"
                  v-model="selectedShortcuts[index]"
                />
                <div class="shortcut-info">
                  <span class="category-badge" :class="`category-${shortcut.category.toLowerCase()}`">
                    {{ getCategoryLabel(shortcut.category) }}
                  </span>
                  <span class="action-text">
                    {{ getActionText(shortcut.binding.action) }}
                  </span>
                </div>
              </label>
            </div>
          </div>
        </div>
      </div>

      <div class="dialog-footer">
        <button class="btn btn-secondary" @click="$emit('close')">取消</button>
        <button class="btn btn-primary" @click="handleResolve" :disabled="!canResolve">解决冲突</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, computed } from 'vue'
  import type { ShortcutConflict, ShortcutAction } from '@/api/shortcuts/types'

  interface Props {
    conflicts: ShortcutConflict[]
  }

  interface Emits {
    (e: 'resolve', conflictIndex: number, selectedShortcutIndex: number): void
    (e: 'close'): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()

  // 响应式状态
  const selectedShortcuts = ref<Record<number, number>>({})

  // 计算属性
  const canResolve = computed(() => {
    return props.conflicts.every((_, index) => selectedShortcuts.value[index] !== undefined)
  })

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

  const handleResolve = () => {
    // 这里应该实现冲突解决逻辑
    // 目前只是简单地关闭对话框
    emit('close')
  }
</script>

<style scoped>
  .conflict-dialog-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .conflict-dialog {
    background: var(--bg-primary);
    border-radius: 8px;
    box-shadow: var(--shadow-lg);
    width: 90%;
    max-width: 600px;
    max-height: 90vh;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .dialog-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px;
    border-bottom: 1px solid var(--border);
  }

  .dialog-header h3 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: var(--warning);
  }

  .btn-close {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
  }

  .btn-close:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .dialog-content {
    padding: 20px;
    flex: 1;
    overflow-y: auto;
  }

  .conflict-description {
    margin: 0 0 20px 0;
    color: var(--text-secondary);
  }

  .conflict-item {
    margin-bottom: 24px;
    padding: 16px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-secondary);
  }

  .conflict-header h4 {
    margin: 0 0 12px 0;
    font-size: 16px;
    font-weight: 500;
  }

  .conflict-header code {
    background: var(--bg-tertiary);
    padding: 2px 6px;
    border-radius: 3px;
    font-family: var(--font-mono);
  }

  .conflicting-shortcuts {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .shortcut-option {
    display: block;
  }

  .shortcut-radio {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px;
    border-radius: 4px;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .shortcut-radio:hover {
    background: var(--bg-hover);
  }

  .shortcut-radio input[type='radio'] {
    margin: 0;
  }

  .shortcut-info {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
  }

  .category-badge {
    display: inline-block;
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 11px;
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

  .action-text {
    font-size: 14px;
    color: var(--text-primary);
  }

  .dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding: 20px;
    border-top: 1px solid var(--border);
  }

  .btn {
    padding: 8px 16px;
    border-radius: 4px;
    border: none;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  .btn-primary {
    background: var(--primary);
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--primary-hover);
  }
</style>
