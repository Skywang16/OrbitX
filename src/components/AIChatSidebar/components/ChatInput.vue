<script setup lang="ts">
  import { computed, ref, nextTick, onMounted, onBeforeUnmount } from 'vue'
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

  interface Props {
    modelValue: string
    placeholder?: string
    loading?: boolean

    canSend?: boolean
    selectedModel?: string | null
    modelOptions?: Array<{ label: string; value: string | number }>
    chatMode?: 'chat' | 'agent'
  }

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

  onBeforeUnmount(() => {
    if (progressTimer) {
      clearInterval(progressTimer)
      progressTimer = undefined
    }
  })

  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const inputTextarea = ref<HTMLTextAreaElement>()

  const terminalSelection = useTerminalSelection()

  const tabManagerStore = useTabManagerStore()
  const terminalStore = useTerminalStore()

  const isInSettingsTab = computed(() => {
    return tabManagerStore.activeTab?.type === TabType.SETTINGS
  })

  const homePath = ref<string>('')

  const resolvedPath = ref<string>('.')

  const normalize = (p: string) => p.replace(/\\/g, '/').replace(/\/$/, '')

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

  const inputValue = computed({
    get: () => props.modelValue,
    set: (value: string) => emit('update:modelValue', value),
  })

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

  const handleKeydown = (event: KeyboardEvent) => {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault()
      handleButtonClick()
    }
  }

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

  const handleButtonClick = () => {
    if (props.loading) {
      emit('stop')
    } else if (props.canSend) {
      emit('send')
    }
  }

  const handleModelChange = (value: string | number | null) => {
    const modelId = typeof value === 'string' ? value : null
    emit('update:selectedModel', modelId)
    emit('model-change', modelId)
  }

  const handleModeChange = (value: string | number | null) => {
    const mode = value as 'chat' | 'agent'
    if (mode === 'chat' || mode === 'agent') {
      emit('mode-change', mode)
    }
  }

  const indexStatus = ref<{
    isReady: boolean
    path: string
  }>({
    isReady: false,
    path: '.',
  })

  const buildProgress = ref(0)
  const isBuilding = ref(false)
  const progressHasData = ref(false)
  let progressTimer: number | undefined

  const showIndexModal = ref(false)

  const handleCkIndexClick = async () => {
    await checkCkIndexStatus()
    showIndexModal.value = true
  }

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

  const startProgressPolling = (targetPath: string) => {
    if (progressTimer) {
      clearInterval(progressTimer)
      progressTimer = undefined
    }
    progressHasData.value = false
    progressTimer = window.setInterval(async () => {
      try {
        const progress = await ckApi.getBuildProgress({ path: targetPath })
        if (progress.totalFiles > 0) {
          const totalFiles = Math.max(progress.totalFiles, 1)
          const filesCompleted = Math.min(progress.filesCompleted, totalFiles)
          const perFile = 100 / totalFiles
          let pct = filesCompleted * perFile

          if (progress.totalChunks && progress.totalChunks > 0) {
            const chunkDone = Math.min(progress.currentFileChunks ?? 0, progress.totalChunks)
            pct += (chunkDone / progress.totalChunks) * perFile
          }

          const nextPct = Math.min(progress.isComplete ? 100 : 99, Math.max(0, pct))
          if (!progressHasData.value) {
            progressHasData.value = true
            buildProgress.value = nextPct
          } else {
            buildProgress.value = Math.max(buildProgress.value, nextPct)
          }
        }

        if (progress.isComplete) {
          if (progressTimer) {
            clearInterval(progressTimer)
            progressTimer = undefined
          }
          buildProgress.value = 100
          setTimeout(() => {
            isBuilding.value = false
            buildProgress.value = 0
          }, 500)
          await checkCkIndexStatus()
        }
      } catch (error) {
        console.warn('获取构建进度失败:', error)
        if (progressHasData.value && buildProgress.value < 95) {
          buildProgress.value = Math.min(95, buildProgress.value + (Math.random() * 3 + 0.5))
        }
      }
    }, 600)
  }

  const buildCkIndex = async () => {
    try {
      const activeTerminal = terminalStore.terminals.find(t => t.id === terminalStore.activeTerminalId)
      if (!activeTerminal || !activeTerminal.cwd) return
      const targetPath = activeTerminal.cwd

      showIndexModal.value = false

      isBuilding.value = true
      buildProgress.value = 0

      await ckApi.buildIndex({ path: targetPath })

      startProgressPolling(targetPath)
    } catch (error) {
      console.error('构建CK索引失败:', error)
      isBuilding.value = false
      buildProgress.value = 0
    }
  }

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

  const getButtonTitle = () => {
    if (indexStatus.value.isReady) {
      return t('ck.index_ready')
    } else {
      return t('ck.build_index')
    }
  }

  const handleInsertSelectedText = () => {
    const selectedText = terminalSelection.getSelectedText()
    if (!selectedText.trim()) return

    const newValue = props.modelValue ? `${props.modelValue} ${selectedText}` : selectedText

    emit('update:modelValue', newValue)

    nextTick(() => {
      inputTextarea.value?.focus()
      adjustTextareaHeight()
    })
  }

  const getTagContextInfo = () => {
    return terminalSelection.getTagContextInfo()
  }

  onMounted(async () => {
    try {
      homePath.value = await homeDir()
    } catch (error) {
      console.warn('获取用户主目录失败:', error)
    }
    await checkCkIndexStatus()
    resolvedPath.value = indexStatus.value.path || terminalStore.currentWorkingDirectory || '.'

    try {
      const targetPath = indexStatus.value.path || terminalStore.currentWorkingDirectory
      if (targetPath) {
        const progress = await ckApi.getBuildProgress({ path: targetPath })
        if (!progress.isComplete && (progress.totalFiles > 0 || progress.error !== 'progress_unavailable')) {
          isBuilding.value = true
          if (progress.totalFiles > 0) {
            const totalFiles = Math.max(progress.totalFiles, 1)
            const filesCompleted = Math.min(progress.filesCompleted, totalFiles)
            const perFile = 100 / totalFiles
            let pct = filesCompleted * perFile
            if (progress.totalChunks && progress.totalChunks > 0) {
              const chunkDone = Math.min(progress.currentFileChunks ?? 0, progress.totalChunks)
              pct += (chunkDone / progress.totalChunks) * perFile
            }
            buildProgress.value = Math.min(99, Math.max(0, pct))
          }
          startProgressPolling(targetPath)
        }
      }
    } catch (e) {}
  })

  defineExpose({
    adjustTextareaHeight,
    focus: () => inputTextarea.value?.focus(),
    getTagContextInfo,
  })
</script>

<template>
  <div class="chat-input">
    <TerminalTabTag
      :visible="terminalSelection.hasTerminalTab.value"
      :terminal-id="terminalSelection.currentTerminalTab.value?.terminalId"
      :shell="terminalSelection.currentTerminalTab.value?.shell"
      :cwd="terminalSelection.currentTerminalTab.value?.cwd"
      :display-path="terminalSelection.currentTerminalTab.value?.displayPath"
    />

    <TerminalSelectionTag
      :visible="terminalSelection.hasSelection.value"
      :selected-text="terminalSelection.selectedText.value"
      :selection-info="terminalSelection.selectionInfo.value"
      @clear="terminalSelection.clearSelection"
      @insert="handleInsertSelectedText"
    />

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
          :title="
            isInSettingsTab
              ? t('ck.index_button_disabled_in_settings')
              : !canBuild
                ? t('ck.index_button_select_non_home')
                : getButtonTitle()
          "
          @click="handleCkIndexClick"
        >
          <div class="button-content">
            <CircularProgress v-if="isBuilding" :percentage="buildProgress">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <ellipse cx="12" cy="5" rx="9" ry="3" />
                <path d="M3 5v14c0 1.66 4.03 3 9 3s9-1.34 9-3V5" />
                <path d="M3 12c0 1.66 4.03 3 9 3s9-1.34 9-3" />
              </svg>
            </CircularProgress>
            <template v-else>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <ellipse cx="12" cy="5" rx="9" ry="3" />
                <path d="M3 5v14c0 1.66 4.03 3 9 3s9-1.34 9-3V5" />
                <path d="M3 12c0 1.66 4.03 3 9 3s9-1.34 9-3" />
              </svg>
              <div v-if="indexStatus.isReady" class="status-indicator ready"></div>
            </template>
          </div>
        </button>
        <button
          class="send-button"
          :class="{ 'stop-button': loading }"
          :disabled="!loading && (!canSend || isInSettingsTab)"
          :title="isInSettingsTab ? t('ck.send_button_disabled_in_settings') : ''"
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
    gap: 8px;
    min-width: 0;
  }

  .bottom-left {
    flex: 1;
    display: flex;
    gap: 8px;
    min-width: 0;
    overflow: hidden;
  }

  .bottom-right {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .mode-selector {
    width: 100px;
    min-width: 60px;
    flex-shrink: 1;
  }

  .model-selector {
    width: 110px;
    min-width: 80px;
    flex-shrink: 1;
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
