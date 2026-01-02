<script setup lang="ts">
  import { computed, ref, onMounted, onBeforeUnmount, watch } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useTerminalSelection } from '@/composables/useTerminalSelection'
  import { useNodeVersion } from '@/composables/useNodeVersion'
  import { useProjectRules } from '@/composables/useProjectRules'
  import { useTerminalStore } from '@/stores/Terminal'
  import { homeDir } from '@tauri-apps/api/path'
  import TerminalTabTag from '../tags/TerminalTabTag.vue'
  import NodeVersionTag from '../tags/NodeVersionTag.vue'
  import ProjectRulesTag from '../tags/ProjectRulesTag.vue'
  import InputPopover from '@/components/ui/InputPopover.vue'
  import VectorIndexContent from '../vectorIndex/VectorIndexContent.vue'
  import FolderPicker from '../tags/FolderPicker.vue'
  import NodeVersionPicker from '../tags/NodeVersionPicker.vue'
  import ProjectRulesPicker from '../tags/ProjectRulesPicker.vue'
  import CircularProgress from '@/components/ui/CircularProgress.vue'
  import ImagePreview, { type ImageAttachment } from './ImagePreview.vue'
  import { vectorDbApi as vdbApi, nodeApi } from '@/api'
  import { processImageFile, getImageFromClipboard, validateImageFile } from '@/utils/imageUtils'
  import { createMessage } from '@/ui/composables/message-api'

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
    (e: 'send', images?: ImageAttachment[]): void
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
    if (compositionTimer) {
      clearTimeout(compositionTimer)
      compositionTimer = undefined
    }
  })

  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const inputTextarea = ref<HTMLTextAreaElement>()
  const fileInput = ref<HTMLInputElement>()
  const isComposing = ref(false)
  let compositionTimer: number | undefined

  // 图片附件
  const imageAttachments = ref<ImageAttachment[]>([])

  const terminalSelection = useTerminalSelection()
  const nodeVersion = useNodeVersion()
  const projectRules = useProjectRules()

  const terminalStore = useTerminalStore()
  const activeTerminalCwd = computed(() => terminalStore.activeTerminal?.cwd || null)

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
    if (event.key === 'Enter' && !event.shiftKey && !isComposing.value) {
      event.preventDefault()
      handleButtonClick()
    }
  }

  const handleCompositionStart = () => {
    if (compositionTimer) {
      clearTimeout(compositionTimer)
      compositionTimer = undefined
    }
    isComposing.value = true
  }

  const handleCompositionEnd = () => {
    compositionTimer = window.setTimeout(() => {
      isComposing.value = false
      compositionTimer = undefined
    }, 10)
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
    } else if (props.canSend || imageAttachments.value.length > 0) {
      emit('send', imageAttachments.value.length > 0 ? imageAttachments.value : undefined)
      imageAttachments.value = []
    }
  }

  // 图片上传相关
  const handleImageUpload = () => {
    fileInput.value?.click()
  }

  const handleFileSelect = async (event: Event) => {
    const target = event.target as HTMLInputElement
    const files = target.files
    if (!files || files.length === 0) return

    for (const file of Array.from(files)) {
      await addImageFile(file)
    }

    // 清空 input，允许重复选择同一文件
    target.value = ''
  }

  const addImageFile = async (file: File) => {
    // 检查图片数量限制
    if (imageAttachments.value.length >= 5) {
      console.warn(t('chat.max_images_reached'))
      // TODO: 显示错误提示
      return
    }

    // 验证文件 (Tauri macOS 下 accept 属性不生效，必须在代码层验证)
    const validation = validateImageFile(file)
    if (!validation.valid) {
      createMessage.error(validation.error || t('chat.invalid_file_type'))
      return
    }

    try {
      const processed = await processImageFile(file)
      const attachment: ImageAttachment = {
        id: `${Date.now()}-${Math.random()}`,
        dataUrl: processed.dataUrl,
        fileName: processed.fileName,
        fileSize: processed.fileSize,
        mimeType: processed.mimeType,
      }
      imageAttachments.value.push(attachment)
    } catch (error) {
      console.error('Failed to process image:', error)
      // TODO: 显示错误提示
    }
  }

  const handlePaste = async (event: ClipboardEvent) => {
    const imageFile = await getImageFromClipboard(event)
    if (imageFile) {
      event.preventDefault()
      await addImageFile(imageFile)
    }
  }

  const removeImage = (id: string) => {
    imageAttachments.value = imageAttachments.value.filter(img => img.id !== id)
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
    size?: string
  }>({
    isReady: false,
    path: '.',
    size: '',
  })

  const syncResolvedPath = () => {
    const cwd = activeTerminalCwd.value
    if (cwd) {
      resolvedPath.value = cwd
      return
    }

    const indexPath = indexStatus.value.path
    resolvedPath.value = indexPath || '.'
  }

  watch(
    () => terminalSelection.currentTerminalTab.value,
    async tab => {
      if (!tab?.cwd || tab.cwd === '~') {
        nodeVersion.state.value = { isNodeProject: false, currentVersion: null, manager: null }
        projectRules.state.value = { hasRulesFile: false, selectedRulesFile: null }
        return
      }

      await Promise.all([nodeVersion.detect(tab.cwd, tab.terminalId), projectRules.detect(tab.cwd)])
    },
    { immediate: true }
  )

  watch(
    [activeTerminalCwd, () => indexStatus.value.path],
    () => {
      syncResolvedPath()
    },
    {
      immediate: true,
    }
  )

  const buildProgress = ref(0)
  const isBuilding = ref(false)
  const progressHasData = ref(false)
  let progressTimer: number | undefined

  const showIndexModal = ref(false)
  const showNavigatorModal = ref(false)
  const showNodeVersionModal = ref(false)
  const showProjectRulesModal = ref(false)

  const handleNodeVersionSelect = async (version: string) => {
    const terminalId = terminalSelection.currentTerminalTab.value?.terminalId
    const manager = nodeVersion.state.value.manager

    if (!terminalId || !manager) return

    const command = await nodeApi.getSwitchCommand(manager, version)
    await terminalStore.writeToTerminal(terminalId, command)
    showNodeVersionModal.value = false
  }

  const handleProjectRulesSelect = async () => {
    await projectRules.refresh()
    showProjectRulesModal.value = false
  }

  const handleVectorIndexClick = async () => {
    await checkVectorIndexStatus()
    showIndexModal.value = true
  }

  const handleOpenNavigator = () => {
    showNavigatorModal.value = true
  }

  const checkVectorIndexStatus = async () => {
    const activeTerminal = terminalStore.terminals.find(t => t.id === terminalStore.activeTerminalId)
    if (!activeTerminal || !activeTerminal.cwd) {
      indexStatus.value = { isReady: false, path: '' }
      return
    }
    const status = await vdbApi.getIndexStatus({ path: activeTerminal.cwd })
    indexStatus.value = { isReady: status.isReady, path: status.path, size: status.size }
  }

  watch(activeTerminalCwd, cwd => {
    if (!cwd) {
      indexStatus.value = { isReady: false, path: '' }
      return
    }
    checkVectorIndexStatus()
  })

  const startProgressPolling = (targetPath: string) => {
    if (progressTimer) {
      clearInterval(progressTimer)
      progressTimer = undefined
    }
    progressHasData.value = false
    progressTimer = window.setInterval(async () => {
      const progress = await vdbApi.getBuildProgress({ path: targetPath })
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
        await checkVectorIndexStatus()
      }
    }, 600)
  }

  const rebuildVectorIndex = async () => {
    const activeTerminal = terminalStore.terminals.find(t => t.id === terminalStore.activeTerminalId)
    if (!activeTerminal || !activeTerminal.cwd) return
    const targetPath = activeTerminal.cwd

    isBuilding.value = true
    buildProgress.value = 0

    await vdbApi.rebuildIndex({ root: targetPath })

    startProgressPolling(targetPath)
  }

  const cancelVectorIndex = async () => {
    const activeTerminal = terminalStore.terminals.find(t => t.id === terminalStore.activeTerminalId)
    if (!activeTerminal || !activeTerminal.cwd) return

    await vdbApi.cancelBuild({ path: activeTerminal.cwd })

    isBuilding.value = false
    buildProgress.value = 0

    if (progressTimer) {
      clearInterval(progressTimer)
      progressTimer = undefined
    }
  }

  const deleteVectorIndex = async () => {
    const activeTerminal = terminalStore.terminals.find(t => t.id === terminalStore.activeTerminalId)
    if (!activeTerminal || !activeTerminal.cwd) return

    await vdbApi.deleteWorkspaceIndex(activeTerminal.cwd)
    await checkVectorIndexStatus()
  }

  const getButtonTitle = () => {
    if (indexStatus.value.isReady) {
      return t('ck.index_ready')
    } else {
      return t('ck.build_index')
    }
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
    await checkVectorIndexStatus()
    syncResolvedPath()

    nodeVersion.setupListener(() => terminalSelection.currentTerminalTab.value?.terminalId ?? 0)

    try {
      const targetPath = indexStatus.value.path || activeTerminalCwd.value
      if (targetPath) {
        const progress = await vdbApi.getBuildProgress({ path: targetPath })
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
    } catch (e) {
      console.warn('Failed to start progress polling:', e)
    }
  })

  defineExpose({
    adjustTextareaHeight,
    focus: () => inputTextarea.value?.focus(),
    getTagContextInfo,
    clearImages: () => {
      imageAttachments.value = []
    },
    setImages: (images: ImageAttachment[]) => {
      imageAttachments.value = images
    },
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
      @open-navigator="handleOpenNavigator"
    />

    <NodeVersionTag
      :visible="nodeVersion.state.value.isNodeProject"
      :version="nodeVersion.state.value.currentVersion"
      @click="showNodeVersionModal = true"
    />

    <ProjectRulesTag
      :visible="projectRules.state.value.hasRulesFile"
      :rules-file="projectRules.state.value.selectedRulesFile"
      @click="showProjectRulesModal = true"
    />

    <ImagePreview :images="imageAttachments" @remove="removeImage" />

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
          @compositionstart="handleCompositionStart"
          @compositionend="handleCompositionEnd"
          @paste="handlePaste"
        />
      </div>
    </div>

    <input ref="fileInput" type="file" accept="image/*" multiple style="display: none" @change="handleFileSelect" />

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
          class="image-upload-button"
          :disabled="imageAttachments.length >= 5"
          :title="imageAttachments.length >= 5 ? t('chat.max_images_reached') : t('chat.upload_image')"
          @click="handleImageUpload"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="3" y="3" width="18" height="18" rx="3" ry="3" />
            <circle cx="8.5" cy="8.5" r="1.5" />
            <path d="M21 15l-5-5L5 21" />
          </svg>
        </button>
        <button
          class="database-button"
          :class="{
            'has-index': indexStatus.isReady,
            building: isBuilding,
          }"
          :disabled="!canBuild"
          :title="!canBuild ? t('ck.index_button_select_non_home') : getButtonTitle()"
          @click="handleVectorIndexClick"
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
          :disabled="!loading && !canSend"
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

    <InputPopover :visible="showIndexModal" @update:visible="showIndexModal = $event">
      <VectorIndexContent
        :index-status="{ hasIndex: indexStatus.isReady, path: indexStatus.path, size: indexStatus.size }"
        :is-building="isBuilding"
        :build-progress="buildProgress"
        @build="rebuildVectorIndex"
        @delete="deleteVectorIndex"
        @refresh="checkVectorIndexStatus"
        @cancel="cancelVectorIndex"
      />
    </InputPopover>

    <InputPopover :visible="showNavigatorModal" @update:visible="showNavigatorModal = $event">
      <FolderPicker
        v-if="terminalSelection.currentTerminalTab.value?.terminalId && terminalSelection.currentTerminalTab.value?.cwd"
        :current-path="terminalSelection.currentTerminalTab.value.cwd"
        :terminal-id="terminalSelection.currentTerminalTab.value.terminalId"
        @close="showNavigatorModal = false"
      />
    </InputPopover>

    <InputPopover :visible="showNodeVersionModal" @update:visible="showNodeVersionModal = $event">
      <NodeVersionPicker
        v-if="nodeVersion.state.value.manager && nodeVersion.state.value.currentVersion"
        :current-version="nodeVersion.state.value.currentVersion"
        :manager="nodeVersion.state.value.manager"
        :cwd="terminalSelection.currentTerminalTab.value?.cwd"
        @select="handleNodeVersionSelect"
        @close="showNodeVersionModal = false"
      />
    </InputPopover>

    <InputPopover :visible="showProjectRulesModal" @update:visible="showProjectRulesModal = $event">
      <ProjectRulesPicker
        :current-rules="projectRules.state.value.selectedRulesFile"
        :cwd="terminalSelection.currentTerminalTab.value?.cwd"
        @select="handleProjectRulesSelect"
        @close="showProjectRulesModal = false"
      />
    </InputPopover>
  </div>
</template>

<style scoped>
  .chat-input {
    position: relative;
    padding: 10px;
    margin: auto;
    width: 96%;
    margin-bottom: 10px;
    border: 1px solid var(--border-300);
    border-radius: 16px;
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
    color: var(--color-error);
    background: var(--color-error);
    border-radius: 50%;
  }

  .stop-button svg {
    color: white;
  }

  .stop-button:hover:not(:disabled) {
    background: var(--ansi-red);
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
    width: 160px;
    min-width: 80px;
    flex-shrink: 1;
  }

  .image-upload-button {
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

  .image-upload-button:hover:not(:disabled) {
    background: var(--bg-300);
    color: var(--color-primary);
  }

  .image-upload-button:active:not(:disabled) {
    transform: scale(0.95);
  }

  .image-upload-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
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
