<template>
  <div class="shortcut-editor-overlay" @click.self="$emit('cancel')">
    <div class="shortcut-editor">
      <div class="editor-header">
        <h3>{{ getTitle() }}</h3>
        <button class="btn-close" @click="$emit('cancel')">
          <i class="icon-close"></i>
        </button>
      </div>

      <div class="editor-content">
        <div class="form-group">
          <label>类别</label>
          <select v-model="currentCategory" class="form-control">
            <option value="Global">全局</option>
            <option value="Terminal">终端</option>
            <option value="Custom">自定义</option>
          </select>
        </div>

        <div class="form-group">
          <label>快捷键</label>
          <div class="shortcut-input">
            <input
              ref="shortcutInput"
              v-model="shortcutDisplay"
              class="form-control"
              placeholder="按下快捷键组合..."
              readonly
              @keydown="handleKeyDown"
              @focus="startCapture"
              @blur="stopCapture"
            />
            <button class="btn-clear" @click="clearShortcut">
              <i class="icon-clear"></i>
            </button>
          </div>
        </div>

        <div class="form-group">
          <label>动作类型</label>
          <select v-model="actionType" class="form-control">
            <option value="simple">简单动作</option>
            <option value="complex">复杂动作</option>
          </select>
        </div>

        <div v-if="actionType === 'simple'" class="form-group">
          <label>动作</label>
          <input v-model="simpleAction" class="form-control" placeholder="输入动作名称..." />
        </div>

        <div v-else class="form-group">
          <label>动作类型</label>
          <select v-model="complexActionType" class="form-control">
            <option value="send_text">发送文本</option>
            <option value="run_command">运行命令</option>
            <option value="execute_script">执行脚本</option>
            <option value="open_url">打开URL</option>
            <option value="copy_to_clipboard">复制到剪贴板</option>
            <option value="paste_from_clipboard">从剪贴板粘贴</option>
          </select>
        </div>

        <div v-if="actionType === 'complex' && needsText" class="form-group">
          <label>文本内容</label>
          <textarea v-model="actionText" class="form-control" rows="3" placeholder="输入文本内容..."></textarea>
        </div>
      </div>

      <div class="editor-footer">
        <button class="btn btn-secondary" @click="$emit('cancel')">取消</button>
        <button class="btn btn-primary" @click="handleSave" :disabled="!isValid">保存</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, computed, onMounted } from 'vue'
  import { ShortcutEditorOptions, ShortcutEditorMode } from './types'
  import { ShortcutBinding, ShortcutCategory, ShortcutAction } from '@/api/shortcuts/types'

  interface Props {
    options: ShortcutEditorOptions
  }

  interface Emits {
    (e: 'save', shortcut: ShortcutBinding, category: ShortcutCategory, index?: number): void
    (e: 'cancel'): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()

  // 响应式状态
  const currentCategory = ref<ShortcutCategory>(ShortcutCategory.Global)
  const currentKey = ref('')
  const currentModifiers = ref<string[]>([])
  const actionType = ref<'simple' | 'complex'>('simple')
  const simpleAction = ref('')
  const complexActionType = ref('send_text')
  const actionText = ref('')
  const capturing = ref(false)

  const shortcutInput = ref<HTMLInputElement>()

  // 计算属性
  const shortcutDisplay = computed(() => {
    if (!currentKey.value) return ''
    const parts = [...currentModifiers.value, currentKey.value]
    return parts.join(' + ')
  })

  const needsText = computed(() => {
    return actionType.value === 'complex' && complexActionType.value === 'send_text'
  })

  const isValid = computed(() => {
    if (!currentKey.value) return false
    if (actionType.value === 'simple' && !simpleAction.value) return false
    if (actionType.value === 'complex' && needsText.value && !actionText.value) return false
    return true
  })

  // 方法
  const getTitle = (): string => {
    switch (props.options.mode) {
      case ShortcutEditorMode.Add:
        return '添加快捷键'
      case ShortcutEditorMode.Edit:
        return '编辑快捷键'
      case ShortcutEditorMode.View:
        return '查看快捷键'
      default:
        return '快捷键编辑器'
    }
  }

  const handleKeyDown = (event: KeyboardEvent) => {
    if (!capturing.value) return

    event.preventDefault()
    event.stopPropagation()

    const modifiers: string[] = []
    if (event.ctrlKey) modifiers.push('ctrl')
    if (event.metaKey) modifiers.push('cmd')
    if (event.altKey) modifiers.push('alt')
    if (event.shiftKey) modifiers.push('shift')

    let key = event.key
    if (key === ' ') key = 'Space'
    if (key === 'Control' || key === 'Meta' || key === 'Alt' || key === 'Shift') return

    currentKey.value = key
    currentModifiers.value = modifiers
  }

  const startCapture = () => {
    capturing.value = true
  }

  const stopCapture = () => {
    capturing.value = false
  }

  const clearShortcut = () => {
    currentKey.value = ''
    currentModifiers.value = []
  }

  const handleSave = () => {
    if (!isValid.value) return

    const action: ShortcutAction =
      actionType.value === 'simple'
        ? simpleAction.value
        : {
            action_type: complexActionType.value,
            text: needsText.value ? actionText.value : undefined,
          }

    const shortcut: ShortcutBinding = {
      key: currentKey.value,
      modifiers: currentModifiers.value,
      action,
    }

    emit('save', shortcut, currentCategory.value, props.options.initialIndex)
  }

  // 初始化
  onMounted(() => {
    if (props.options.initialShortcut) {
      const shortcut = props.options.initialShortcut
      currentKey.value = shortcut.key
      currentModifiers.value = [...shortcut.modifiers]

      if (typeof shortcut.action === 'string') {
        actionType.value = 'simple'
        simpleAction.value = shortcut.action
      } else {
        actionType.value = 'complex'
        complexActionType.value = shortcut.action.action_type
        actionText.value = shortcut.action.text || ''
      }
    }

    if (props.options.initialCategory) {
      currentCategory.value = props.options.initialCategory
    }
  })
</script>

<style scoped>
  .shortcut-editor-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--color-selection);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .shortcut-editor {
    background: var(--bg-400);
    border-radius: 8px;
    box-shadow: var(--shadow-lg);
    width: 90%;
    max-width: 500px;
    max-height: 90vh;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .editor-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px;
    border-bottom: 1px solid var(--border);
  }

  .editor-header h3 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
  }

  .btn-close {
    background: none;
    border: none;
    color: var(--text-400);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
  }

  .btn-close:hover {
    background: var(--bg-hover);
    color: var(--text-200);
  }

  .editor-content {
    padding: 20px;
    flex: 1;
    overflow-y: auto;
  }

  .form-group {
    margin-bottom: 16px;
  }

  .form-group label {
    display: block;
    margin-bottom: 6px;
    font-weight: 500;
    color: var(--text-200);
  }

  .form-control {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-secondary);
    color: var(--text-200);
    font-size: 14px;
  }

  .form-control:focus {
    outline: none;
    border-color: var(--primary);
    box-shadow: 0 0 0 2px var(--primary-bg);
  }

  .shortcut-input {
    position: relative;
  }

  .shortcut-input .form-control {
    padding-right: 40px;
    font-family: var(--font-mono);
  }

  .btn-clear {
    position: absolute;
    right: 8px;
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    color: var(--text-400);
    cursor: pointer;
    padding: 4px;
    border-radius: 3px;
  }

  .btn-clear:hover {
    background: var(--bg-hover);
    color: var(--text-200);
  }

  .editor-footer {
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
    color: var(--text-200);
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
