<script setup lang="ts">
  import { nodeApi, vectorDbApi as vdbApi } from '@/api'
  import type { ChannelSubscription } from '@/api/channel'
  import { useAIChatStore } from '@/components/AIChatSidebar/store'
  import CircularProgress from '@/components/ui/CircularProgress.vue'
  import InputPopover from '@/components/ui/InputPopover.vue'
  import { useNodeVersion } from '@/composables/useNodeVersion'
  import { useProjectRules } from '@/composables/useProjectRules'
  import { useTerminalSelection } from '@/composables/useTerminalSelection'
  import { useTerminalStore } from '@/stores/Terminal'
  import { useLayoutStore } from '@/stores/layout'
  import { createSlashCommands, SLASH_COMMAND_ICONS, type SlashCommand } from '@/types/slashCommand'
  import { createMessage } from '@/ui/composables/message-api'
  import { getImageFromClipboard, processImageFile, validateImageFile } from '@/utils/imageUtils'
  import { homeDir } from '@tauri-apps/api/path'
  import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'
  import NodeVersionPicker from '../tags/NodeVersionPicker.vue'
  import NodeVersionTag from '../tags/NodeVersionTag.vue'
  import ProjectRulesPicker from '../tags/ProjectRulesPicker.vue'
  import ProjectRulesTag from '../tags/ProjectRulesTag.vue'
  import VectorIndexContent from '../vectorIndex/VectorIndexContent.vue'
  import ContextUsageRing from './ContextUsageRing.vue'
  import ImagePreview, { type ImageAttachment } from './ImagePreview.vue'
  import MessageQueue from './MessageQueue.vue'
  import SlashCommandMenu from './SlashCommandMenu.vue'

  interface Props {
    modelValue: string
    placeholder?: string
    loading?: boolean

    canSend?: boolean
    selectedModel?: string | null
    modelOptions?: Array<{ label: string; value: string | number }>
  }

  interface Emits {
    (e: 'update:modelValue', value: string): void
    (e: 'send', images?: ImageAttachment[]): void
    (e: 'stop'): void
    (e: 'update:selectedModel', value: string | null): void
    (e: 'model-change', value: string | null): void
  }

  const props = withDefaults(defineProps<Props>(), {
    placeholder: '',
    loading: false,

    canSend: false,
    selectedModel: null,
    modelOptions: () => [],
  })

  onBeforeUnmount(() => {
    buildSubscription?.unsubscribe().catch(error => {
      console.warn('Failed to unsubscribe vector build progress channel on unmount:', error)
    })
    buildSubscription = null
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

  // Image attachments
  const imageAttachments = ref<ImageAttachment[]>([])
  const selectedCommand = ref<SlashCommand | null>(null)

  const terminalSelection = useTerminalSelection()
  const nodeVersion = useNodeVersion()
  const projectRules = useProjectRules()
  const aiChatStore = useAIChatStore()
  const terminalStore = useTerminalStore()
  const layoutStore = useLayoutStore()

  const workspacePath = computed(() => aiChatStore.currentWorkspacePath ?? null)

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

  const handleKeydown = (event: KeyboardEvent) => {
    // When / is pressed (half-width), trigger command menu (Claude-style at start)
    if (event.key === '/' && !isComposing.value) {
      const value = inputValue.value
      const textarea = inputTextarea.value
      const cursorAtStart =
        textarea?.selectionStart === 0 && textarea?.selectionEnd === 0 && document.activeElement === textarea

      if (value.trim() === '' || cursorAtStart) {
        event.preventDefault()
        showSlashCommandMenu.value = true
        return
      }
    }

    // If command menu is open, handle keyboard events
    if (showSlashCommandMenu.value) {
      if (event.key === 'Escape') {
        event.preventDefault()
        showSlashCommandMenu.value = false
      }
      return
    }

    if (event.key === 'Enter' && !event.shiftKey && !isComposing.value) {
      event.preventDefault()
      handleButtonClick()
    }
  }

  // Handle input change, also detect full-width slash from Chinese input
  const handleInput = () => {
    adjustTextareaHeight()

    // Detect pasted/typed command prefix like "/code-review ..." or "／code-review ..."
    if (!selectedCommand.value && !showSlashCommandMenu.value) {
      const value = inputValue.value
      const match = value.match(/^(\/|／)([a-zA-Z0-9-]+)\b\s*/i)
      if (match) {
        const id = match[2].toLowerCase()
        const cmd = createSlashCommands(t).find(c => c.id === id)
        if (cmd) {
          selectedCommand.value = cmd
          inputValue.value = value.slice(match[0].length)
          return
        }
      }
    }

    // Detect full-width slash (／) from Chinese input method (single-char trigger)
    const value = inputValue.value
    if (value === '／' || value === '/') {
      inputValue.value = ''
      showSlashCommandMenu.value = true
    }
  }

  // Handle slash command selection
  const handleSlashCommandSelect = (command: SlashCommand) => {
    showSlashCommandMenu.value = false
    selectedCommand.value = command
    inputTextarea.value?.focus()
  }

  const removeCommand = () => {
    selectedCommand.value = null
  }

  const getIcon = (iconName: string) => SLASH_COMMAND_ICONS[iconName] || ''

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

  const hasInput = computed(() => props.canSend || imageAttachments.value.length > 0)
  const showStopButton = computed(() => props.loading && !hasInput.value)

  const handleButtonClick = () => {
    if (hasInput.value) {
      aiChatStore.pendingCommandId = selectedCommand.value?.id ?? null
      emit('send', imageAttachments.value.length > 0 ? imageAttachments.value : undefined)
      imageAttachments.value = []
      selectedCommand.value = null
    } else if (props.loading) {
      emit('stop')
    }
  }

  // Image upload related
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

    // Clear input to allow selecting the same file again
    target.value = ''
  }

  const addImageFile = async (file: File) => {
    // Check image count limit
    if (imageAttachments.value.length >= 5) {
      console.warn(t('chat.max_images_reached'))
      // TODO: Show error message
      return
    }

    // Validate file (accept attribute doesn't work on Tauri macOS, must validate in code)
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
      // TODO: Show error message
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

  const handleModelChange = (event: Event) => {
    const target = event.target as HTMLSelectElement
    const modelId = target.value || null
    emit('update:selectedModel', modelId)
    emit('model-change', modelId)
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
    const wp = workspacePath.value
    if (wp) {
      resolvedPath.value = wp
      return
    }

    const indexPath = indexStatus.value.path
    resolvedPath.value = indexPath || '.'
  }

  watch(
    workspacePath,
    async path => {
      if (!path) {
        nodeVersion.state.value = { isNodeProject: false, currentVersion: null, manager: null }
        projectRules.state.value = { hasRulesFile: false, selectedRulesFile: null }
        return
      }
      await Promise.all([nodeVersion.detect(path), projectRules.detect(path)])
    },
    { immediate: true }
  )

  watch(
    [workspacePath, () => indexStatus.value.path],
    () => {
      syncResolvedPath()
    },
    {
      immediate: true,
    }
  )

  const buildProgress = ref(0)
  const isBuilding = ref(false)
  let buildSubscription: ChannelSubscription | null = null

  const showIndexModal = ref(false)
  const showNodeVersionModal = ref(false)
  const showProjectRulesModal = ref(false)
  const showSlashCommandMenu = ref(false)

  const handleNodeVersionSelect = async (version: string) => {
    const manager = nodeVersion.state.value.manager
    if (!manager) return

    const command = await nodeApi.getSwitchCommand(manager, version)

    let terminalId = terminalSelection.currentTerminalTab.value?.terminalId
    if (!terminalId) {
      // No terminal open — create one in the workspace directory and show it
      layoutStore.openTerminalPanel()
      terminalId = await terminalStore.createTerminalPane(workspacePath.value ?? undefined)
      await terminalStore.setActiveTerminal(terminalId)
    }

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

  const checkVectorIndexStatus = async () => {
    const wp = workspacePath.value
    if (!wp) {
      indexStatus.value = { isReady: false, path: '' }
      return
    }
    const status = await vdbApi.getIndexStatus({ path: wp })
    indexStatus.value = { isReady: status.isReady, path: status.path, size: status.size }
  }

  watch(workspacePath, wp => {
    if (!wp) {
      indexStatus.value = { isReady: false, path: '' }
      return
    }
    checkVectorIndexStatus()
  })

  const computeBuildPercent = (p: {
    totalFiles: number
    filesDone: number
    currentFileChunksTotal: number
    currentFileChunksDone: number
    isDone: boolean
  }): number => {
    if (p.totalFiles <= 0) return 0
    const currentFrac = p.currentFileChunksTotal > 0 ? p.currentFileChunksDone / p.currentFileChunksTotal : 0
    const pct = ((p.filesDone + currentFrac) / p.totalFiles) * 100
    return p.isDone ? 100 : Math.min(99, Math.max(0, pct))
  }

  const rebuildVectorIndex = async () => {
    const targetPath = workspacePath.value
    if (!targetPath) return

    isBuilding.value = true
    buildProgress.value = 0

    await buildSubscription?.unsubscribe().catch(error => {
      console.warn('Failed to unsubscribe previous vector build progress channel:', error)
    })
    buildSubscription = null

    await vdbApi.startBuildIndex({ root: targetPath })
    buildSubscription = vdbApi.subscribeBuildProgress(
      { root: targetPath },
      {
        onMessage: async progress => {
          buildProgress.value = computeBuildPercent(progress)
          if (!progress.isDone) return

          await buildSubscription?.unsubscribe().catch(error => {
            console.warn('Failed to unsubscribe completed vector build progress channel:', error)
          })
          buildSubscription = null

          isBuilding.value = false
          buildProgress.value = 0

          if (progress.phase === 'failed') {
            createMessage.error(
              progress.error ? t('ck.build_failed_with_error', { error: progress.error }) : t('ck.build_failed')
            )
          } else if (progress.phase === 'cancelled') {
            createMessage.info(t('ck.build_cancelled'))
          } else if (progress.filesFailed > 0) {
            createMessage.warning(t('ck.build_done_with_failures', { count: progress.filesFailed }))
          } else {
            createMessage.success(t('ck.build_done'))
          }

          await checkVectorIndexStatus()
        },
        onError: err => {
          console.warn('vector build channel error:', err)
          createMessage.error(t('ck.build_channel_error'))
          isBuilding.value = false
          buildProgress.value = 0
        },
      }
    )
  }

  const cancelVectorIndex = async () => {
    const targetPath = workspacePath.value
    if (!targetPath) return

    await vdbApi.cancelBuild({ root: targetPath })
    await buildSubscription?.unsubscribe().catch(error => {
      console.warn('Failed to unsubscribe cancelled vector build progress channel:', error)
    })
    buildSubscription = null

    isBuilding.value = false
    buildProgress.value = 0
  }

  const deleteVectorIndex = async () => {
    const targetPath = workspacePath.value
    if (!targetPath) return

    await vdbApi.deleteWorkspaceIndex(targetPath)
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
      console.warn('Failed to get user home directory:', error)
    }
    await checkVectorIndexStatus()
    syncResolvedPath()

    nodeVersion.setupListener(() => terminalSelection.currentTerminalTab.value?.terminalId ?? 0)

    const targetPath = indexStatus.value.path || workspacePath.value
    if (targetPath) {
      const progress = await vdbApi.getBuildStatus({ root: targetPath })
      if (progress && !progress.isDone) {
        isBuilding.value = true
        buildProgress.value = computeBuildPercent(progress)
        buildSubscription = vdbApi.subscribeBuildProgress(
          { root: targetPath },
          {
            onMessage: async p => {
              buildProgress.value = computeBuildPercent(p)
              if (!p.isDone) return

              await buildSubscription?.unsubscribe().catch(error => {
                console.warn('Failed to unsubscribe recovered vector build progress channel:', error)
              })
              buildSubscription = null
              isBuilding.value = false
              buildProgress.value = 0
              await checkVectorIndexStatus()
            },
            onError: err => {
              console.warn('vector build channel error:', err)
              isBuilding.value = false
              buildProgress.value = 0
            },
          }
        )
      }
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

    <div v-if="selectedCommand" class="command-tag">
      <span class="command-icon" v-html="getIcon(selectedCommand.icon)" />
      <span class="command-label">{{ selectedCommand.label }}</span>
      <button class="command-remove" @click="removeCommand">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="18" y1="6" x2="6" y2="18" />
          <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
      </button>
    </div>

    <MessageQueue
      :queue="aiChatStore.currentThreadQueue"
      @remove="aiChatStore.removeQueuedMessage"
      @update="aiChatStore.updateQueuedMessage"
      @send-now="aiChatStore.sendQueuedMessageNow"
      @reorder="aiChatStore.reorderQueuedMessage"
    />

    <ContextUsageRing :context-usage="aiChatStore.contextUsage" class="context-usage-indicator" />

    <div class="input-main">
      <div class="input-content">
        <textarea
          ref="inputTextarea"
          v-model="inputValue"
          class="message-input"
          :placeholder="placeholder || t('chat.input_placeholder')"
          rows="1"
          @keydown="handleKeydown"
          @input="handleInput"
          @compositionstart="handleCompositionStart"
          @compositionend="handleCompositionEnd"
          @paste="handlePaste"
        />
      </div>
    </div>

    <input ref="fileInput" type="file" accept="image/*" multiple style="display: none" @change="handleFileSelect" />

    <div class="input-bottom">
      <div class="bottom-left">
        <select class="native-select model-selector" :value="selectedModel" @change="handleModelChange">
          <option value="" disabled>{{ t('ai.select_model') }}</option>
          <option v-for="option in modelOptions" :key="option.value" :value="option.value">
            {{ option.label }}
          </option>
        </select>
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
          :class="{ 'stop-button': showStopButton }"
          :disabled="!loading && !canSend"
          @click="handleButtonClick"
        >
          <svg v-if="showStopButton" width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
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

    <InputPopover :visible="showNodeVersionModal" @update:visible="showNodeVersionModal = $event">
      <NodeVersionPicker
        :current-version="nodeVersion.state.value.currentVersion ?? ''"
        :manager="nodeVersion.state.value.manager ?? ''"
        :cwd="workspacePath ?? undefined"
        @select="handleNodeVersionSelect"
        @close="showNodeVersionModal = false"
      />
    </InputPopover>

    <InputPopover :visible="showProjectRulesModal" @update:visible="showProjectRulesModal = $event">
      <ProjectRulesPicker
        :current-rules="projectRules.state.value.selectedRulesFile"
        :cwd="workspacePath ?? undefined"
        @select="handleProjectRulesSelect"
        @close="showProjectRulesModal = false"
      />
    </InputPopover>

    <InputPopover :visible="showSlashCommandMenu" @update:visible="showSlashCommandMenu = $event">
      <SlashCommandMenu @select="handleSlashCommandSelect" @close="showSlashCommandMenu = false" />
    </InputPopover>
  </div>
</template>

<style scoped>
  .chat-input {
    position: relative;
    padding: 12px 14px;
    margin: auto;
    width: 75%;
    margin-bottom: 10px;
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-2xl);
    background-color: var(--bg-100);
    box-shadow: var(--shadow-md);
    transition:
      border-color 0.15s ease,
      box-shadow 0.15s ease;
  }

  .chat-input:focus-within {
    border-color: var(--border-300);
    box-shadow: var(--shadow-lg);
  }

  .context-usage-indicator {
    position: absolute;
    top: 10px;
    right: 10px;
    z-index: 10;
  }

  .command-tag {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px 4px 8px;
    margin-bottom: 8px;
    background: var(--bg-200);
    border-radius: var(--border-radius-md);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-300);
  }

  .command-tag .command-icon {
    width: 14px;
    height: 14px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .command-tag .command-icon :deep(svg) {
    width: 14px;
    height: 14px;
    stroke: var(--text-400);
    stroke-width: 2;
  }

  .command-tag .command-label {
    line-height: 1;
  }

  .command-tag .command-remove {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    padding: 0;
    margin-left: 2px;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-sm);
    cursor: pointer;
    color: var(--text-400);
  }

  .command-tag .command-remove:hover {
    background: var(--bg-300);
    color: var(--text-200);
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
    color: var(--text-200);
    font-size: 14px;
    outline: none;
    resize: none;
  }

  .message-input::-webkit-scrollbar {
    display: none;
  }

  .message-input::placeholder {
    color: var(--text-400);
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

  .native-select {
    appearance: none;
    -webkit-appearance: none;
    background: transparent;
    border: none;
    padding: 4px 20px 4px 8px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-300);
    cursor: pointer;
    outline: none;
    border-radius: var(--border-radius-md);
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%23737373' stroke-width='2'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 4px center;
    background-size: 12px;
    transition:
      background-color 0.15s ease,
      color 0.15s ease;
  }

  .native-select:hover {
    background-color: var(--color-hover);
    color: var(--text-200);
  }

  .native-select:focus {
    background-color: var(--color-hover);
  }

  .native-select option {
    background: var(--bg-200);
    color: var(--text-200);
  }

  .model-selector {
    min-width: 80px;
    max-width: 160px;
  }

  .image-upload-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-sm);
    color: var(--text-300);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .image-upload-button:hover:not(:disabled) {
    background: var(--color-hover);
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
    border-radius: var(--border-radius-sm);
    color: var(--text-300);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .database-button:hover {
    background: var(--color-hover);
    color: var(--color-warning);
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
    color: var(--color-warning);
  }

  .database-button.has-index:hover {
    background: var(--color-hover);
    color: var(--color-warning);
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
    border: 1.5px solid var(--bg-100);
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

    .native-select {
      font-size: 11px;
    }
  }

  @container (max-width: 280px) {
    .model-selector {
      min-width: 55px;
      max-width: 85px;
    }

    .input-bottom {
      gap: 4px;
    }

    .bottom-left {
      gap: 4px;
    }

    .native-select {
      font-size: 11px;
      padding: 4px 16px 4px 6px;
    }
  }
</style>
