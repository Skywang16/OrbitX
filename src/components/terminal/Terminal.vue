<template>
  <div class="terminal-wrapper" @mousedown="handleWrapperMouseDown">
    <TerminalLoading v-if="isLoading" />

    <div
      ref="terminalRef"
      class="terminal-container"
      :class="{ 'terminal-active': isActive, 'terminal-loading': isLoading }"
      @click="focusTerminal"
    ></div>

    <SearchBox
      :visible="searchState.visible"
      @close="() => closeSearch(searchAddon)"
      @search="(query, options) => handleSearch(terminal, searchAddon, query, options)"
      @find-next="() => findNext(searchAddon)"
      @find-previous="() => findPrevious(searchAddon)"
      ref="searchBoxRef"
    />
  </div>
</template>

<script setup lang="ts">
  import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'

  import { openUrl } from '@tauri-apps/plugin-opener'
  import { CanvasAddon } from '@xterm/addon-canvas'
  import { FitAddon } from '@xterm/addon-fit'
  import { LigaturesAddon } from '@xterm/addon-ligatures'
  import { SearchAddon } from '@xterm/addon-search'
  import { Unicode11Addon } from '@xterm/addon-unicode11'
  import { WebLinksAddon } from '@xterm/addon-web-links'
  import { Terminal } from '@xterm/xterm'

  import { windowApi } from '@/api'
  import { terminalChannelApi } from '@/api/channel/terminal'
  import { useShellIntegration } from '@/composables/useShellIntegration'
  import { useTerminalOutput } from '@/composables/useTerminalOutput'
  import { useTerminalSearch } from '@/composables/useTerminalSearch'
  import { useTerminalSelection } from '@/composables/useTerminalSelection'
  import { useTerminalState } from '@/composables/useTerminalState'
  import { TERMINAL_CONFIG } from '@/constants/terminal'
  import { useTerminalStore } from '@/stores/Terminal'
  import { useLayoutStore } from '@/stores/layout'
  import { useThemeStore } from '@/stores/theme'
  import type { Theme } from '@/types'
  import { convertThemeToXTerm, createDefaultXTermTheme } from '@/utils/themeConverter'
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'

  import SearchBox from '@/components/common/SearchBox.vue'
  import type { ITheme } from '@xterm/xterm'
  import TerminalLoading from './TerminalLoading.vue'

  import '@xterm/xterm/css/xterm.css'

  // === Component Interface ===
  interface Props {
    terminalId: number // Terminal unique identifier (matches backend pane_id)
    isActive: boolean // Whether this is the active terminal
    disableStdin?: boolean // Disable user input
  }

  const props = defineProps<Props>()

  // === State Management ===
  const terminalStore = useTerminalStore()
  const layoutStore = useLayoutStore()
  const themeStore = useThemeStore()
  const terminalSelection = useTerminalSelection()

  const { terminalEnv, updateInputLine } = useTerminalState()
  const { searchState, searchBoxRef, closeSearch, handleSearch, findNext, findPrevious, handleOpenTerminalSearch } =
    useTerminalSearch()
  const { handleOutputBinary: handleTerminalOutputBinary } = useTerminalOutput()

  // === Core References ===
  const terminalRef = ref<HTMLElement | null>(null)
  const terminal = ref<Terminal | null>(null)
  const fitAddon = ref<FitAddon | null>(null)
  const searchAddon = ref<SearchAddon | null>(null)
  // Streaming UTF-8 decoder: for OSC parsing and state dispatch only, rendering uses writeUtf8
  let binaryDecoder = new TextDecoder('utf-8', { fatal: false })
  let resizeObserver: ResizeObserver | null = null

  const MAX_INITIAL_FIT_RETRIES = 20

  let isXtermReady = false
  let subscribedPaneId: number | null = null
  let lastEmittedResize: { rows: number; cols: number } | null = null
  let fitRetryCount = 0

  const logTerminalEvent = (...args: unknown[]) => {
    if (import.meta.env.DEV) {
      // eslint-disable-next-line no-console
      console.debug(`[Terminal ${props.terminalId ?? 'unknown'}]`, ...args)
    }
  }

  let hasDisposed = false
  let channelSub: { unsubscribe: () => Promise<void> } | null = null
  let keyListener: { dispose: () => void } | null = null
  let ligaturesAddonLoaded = false

  // Loading state: only shown for newly created terminals without output yet
  const isLoading = ref(
    typeof props.terminalId === 'number' &&
      terminalStore.isPaneNew(props.terminalId) &&
      !terminalStore.hasPaneOutput(props.terminalId)
  )
  let loadingTimer: number | null = null
  let hasReceivedData = false
  const LOADING_TIMEOUT = 5000 // 5 second timeout

  const shouldDecodeTextOutput = () => {
    if (props.isActive) return true
    return terminalStore.hasOutputSubscribers(props.terminalId)
  }

  // Unified event resource management
  const disposers: Array<() => void> = []
  const addDomListener = (target: EventTarget, type: string, handler: EventListenerOrEventListenerObject) => {
    target.addEventListener(type, handler as EventListener)
    disposers.push(() => target.removeEventListener(type, handler as EventListener))
  }
  const trackDisposable = (d: { dispose: () => void } | undefined | null) => {
    if (d && typeof d.dispose === 'function') {
      disposers.push(() => d.dispose())
    }
  }

  const commitResize = () => {
    if (!terminal.value) {
      return
    }

    const rows = terminal.value.rows
    const cols = terminal.value.cols

    if (rows <= 0 || cols <= 0) {
      return
    }

    if (lastEmittedResize && lastEmittedResize.rows === rows && lastEmittedResize.cols === cols) {
      return
    }

    lastEmittedResize = { rows, cols }
    terminalStore.resizeTerminal(props.terminalId, rows, cols).catch(error => {
      console.warn(`Failed to resize terminal ${props.terminalId}:`, error)
    })
  }

  const processBinaryChunk = (paneId: number, bytes: Uint8Array) => {
    if (paneId !== props.terminalId || !terminal.value) return

    // Stop loading when first data is received
    if (!hasReceivedData && bytes.length > 0) {
      hasReceivedData = true
      terminalStore.markPaneHasOutput(paneId)
      stopLoading()
    }

    // Write directly to xterm
    handleTerminalOutputBinary(terminal.value, bytes)

    if (!shouldDecodeTextOutput()) return
    const text = binaryDecoder.decode(bytes, { stream: true })
    if (!text) return
    shellIntegration.processTerminalOutput(text)
    terminalStore.dispatchOutputForPaneId(paneId, text)
  }

  const startLoading = () => {
    isLoading.value = true
    hasReceivedData = false

    // Clear previous timeout timer
    if (loadingTimer) {
      clearTimeout(loadingTimer)
    }

    // Set timeout to auto-stop loading
    loadingTimer = window.setTimeout(() => {
      stopLoading()
    }, LOADING_TIMEOUT)
  }

  const stopLoading = () => {
    isLoading.value = false

    if (loadingTimer) {
      clearTimeout(loadingTimer)
      loadingTimer = null
    }
  }

  const disposeChannelSubscription = async () => {
    if (channelSub) {
      const sub = channelSub
      channelSub = null
      await sub.unsubscribe().catch(error => {
        console.warn('Failed to unsubscribe terminal channel:', error)
      })
    }
  }

  const subscribeToPane = async (paneId: number | null) => {
    logTerminalEvent('subscribeToPane', { paneId })
    await disposeChannelSubscription()

    if (paneId == null) {
      subscribedPaneId = null
      stopLoading()
      return
    }

    subscribedPaneId = paneId

    if (terminalStore.isPaneNew(paneId) && !terminalStore.hasPaneOutput(paneId)) {
      startLoading()
    } else {
      stopLoading()
    }

    try {
      shellIntegration.updateTerminalId(paneId)
    } catch (error) {
      console.warn('Failed to update shell integration terminal id:', error)
    }

    try {
      channelSub = terminalChannelApi.subscribeBinary(paneId, bytes => {
        if (subscribedPaneId !== paneId) return
        processBinaryChunk(paneId, bytes)
      })
    } catch (e) {
      console.warn('Failed to subscribe terminal channel:', e)
      stopLoading()
    }
  }

  // === Performance Optimization ===
  let resizeTimer: number | null = null

  const MAX_SELECTION_LENGTH = 4096

  let selectionRaf: number | null = null
  const scheduleSelectionSync = () => {
    if (selectionRaf) return
    selectionRaf = requestAnimationFrame(() => {
      selectionRaf = null
      syncSelection()
    })
  }

  const syncSelection = () => {
    try {
      const selectedText = terminal.value?.getSelection()

      if (!selectedText || !selectedText.trim()) {
        terminalSelection.clearSelection()
        return
      }

      const truncatedText =
        selectedText.length > MAX_SELECTION_LENGTH ? `${selectedText.slice(0, MAX_SELECTION_LENGTH)}...` : selectedText
      const selection = terminal.value?.getSelectionPosition()
      const startLine = selection ? selection.start.y + 1 : 1
      const endLine = selection ? selection.end.y + 1 : undefined

      terminalSelection.setSelectedText(truncatedText, startLine, endLine, terminalEnv.workingDirectory)
    } catch (error) {
      console.warn('Selection processing error:', error)
      terminalSelection.clearSelection()
    }
  }

  // Shell Integration settings
  const shellIntegration = useShellIntegration({
    terminalId: props.terminalId,
    workingDirectory: terminalEnv.workingDirectory,
    onCwdUpdate: (cwd: string) => {
      terminalEnv.workingDirectory = cwd
    },
  })

  // === Core Functions ===

  /**
   * Initialize XTerm.js terminal instance
   * Configure terminal, load plugins, and set up event listeners
   */
  const initXterm = async () => {
    try {
      if (!terminalRef.value) {
        return
      }

      logTerminalEvent('initXterm:start')
      isXtermReady = false
      fitRetryCount = 0
      lastEmittedResize = null

      const currentTheme = themeStore.currentTheme
      const xtermTheme = currentTheme ? convertThemeToXTerm(currentTheme) : createDefaultXTermTheme()

      terminal.value = new Terminal({
        ...TERMINAL_CONFIG,
        fontWeight: 400,
        fontWeightBold: 700,
        theme: xtermTheme,
        disableStdin: props.disableStdin ?? false,
      })

      // Handle Unicode wide characters and ligature width (e.g., CJK, emoji, Nerd Font icons)
      try {
        const unicode11 = new Unicode11Addon()
        terminal.value.loadAddon(unicode11)
        terminal.value.unicode.activeVersion = '11'
      } catch (e) {
        console.warn('Unicode11 addon failed to load.', e)
      }

      // Use Canvas renderer for better performance
      try {
        const canvasAddon = new CanvasAddon()
        terminal.value.loadAddon(canvasAddon)
      } catch (e) {
        console.warn('Canvas addon failed to load, falling back to default renderer.', e)
      }

      fitAddon.value = new FitAddon()
      terminal.value.loadAddon(fitAddon.value)

      searchAddon.value = new SearchAddon()
      terminal.value.loadAddon(searchAddon.value)

      terminal.value.loadAddon(
        new WebLinksAddon((event, uri) => {
          if (event.ctrlKey || event.metaKey) {
            openUrl(uri).catch(error => {
              console.warn(`Failed to open terminal link '${uri}':`, error)
            })
          }
        })
      )

      terminal.value.open(terminalRef.value)

      // Enable ligature support for better programming ligatures and special character display
      // Must load after terminal opens as ligature plugin needs to register character connectors
      if (props.isActive && !ligaturesAddonLoaded) {
        try {
          const ligaturesAddon = new LigaturesAddon()
          terminal.value.loadAddon(ligaturesAddon)
          ligaturesAddonLoaded = true
        } catch (e) {
          console.warn('Ligatures addon failed to load.', e)
        }
      }

      // After loading plugins and opening, reapply theme and force refresh to ensure correct colors
      try {
        terminal.value.options.theme = xtermTheme
        if (terminal.value.rows > 0) {
          terminal.value.refresh(0, terminal.value.rows - 1)
        }
      } catch (error) {
        console.warn('Failed to refresh terminal theme after initialization:', error)
      }

      // Only active terminals send resize events to avoid API calls from inactive terminals
      trackDisposable(terminal.value.onResize(() => commitResize()))

      trackDisposable(
        terminal.value.onData(data => {
          if (props.disableStdin) return
          terminalStore.writeToTerminal(props.terminalId, data).catch(error => {
            console.warn(`Failed to write terminal input to pane ${props.terminalId}:`, error)
          })
          updateInputLine(data)
          scheduleCursorPositionUpdate()
        })
      )

      trackDisposable(terminal.value.onCursorMove(scheduleCursorPositionUpdate))
      // Removed onScroll listener to reduce performance overhead during scrolling

      trackDisposable(terminal.value.onSelectionChange(scheduleSelectionSync))

      // Initial size fitting
      resizeTerminal()
      // Use ResizeObserver to watch container size changes and auto-fit
      if (typeof ResizeObserver !== 'undefined' && terminalRef.value) {
        resizeObserver = new ResizeObserver(() => {
          resizeTerminal()
        })
        resizeObserver.observe(terminalRef.value)
      }
      focusTerminal()
      isXtermReady = true
      commitResize()
      logTerminalEvent('initXterm:ready', {
        rows: terminal.value.rows,
        cols: terminal.value.cols,
      })
    } catch (error) {
      if (!hasDisposed && terminal.value) {
        try {
          terminal.value.dispose()
        } catch (disposeError) {
          console.warn('Failed to dispose terminal after initialization error:', disposeError)
        }
        terminal.value = null
        hasDisposed = true
      }
      fitAddon.value = null
      isXtermReady = false
      console.error('Failed to initialize xterm terminal:', error)
      logTerminalEvent('initXterm:error')
    }
  }

  /**
   * Update terminal theme
   * Called when theme settings change, with optimized refresh mechanism
   */
  const updateTerminalTheme = (newThemeData: Theme | null) => {
    if (!terminal.value) return

    try {
      let xtermTheme: ITheme
      if (newThemeData) {
        xtermTheme = convertThemeToXTerm(newThemeData)
      } else {
        xtermTheme = createDefaultXTermTheme()
      }

      terminal.value.options.theme = xtermTheme

      if (terminal.value.rows > 0) {
        terminal.value.refresh(0, terminal.value.rows - 1)
      }
    } catch (error) {
      console.warn('Failed to update terminal theme:', error)
    }
  }

  watch(
    () => themeStore.currentTheme,
    newTheme => {
      updateTerminalTheme(newTheme)
    },
    { immediate: true }
  )

  watch(
    () => props.disableStdin,
    newVal => {
      if (terminal.value) {
        terminal.value.options.disableStdin = !!newVal
      }
    }
  )

  // === Event Handlers ===

  /**
   * Initialize platform information
   */
  const initPlatformInfo = async () => {
    try {
      terminalEnv.isMac = await windowApi.isMac()
    } catch (error) {
      console.warn('Failed to query macOS platform state, using browser fallback:', error)
      terminalEnv.isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0
    }
  }

  /**
   * Handle clear terminal event
   */
  const handleClearTerminal = () => {
    if (terminal.value) {
      terminal.value.clear()
    }
  }

  /**
   * Handle font size change event
   */
  const handleFontSizeChange = (event: Event) => {
    const customEvent = event as CustomEvent<{ action: 'increase' | 'decrease' }>
    if (!terminal.value || !fitAddon.value) return

    const action = customEvent.detail?.action
    if (action === 'increase') {
      const currentFontSize = terminal.value.options.fontSize || 12
      const newFontSize = Math.min(currentFontSize + 1, 24)
      terminal.value.options.fontSize = newFontSize
      nextTick(() => {
        fitAddon.value?.fit()
      })
    } else if (action === 'decrease') {
      const currentFontSize = terminal.value.options.fontSize || 12
      const newFontSize = Math.max(currentFontSize - 1, 8)
      terminal.value.options.fontSize = newFontSize
      nextTick(() => {
        fitAddon.value?.fit()
      })
    }
  }

  const handleOpenTerminalSearchEvent = () => {
    handleOpenTerminalSearch(props.isActive, searchAddon.value)
  }

  /**
   * Focus terminal to allow user input
   */
  const focusTerminal = () => {
    try {
      if (terminal.value && terminal.value.element) {
        terminal.value.focus()
      }
    } catch (error) {
      console.warn('Failed to focus terminal:', error)
    }
  }

  /**
   * Handle wrapper mousedown event
   * Prevent browser native element selection (blue mask) when dragging from padding area
   */
  const handleWrapperMouseDown = (event: MouseEvent) => {
    // Only handle clicks in padding area (direct clicks on wrapper itself)
    if (event.target === event.currentTarget) {
      event.preventDefault()
      focusTerminal()
    }
  }

  const resizeTerminal = () => {
    try {
      if (!terminal.value || !fitAddon.value || !terminalRef.value) {
        return
      }

      const { clientWidth, clientHeight } = terminalRef.value
      if ((clientWidth === 0 || clientHeight === 0) && !props.isActive) {
        logTerminalEvent('resizeTerminal:skip-hidden')
        return
      }

      if (clientWidth === 0 || clientHeight === 0) {
        if (fitRetryCount < MAX_INITIAL_FIT_RETRIES) {
          fitRetryCount += 1
          requestAnimationFrame(() => {
            resizeTerminal()
          })
        }
        logTerminalEvent('resizeTerminal:pending', {
          clientWidth,
          clientHeight,
          retry: fitRetryCount,
        })
        return
      }

      fitRetryCount = 0

      if (resizeTimer) {
        clearTimeout(resizeTimer)
      }

      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          try {
            fitAddon.value?.fit()
            commitResize()
          } catch (error) {
            console.warn('Failed to fit terminal after resize:', error)
          }
        })
      })
    } catch (error) {
      console.warn('Failed to resize terminal:', error)
    }
  }

  const updateTerminalCursorPosition = () => {
    if (!props.isActive || !terminal.value || !terminalRef.value) {
      return
    }

    try {
      const buffer = terminal.value.buffer.active

      const cursorElement = terminalRef.value.querySelector('.xterm-cursor')
      if (cursorElement) {
        const cursorRect = cursorElement.getBoundingClientRect()
        terminalEnv.cursorPosition = {
          x: cursorRect.left,
          y: cursorRect.top,
        }
        return
      }

      const xtermScreen = terminalRef.value.querySelector('.xterm-screen')
      if (!xtermScreen) return

      const terminalCols = terminal.value.cols
      const terminalRows = terminal.value.rows
      const screenRect = xtermScreen.getBoundingClientRect()

      const charWidth = screenRect.width / terminalCols
      const lineHeight = screenRect.height / terminalRows

      const x = screenRect.left + buffer.cursorX * charWidth
      const y = screenRect.top + buffer.cursorY * lineHeight

      terminalEnv.cursorPosition = { x, y }
    } catch (error) {
      console.warn('Failed to update terminal cursor position, resetting to origin:', error)
      terminalEnv.cursorPosition = { x: 0, y: 0 }
    }
  }

  let cursorRaf: number | null = null
  const scheduleCursorPositionUpdate = () => {
    if (cursorRaf) return
    cursorRaf = requestAnimationFrame(() => {
      cursorRaf = null
      updateTerminalCursorPosition()
    })
  }

  const insertPathToTerminal = (path: string) => {
    const quoted = path.includes(' ') ? `"${path}"` : path
    terminalStore.writeToTerminal(props.terminalId, quoted)
  }

  let unlistenDragDrop: (() => void) | null = null

  const setupDragDropListener = async () => {
    const webview = getCurrentWebviewWindow()
    unlistenDragDrop = await webview.onDragDropEvent(event => {
      if (event.payload.type !== 'drop' || !props.isActive) return

      // Internal drag takes priority
      const internalPath = layoutStore.consumeDragPath()
      if (internalPath) {
        insertPathToTerminal(internalPath)
        return
      }

      // External file drag
      const paths = event.payload.paths
      if (paths.length > 0) {
        insertPathToTerminal(paths[0])
      }
    })
  }

  // === Event Handlers for Terminal ===

  // === Lifecycle ===
  onMounted(() => {
    nextTick(async () => {
      logTerminalEvent('onMounted:init')

      // If terminalId exists, start loading immediately and set timeout
      if (terminalStore.isPaneNew(props.terminalId) && !terminalStore.hasPaneOutput(props.terminalId)) {
        startLoading()
      } else {
        stopLoading()
      }

      await initPlatformInfo()
      await initXterm()

      const tmeta = terminalStore.terminals.find(t => t.id === props.terminalId)
      if (tmeta && tmeta.cwd) {
        terminalEnv.workingDirectory = tmeta.cwd
      } else {
        try {
          const dir: string = await windowApi.getHomeDirectory()
          terminalEnv.workingDirectory = dir
        } catch (error) {
          console.warn('Failed to read home directory for terminal, using /tmp:', error)
          terminalEnv.workingDirectory = '/tmp'
        }
      }

      if (terminalRef.value) {
        addDomListener(terminalRef.value, 'clear-terminal', handleClearTerminal)
      }

      addDomListener(document, 'font-size-change', handleFontSizeChange)

      addDomListener(document, 'open-terminal-search', handleOpenTerminalSearchEvent)

      await setupDragDropListener()

      await shellIntegration.initShellIntegration(terminal.value)
      await nextTick()

      if (typeof props.terminalId === 'number') {
        subscribeToPane(props.terminalId)
      }
    })
  })

  onBeforeUnmount(() => {
    if (hasDisposed) return
    hasDisposed = true
    logTerminalEvent('onBeforeUnmount')

    // Clean up loading-related resources
    stopLoading()

    // Clean up Tauri drag drop listener
    if (unlistenDragDrop) {
      unlistenDragDrop()
      unlistenDragDrop = null
    }

    // Flush decoder tail to avoid character loss
    const remaining = binaryDecoder.decode()
    if (remaining) {
      shellIntegration.processTerminalOutput(remaining)
      if (props.terminalId != null) {
        terminalStore.dispatchOutputForPaneId(props.terminalId, remaining)
      }
    }

    if (terminalRef.value) {
      terminalRef.value.removeEventListener('clear-terminal', handleClearTerminal)
    }

    document.removeEventListener('font-size-change', handleFontSizeChange)

    document.removeEventListener('open-terminal-search', handleOpenTerminalSearchEvent)

    if (resizeTimer) clearTimeout(resizeTimer)
    if (selectionRaf) cancelAnimationFrame(selectionRaf)
    if (cursorRaf) cancelAnimationFrame(cursorRaf)

    // Prevent async Shell Integration calls after component unmount
    try {
      shellIntegration.dispose()
    } catch (error) {
      console.warn('Failed to dispose shell integration:', error)
    }

    // Cancel Tauri Channel subscription to avoid backend channel residue
    disposeChannelSubscription().catch(error => {
      console.warn('Failed to dispose terminal channel subscription:', error)
    })
    subscribedPaneId = null
    isXtermReady = false
    fitRetryCount = 0
    if (keyListener) {
      try {
        keyListener.dispose()
      } catch (error) {
        console.warn('Failed to dispose terminal key listener:', error)
      }
      keyListener = null
    }

    if (terminal.value) {
      try {
        terminal.value.dispose()
      } catch (error) {
        console.warn('Failed to dispose terminal instance:', error)
      }
      terminal.value = null
    }

    if (resizeObserver && terminalRef.value) {
      resizeObserver.unobserve(terminalRef.value)
      resizeObserver.disconnect()
      resizeObserver = null
    }

    fitAddon.value = null
  })

  // === Watchers ===
  watch(
    () => props.isActive,
    isActive => {
      if (isActive) {
        logTerminalEvent('watch:isActive->true')
        nextTick(() => {
          focusTerminal()
          resizeTerminal()
          if (terminal.value && !ligaturesAddonLoaded) {
            try {
              const ligaturesAddon = new LigaturesAddon()
              terminal.value.loadAddon(ligaturesAddon)
              ligaturesAddonLoaded = true
            } catch (e) {
              console.warn('Ligatures addon failed to load.', e)
            }
          }
        })
      } else {
        logTerminalEvent('watch:isActive->false')
      }
    },
    { immediate: true }
  )

  watch(
    () => props.terminalId,
    (newId, oldId) => {
      logTerminalEvent('watch:terminalId', { newId, oldId })

      if (!isXtermReady) {
        // xterm not ready, wait for subscription in onMounted
        return
      }

      if (typeof oldId === 'number' && typeof newId === 'number' && oldId !== newId) {
        try {
          terminal.value?.reset()
        } catch (error) {
          console.warn('Failed to reset terminal during pane switch:', error)
        }

        // Reset decoder and shell integration state when switching panes to avoid history/prompt stacking
        try {
          binaryDecoder.decode()
        } catch (error) {
          console.warn('Failed to flush terminal decoder during pane switch:', error)
        }
        binaryDecoder = new TextDecoder('utf-8', { fatal: false })
        shellIntegration.resetState()
        terminalSelection.clearSelection()
      }

      if (typeof newId === 'number') {
        subscribeToPane(newId)
      } else {
        subscribeToPane(null)
        shellIntegration.resetState()
        terminalSelection.clearSelection()
        try {
          terminal.value?.reset()
        } catch (error) {
          console.warn('Failed to reset terminal after clearing pane binding:', error)
        }
      }

      lastEmittedResize = null
      fitRetryCount = 0
    }
  )

  // === Expose ===
  defineExpose({
    focusTerminal,
    resizeTerminal,
  })
</script>

<style scoped>
  .terminal-wrapper {
    position: relative;
    height: 100%;
    width: 100%;
    padding: 10px;
    box-sizing: border-box;
    background: transparent;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .terminal-container {
    flex: 1;
    width: 100%;
    background: transparent;
    overflow: hidden;
    min-height: 0;
  }

  .terminal-container :global(.xterm) {
    width: 100%;
    height: 100%;
  }

  .terminal-container :global(.xterm .xterm-viewport) {
    height: 100% !important;
    overscroll-behavior: contain;
    scroll-behavior: auto;
    background-color: transparent !important;
    transform: translateZ(0);
    will-change: scroll-position;
  }

  .terminal-container :global(.xterm .xterm-screen canvas) {
    transform: translateZ(0);
  }

  :global(.xterm-link-layer a) {
    text-decoration: underline !important;
    text-decoration-style: dotted !important;
    text-decoration-color: var(--text-400) !important;
  }

  .terminal-container.terminal-loading {
    opacity: 0;
  }
</style>
