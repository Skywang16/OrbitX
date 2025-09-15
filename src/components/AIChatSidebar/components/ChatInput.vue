<script setup lang="ts">
  import { computed, ref, nextTick, onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useTerminalSelection } from '@/composables/useTerminalSelection'
  import { useTabManagerStore } from '@/stores/TabManager'
  import { useTerminalStore } from '@/stores/Terminal'
  import { TabType } from '@/types'
  import { homeDir } from '@tauri-apps/api/path'
  import TerminalSelectionTag from './TerminalSelectionTag.vue'
  import TerminalTabTag from './TerminalTabTag.vue'
  import InputPopover from '@/components/ui/InputPopover.vue'
  import CkIndexContent from './CkIndexContent.vue'
  import CircularProgress from '@/components/ui/CircularProgress.vue'
  import { ckApi } from '@/api'

  // Props定义
  interface Props {
    modelValue: string
    placeholder?: string
    loading?: boolean

    canSend?: boolean
    selectedModel?: string | null
    modelOptions?: Array<{ label: string; value: string | number }>
    chatMode?: 'chat' | 'agent'
  }

  // Emits定义
  interface Emits {
    (e: 'update:modelValue', value: string): void
    (e: 'send'): void
    (e: 'stop'): void
    (e: 'update:selectedModel', value: string | null): void
    (e: 'model-change', value: string | null): void
    (e: 'mode-change', mode: 'chat' | 'agent'): void
  }

  const props = withDefaults(defineProps<Props>(), {
    placeholder: '',
    loading: false,

    canSend: false,
    selectedModel: null,
    modelOptions: () => [],
    chatMode: 'chat',
  })

  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  // 响应式引用
  const inputTextarea = ref<HTMLTextAreaElement>()

  // 终端选择管理
  const terminalSelection = useTerminalSelection()

  // Tab管理器
  const tabManagerStore = useTabManagerStore()
  const terminalStore = useTerminalStore()

  // 计算当前是否在设置tab
  const isInSettingsTab = computed(() => {
    return tabManagerStore.activeTab?.type === TabType.SETTINGS
  })

  // 用户主目录路径
  const homePath = ref<string>('')

  // 获取当前解析的路径
  const resolvedPath = ref<string>('.')

  // 标准化路径
  const normalize = (p: string) => p.replace(/\\/g, '/').replace(/\/$/, '')

  // 计算是否可以构建索引（与CkIndexContent保持一致的逻辑）
  const canBuild = computed(() => {
    const pRaw = resolvedPath.value
    if (!pRaw) return false
    const p = normalize(pRaw)
    if (p === '.' || p === '~' || p === '/' || /^[A-Za-z]:$/.test(p)) return false
    if (homePath.value) {
      const h = normalize(homePath.value)
      if (p === h) return false
    }
    return true
  })

  // 计算属性
  const inputValue = computed({
    get: () => props.modelValue,
    set: (value: string) => emit('update:modelValue', value),
  })

  // 模式选项数据
  const modeOptions = computed(() => [
    {
      label: 'Chat',
      value: 'chat',
    },
    {
      label: 'Agent',
      value: 'agent',
    },
  ])

  // 方法
  /**
   * 处理键盘事件
   */
  const handleKeydown = (event: KeyboardEvent) => {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault()
      handleButtonClick()
    }
  }

  /**
   * 调整输入框高度
   */
  const adjustTextareaHeight = () => {
    if (!inputTextarea.value) return

    const textarea = inputTextarea.value
    textarea.style.height = 'auto'

    const scrollHeight = textarea.scrollHeight
    const maxHeight = 100
    const minHeight = 32
    const newHeight = Math.max(minHeight, Math.min(scrollHeight, maxHeight))

    textarea.style.height = newHeight + 'px'
    textarea.style.overflowY = scrollHeight > maxHeight ? 'auto' : 'hidden'
  }

  /**
   * 处理发送/停止按钮点击
   */
  const handleButtonClick = () => {
    if (props.loading) {
      emit('stop')
    } else if (props.canSend) {
      // 发送包含终端上下文的完整消息
      emit('send')
    }
  }

  /**
   * 处理模型选择变化
   */
  const handleModelChange = (value: string | number | null) => {
    const modelId = typeof value === 'string' ? value : null
    emit('update:selectedModel', modelId)
    emit('model-change', modelId)
  }

  /**
   * 处理模式切换
   */
  const handleModeChange = (value: string | number | null) => {
    const mode = value as 'chat' | 'agent'
    if (mode === 'chat' || mode === 'agent') {
      emit('mode-change', mode)
    }
  }

  // CK索引状态
  const indexStatus = ref<{
    isReady: boolean
    path: string
  }>({
    isReady: false,
    path: '.',
  })

  // 构建状态
  const buildProgress = ref(0)
  const isBuilding = ref(false)

  const showIndexModal = ref(false)

  /**
   * 处理CK索引按钮点击
   */
  const handleCkIndexClick = async () => {
    await checkCkIndexStatus()
    showIndexModal.value = true
  }

  /**
   * 检查CK索引状态
   */
  const checkCkIndexStatus = async () => {
    try {
      const activeTerminal = terminalStore.terminals.find(t => t.id === terminalStore.activeTerminalId)
      if (!activeTerminal || !activeTerminal.cwd) {
        indexStatus.value = { isReady: false, path: '' }
        return
      }
      const status = await ckApi.getIndexStatus({ path: activeTerminal.cwd })
      indexStatus.value = status
    } catch (error) {
      console.error('[Error] 获取CK索引状态失败:', error)
      indexStatus.value = { isReady: false, path: '' }
    }
  }

  /**
   * 构建CK索引
   */
  const buildCkIndex = async () => {
    try {
      isBuilding.value = true
      buildProgress.value = 0

      const activeTerminal = terminalStore.terminals.find(t => t.id === terminalStore.activeTerminalId)
      if (!activeTerminal || !activeTerminal.cwd) return
      const targetPath = activeTerminal.cwd
      const buildPromise = ckApi.buildIndex({ path: targetPath })

      // 轮询进度
      const progressInterval = setInterval(async () => {
        try {
          const progress = await ckApi.getBuildProgress({ path: targetPath })
          // 计算更平滑的进度：文件级 + 当前文件的 chunk 级
          if (progress.totalFiles > 0) {
            const totalFiles = Math.max(progress.totalFiles, 1)
            const filesCompleted = Math.min(progress.filesCompleted, totalFiles)
            const perFile = 100 / totalFiles
            let pct = filesCompleted * perFile

            if (progress.totalChunks && progress.totalChunks > 0) {
              const chunkDone = Math.min(progress.currentFileChunks ?? 0, progress.totalChunks)
              pct += (chunkDone / progress.totalChunks) * perFile
            }

            // 未完成时最高保留在 99%，避免和完成态的 100% 混淆
            buildProgress.value = Math.min(progress.isComplete ? 100 : 99, Math.max(0, pct))
          }
          if (progress.isComplete) {
            clearInterval(progressInterval)
            buildProgress.value = 100
            setTimeout(() => {
              isBuilding.value = false
              buildProgress.value = 0
            }, 500)
          }
        } catch (error) {
          console.warn('获取构建进度失败:', error)
          // 如果API不存在，回退到时间估算
          if (buildProgress.value < 90) {
            buildProgress.value += Math.random() * 10 + 2
          }
        }
      }, 500)

      try {
        await buildPromise
      } finally {
        clearInterval(progressInterval)
        if (!isBuilding.value) {
          // 已经通过进度查询完成
          return
        }

        // 如果没有通过进度查询完成，手动完成
        buildProgress.value = 100
        setTimeout(() => {
          isBuilding.value = false
          buildProgress.value = 0
        }, 500)
      }

      await checkCkIndexStatus()
    } catch (error) {
      console.error('构建CK索引失败:', error)
      isBuilding.value = false
      buildProgress.value = 0
    }
  }

  /**
   * 删除CK索引
   */
  const deleteCkIndex = async () => {
    try {
      const activeTerminal = terminalStore.terminals.find(t => t.id === terminalStore.activeTerminalId)
      if (!activeTerminal || !activeTerminal.cwd) return
      await ckApi.deleteIndex({ path: activeTerminal.cwd })
      await checkCkIndexStatus()
    } catch (error) {
      console.error('删除CK索引失败:', error)
    }
  }

  /**
   * 获取按钮提示文字
   */
  const getButtonTitle = () => {
    if (indexStatus.value.isReady) {
      return 'CK 语义索引已就绪'
    } else {
      return '构建 CK 语义索引'
    }
  }

  /**
   * 处理插入选定文本 - 优化逻辑
   */
  const handleInsertSelectedText = () => {
    const selectedText = terminalSelection.getSelectedText()
    if (!selectedText.trim()) return

    // 智能拼接：有内容时添加空格分隔
    const newValue = props.modelValue ? `${props.modelValue} ${selectedText}` : selectedText

    emit('update:modelValue', newValue)

    // 异步聚焦和调整高度
    nextTick(() => {
      inputTextarea.value?.focus()
      adjustTextareaHeight()
    })
  }

  /**
   * 获取标签上下文信息（用于传递给后端）
   */
  const getTagContextInfo = () => {
    return terminalSelection.getTagContextInfo()
  }

  // 初始化时检查CK索引状态
  onMounted(async () => {
    // 获取用户主目录，用于判断是否是初始目录
    try {
      homePath.value = await homeDir()
    } catch (error) {
      console.warn('获取用户主目录失败:', error)
    }
    await checkCkIndexStatus()
    // 在索引状态更新后，同步解析路径，来源统一为 indexStatus 或 terminalStore
    resolvedPath.value = indexStatus.value.path || terminalStore.currentWorkingDirectory || '.'
  })

  // 暴露方法给父组件
  defineExpose({
    adjustTextareaHeight,
    focus: () => inputTextarea.value?.focus(),
    getTagContextInfo,
  })

  // 去除本组件的事件监听，统一在可复用的 composable 内管理
</script>

<template>
  <div class="chat-input">
    <!-- 终端标签页标签 -->
    <TerminalTabTag
      :visible="terminalSelection.hasTerminalTab.value"
      :terminal-id="terminalSelection.currentTerminalTab.value?.terminalId"
      :shell="terminalSelection.currentTerminalTab.value?.shell"
      :cwd="terminalSelection.currentTerminalTab.value?.cwd"
      :display-path="terminalSelection.currentTerminalTab.value?.displayPath"
    />

    <!-- 终端选择标签 -->
    <TerminalSelectionTag
      :visible="terminalSelection.hasSelection.value"
      :selected-text="terminalSelection.selectedText.value"
      :selection-info="terminalSelection.selectionInfo.value"
      @clear="terminalSelection.clearSelection"
      @insert="handleInsertSelectedText"
    />

    <!-- 主输入区域 -->
    <div class="input-main">
      <div class="input-content">
        <textarea
          ref="inputTextarea"
          v-model="inputValue"
          class="message-input"
          :placeholder="placeholder || t('chat.input_placeholder')"
          rows="1"
          @keydown="handleKeydown"
          @input="adjustTextareaHeight"
        />
      </div>
    </div>

    <!-- 模型选择器和模式切换 -->
    <div class="input-bottom">
      <div class="bottom-left">
        <x-select
          class="mode-selector"
          :model-value="chatMode"
          :options="modeOptions"
          :placeholder="t('ai.select_mode')"
          size="small"
          borderless
          @update:model-value="handleModeChange"
        />
        <x-select
          class="model-selector"
          :model-value="selectedModel"
          :options="modelOptions"
          :placeholder="t('ai.select_model')"
          size="small"
          borderless
          @update:model-value="handleModelChange"
        />
      </div>
      <div class="bottom-right">
        <button
          class="database-button"
          :class="{
            'has-index': indexStatus.isReady,
            building: isBuilding,
          }"
          :disabled="isInSettingsTab || !canBuild"
          :title="isInSettingsTab ? '在设置页面时不可用' : !canBuild ? '请选择非初始目录后再使用' : getButtonTitle()"
          @click="handleCkIndexClick"
        >
          <div class="button-content">
            <!-- 构建进度圆环 -->
            <CircularProgress v-if="isBuilding" :percentage="buildProgress">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <ellipse cx="12" cy="5" rx="9" ry="3" />
                <path d="M3 5v14c0 1.66 4.03 3 9 3s9-1.34 9-3V5" />
                <path d="M3 12c0 1.66 4.03 3 9 3s9-1.34 9-3" />
              </svg>
            </CircularProgress>
            <!-- 正常状态图标 -->
            <template v-else>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <ellipse cx="12" cy="5" rx="9" ry="3" />
                <path d="M3 5v14c0 1.66 4.03 3 9 3s9-1.34 9-3V5" />
                <path d="M3 12c0 1.66 4.03 3 9 3s9-1.34 9-3" />
              </svg>
              <!-- 状态指示器 -->
              <div v-if="indexStatus.isReady" class="status-indicator ready"></div>
            </template>
          </div>
        </button>
        <button
          class="send-button"
          :class="{ 'stop-button': loading }"
          :disabled="!loading && (!canSend || isInSettingsTab)"
          :title="isInSettingsTab ? '在设置页面时不可用' : ''"
          @click="handleButtonClick"
        >
          <svg v-if="loading" width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <rect x="6" y="6" width="12" height="12" rx="2" />
          </svg>
          <svg v-else width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="m3 3 3 9-3 9 19-9z" />
            <path d="m6 12h16" />
          </svg>
        </button>
      </div>
    </div>

    <!-- CK索引弹窗 -->
    <InputPopover :visible="showIndexModal" :target-ref="inputTextarea" @update:visible="showIndexModal = $event">
      <CkIndexContent
        :index-status="{ hasIndex: indexStatus.isReady, path: indexStatus.path }"
        @build="buildCkIndex"
        @delete="deleteCkIndex"
        @refresh="checkCkIndexStatus"
      />
    </InputPopover>
  </div>
</template>

<style scoped>
  .chat-input {
    padding: 10px;
    margin: auto;
    width: 90%;
    margin-bottom: 10px;
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius-lg);
    background-color: var(--bg-400);
    transition: border-color 0.1s ease;
  }

  .chat-input:hover {
    border-color: var(--color-primary);
  }

  .input-main {
    display: flex;
    align-items: flex-end;
  }

  .input-content {
    flex: 1;
    min-height: 32px;
  }
  .message-input {
    width: 100%;
    min-height: 32px;
    max-height: 100px;
    border: none;
    background: transparent;
    color: var(--text-300);
    font-size: 14px;
    outline: none;
    resize: none;
  }

  .message-input::-webkit-scrollbar {
    display: none;
  }

  .message-input::placeholder {
    color: var(--text-400);
    opacity: 0.6;
  }

  .send-button {
    width: 28px;
    height: 28px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--color-primary);
    transition: color 0.2s ease;
  }

  .send-button:hover:not(:disabled) {
    color: var(--color-primary-hover);
  }

  .send-button:disabled {
    color: var(--text-400);
    cursor: not-allowed;
    opacity: 0.5;
  }

  .stop-button {
    color: var(--color-error) !important;
  }

  .stop-button:hover:not(:disabled) {
    color: var(--ansi-red) !important;
  }

  .input-bottom {
    margin-top: 8px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px; /* 减少整体间距 */
    min-width: 0; /* 确保可以缩小 */
  }

  .bottom-left {
    flex: 1;
    display: flex;
    gap: 8px;
    min-width: 0; /* 允许内容缩小 */
    overflow: hidden; /* 防止内容溢出 */
  }

  .bottom-right {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 4px; /* 减少按钮间距 */
  }

  .mode-selector {
    width: 100px;
    min-width: 60px; /* 设置最小宽度 */
    flex-shrink: 1; /* 允许缩小 */
  }

  .model-selector {
    width: 110px;
    min-width: 80px; /* 设置最小宽度 */
    flex-shrink: 1; /* 允许缩小 */
  }

  .database-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-300);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .database-button:hover {
    background: var(--bg-300);
    color: var(--accent-500);
  }

  .database-button:active {
    transform: scale(0.95);
  }

  .database-button .button-content {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .database-button.has-index {
    color: var(--accent-500);
  }

  .database-button.has-index:hover {
    background: var(--bg-300);
    color: var(--accent-500);
  }

  .database-button.disabled,
  .database-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .database-button.disabled:hover,
  .database-button:disabled:hover {
    background: transparent;
    color: var(--text-300);
    transform: none;
  }

  .status-indicator {
    position: absolute;
    top: -2px;
    right: -2px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    border: 1px solid var(--bg-400);
  }

  .status-indicator.ready {
    background: var(--color-success);
  }

  @keyframes pulse {
    0% {
      opacity: 0.6;
    }
    100% {
      opacity: 1;
    }
  }

  .chat-input {
    container-type: inline-size;
  }

  /* 极窄屏幕适配 */
  @container (max-width: 200px) {
    .input-bottom {
      flex-direction: column;
      gap: 6px;
      align-items: stretch;
    }

    .bottom-left {
      justify-content: space-between;
    }

    .bottom-right {
      justify-content: center;
    }

    .mode-selector,
    .model-selector {
      min-width: 50px;
      font-size: 12px;
    }
  }

  /* 窄屏幕适配 */
  @container (max-width: 280px) {
    .mode-selector {
      min-width: 45px;
      width: 70px;
    }

    .model-selector {
      min-width: 55px;
      width: 85px;
    }

    .input-bottom {
      gap: 4px;
    }

    .bottom-left {
      gap: 4px;
    }
  }
</style>
