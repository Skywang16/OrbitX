<template>
  <div class="terminal-wrapper" @mousedown="handleWrapperMouseDown">
    <TerminalLoading v-if="isLoading" />

    <div
      ref="terminalRef"
      class="terminal-container"
      :class="{ 'terminal-active': isActive, 'terminal-loading': isLoading }"
      @click="focusTerminal"
    ></div>

    <TerminalCompletion
      ref="completionRef"
      :input="inputState.currentLine"
      :working-directory="terminalEnv.workingDirectory"
      :terminal-element="terminalRef"
      :terminal-cursor-position="terminalEnv.cursorPosition"
      :is-mac="terminalEnv.isMac"
      @suggestion-change="handleSuggestionChange"
    />

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
  import { FitAddon } from '@xterm/addon-fit'
  import { WebLinksAddon } from '@xterm/addon-web-links'
  import { SearchAddon } from '@xterm/addon-search'
  import { CanvasAddon } from '@xterm/addon-canvas'
  import { LigaturesAddon } from '@xterm/addon-ligatures'
  import { Unicode11Addon } from '@xterm/addon-unicode11'
  import { Terminal } from '@xterm/xterm'

  import type { Theme } from '@/types'
  import { windowApi } from '@/api'
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
  import { useThemeStore } from '@/stores/theme'
  import { useTerminalSelection } from '@/composables/useTerminalSelection'
  import { useTerminalState } from '@/composables/useTerminalState'
  import { useTerminalSearch } from '@/composables/useTerminalSearch'
  import { useShellIntegration } from '@/composables/useShellIntegration'
  import { useTerminalOutput } from '@/composables/useTerminalOutput'
  import { TERMINAL_CONFIG } from '@/constants/terminal'
  import { useTerminalStore } from '@/stores/Terminal'
  import { useLayoutStore } from '@/stores/layout'
  import { convertThemeToXTerm, createDefaultXTermTheme } from '@/utils/themeConverter'
  import { terminalChannelApi } from '@/api/channel/terminal'

  import type { ITheme } from '@xterm/xterm'
  import TerminalCompletion from './TerminalCompletion.vue'
  import TerminalLoading from './TerminalLoading.vue'
  import SearchBox from '@/components/common/SearchBox.vue'

  // XTerm.js 样式
  import '@xterm/xterm/css/xterm.css'

  // === 组件接口定义 ===
  interface Props {
    terminalId: number // 终端唯一标识符（与后端 pane_id 一致）
    isActive: boolean // 是否为当前活跃终端
  }

  const props = defineProps<Props>()

  // === 状态管理 ===
  const terminalStore = useTerminalStore()
  const layoutStore = useLayoutStore()
  const themeStore = useThemeStore()
  const terminalSelection = useTerminalSelection()

  const { inputState, terminalEnv, updateInputLine, handleSuggestionChange } = useTerminalState()
  const { searchState, searchBoxRef, closeSearch, handleSearch, findNext, findPrevious, handleOpenTerminalSearch } =
    useTerminalSearch()
  const { handleOutputBinary: handleTerminalOutputBinary } = useTerminalOutput()

  // === 核心引用 ===
  const terminalRef = ref<HTMLElement | null>(null)
  const terminal = ref<Terminal | null>(null)
  const completionRef = ref<{ hasCompletion: () => boolean; acceptCompletion: () => string } | null>(null)

  const fitAddon = ref<FitAddon | null>(null)
  const searchAddon = ref<SearchAddon | null>(null)
  // 流式 UTF-8 解码器：仅用于 OSC 解析与状态分发，渲染走 writeUtf8
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

  // Loading 状态管理：只在“刚创建的新终端”且“尚未收到输出”时显示
  const isLoading = ref(
    typeof props.terminalId === 'number' &&
      terminalStore.isPaneNew(props.terminalId) &&
      !terminalStore.hasPaneOutput(props.terminalId)
  )
  let loadingTimer: number | null = null
  let hasReceivedData = false
  const LOADING_TIMEOUT = 5000 // 5秒超时

  const shouldDecodeTextOutput = () => {
    if (props.isActive) return true
    return terminalStore.hasOutputSubscribers(props.terminalId)
  }

  // 统一的事件资源管理
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
    terminalStore.resizeTerminal(props.terminalId, rows, cols).catch(() => {})
  }

  const processBinaryChunk = (paneId: number, bytes: Uint8Array) => {
    if (paneId !== props.terminalId || !terminal.value) return

    // 首次收到数据时停止 loading
    if (!hasReceivedData && bytes.length > 0) {
      hasReceivedData = true
      terminalStore.markPaneHasOutput(paneId)
      stopLoading()
    }

    // 直接写入 xterm
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

    // 清除之前的超时计时器
    if (loadingTimer) {
      clearTimeout(loadingTimer)
    }

    // 设置超时自动停止 loading
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

  const disposeChannelSubscription = () => {
    if (channelSub) {
      const sub = channelSub
      channelSub = null
      sub.unsubscribe().catch(() => {})
    }
  }

  const subscribeToPane = (paneId: number | null) => {
    logTerminalEvent('subscribeToPane', { paneId })
    disposeChannelSubscription()

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

  // === 性能优化 ===
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

  // Shell Integration 设置
  const shellIntegration = useShellIntegration({
    terminalId: props.terminalId,
    workingDirectory: terminalEnv.workingDirectory,
    onCwdUpdate: (cwd: string) => {
      terminalEnv.workingDirectory = cwd
    },
  })

  // === 核心功能函数 ===

  /**
   * 初始化 XTerm.js 终端实例
   * 配置终端、加载插件、设置事件监听器
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
      })

      // 处理 Unicode 宽字符与合字宽度问题（例如中文、emoji、Nerd Font 图标）
      try {
        const unicode11 = new Unicode11Addon()
        terminal.value.loadAddon(unicode11)
        terminal.value.unicode.activeVersion = '11'
      } catch (e) {
        console.warn('Unicode11 addon failed to load.', e)
      }

      // 使用 Canvas 渲染器提升性能
      try {
        const canvasAddon = new CanvasAddon()
        terminal.value.loadAddon(canvasAddon)
      } catch (e) {
        console.warn('Canvas addon failed to load, falling back to default renderer.', e)
      }

      fitAddon.value = new FitAddon() // 创建自适应大小插件实例
      terminal.value.loadAddon(fitAddon.value) // 自适应大小插件

      searchAddon.value = new SearchAddon() // 创建搜索插件实例
      terminal.value.loadAddon(searchAddon.value) // 搜索插件

      terminal.value.loadAddon(
        new WebLinksAddon((event, uri) => {
          if (event.ctrlKey || event.metaKey) {
            openUrl(uri).catch(() => {})
          }
        })
      ) // 链接点击插件

      // 先打开终端
      terminal.value.open(terminalRef.value)

      // 启用连字支持，提升编程连字与特殊字符的显示效果
      // 必须在终端打开后加载，因为连字插件需要注册字符连接器
      if (props.isActive && !ligaturesAddonLoaded) {
        try {
          const ligaturesAddon = new LigaturesAddon()
          terminal.value.loadAddon(ligaturesAddon)
          ligaturesAddonLoaded = true
        } catch (e) {
          console.warn('Ligatures addon failed to load.', e)
        }
      }

      // 加载插件与 open 之后，重新应用主题并强制刷新以确保颜色正确
      try {
        terminal.value.options.theme = xtermTheme
        if (terminal.value.rows > 0) {
          terminal.value.refresh(0, terminal.value.rows - 1)
        }
      } catch {
        // ignore
      }

      // 只有激活的终端才发送resize事件，避免非激活终端触发API调用
      trackDisposable(terminal.value.onResize(() => commitResize()))

      trackDisposable(
        terminal.value.onData(data => {
          terminalStore.writeToTerminal(props.terminalId, data).catch(() => {})
          updateInputLine(data)
          scheduleCursorPositionUpdate()
        })
      )

      trackDisposable(terminal.value.onKey(e => handleKeyDown(e.domEvent)))

      trackDisposable(terminal.value.onCursorMove(scheduleCursorPositionUpdate))
      // 移除 onScroll 事件监听，减少滚动时的性能开销

      trackDisposable(terminal.value.onSelectionChange(scheduleSelectionSync))

      // 初始尺寸适配
      resizeTerminal()
      // 使用 ResizeObserver 监听容器尺寸变化，自动适配
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
    } catch {
      if (!hasDisposed && terminal.value) {
        try {
          terminal.value.dispose()
        } catch {
          // ignore
        }
        terminal.value = null
        hasDisposed = true
      }
      fitAddon.value = null
      isXtermReady = false
      logTerminalEvent('initXterm:error')
    }
  }

  /**
   * 更新终端主题
   * 当主题设置变化时调用，优化刷新机制
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
    } catch {
      // ignore
    }
  }

  watch(
    () => themeStore.currentTheme,
    newTheme => {
      updateTerminalTheme(newTheme)
    },
    { immediate: true }
  )

  // === 事件处理器 ===

  /**
   * 初始化平台信息
   */
  const initPlatformInfo = async () => {
    try {
      terminalEnv.isMac = await windowApi.isMac()
    } catch {
      terminalEnv.isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0
    }
  }

  /**
   * 处理键盘事件，专门处理补全快捷键
   * Mac系统使用 Cmd + 右箭头键，其他系统使用 Ctrl + 右箭头键
   */
  const handleKeyDown = (event: KeyboardEvent) => {
    const isCompletionShortcut = terminalEnv.isMac
      ? event.metaKey && event.key === 'ArrowRight' // Mac: Cmd + 右箭头
      : event.ctrlKey && event.key === 'ArrowRight' // Windows/Linux: Ctrl + 右箭头

    if (isCompletionShortcut) {
      try {
        if (completionRef.value?.hasCompletion()) {
          event.preventDefault() // 阻止默认行为
          event.stopPropagation() // 阻止事件传播

          const completionText = completionRef.value.acceptCompletion()
          if (completionText && completionText.trim()) {
            acceptCompletion(completionText)
          }
        }
      } catch (error) {
        console.warn('Failed to accept completion:', error)
      }
    }
  }

  /**
   * 接受补全建议，将补全文本插入到当前输入行
   */
  const acceptCompletion = (completionText: string) => {
    if (!completionText || !completionText.trim() || !terminal.value) {
      return
    }

    try {
      inputState.currentLine += completionText
      inputState.cursorCol += completionText.length

      terminalStore.writeToTerminal(props.terminalId, completionText).catch(() => {})

      updateTerminalCursorPosition()
    } catch (error) {
      console.warn('Failed to update terminal cursor position:', error)
    }
  }

  /**
   * 处理快捷键触发的补全接受事件
   */
  const handleAcceptCompletionShortcut = () => {
    if (completionRef.value?.hasCompletion()) {
      const completionText = completionRef.value.acceptCompletion()
      if (completionText && completionText.trim()) {
        acceptCompletion(completionText)
      }
    }
  }

  /**
   * 处理清空终端事件
   */
  const handleClearTerminal = () => {
    if (terminal.value) {
      terminal.value.clear()
    }
  }

  /**
   * 处理字体大小变化事件
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
   * 处理透明度变化事件
   * 当窗口透明度改变时刷新终端显示
   */
  const handleOpacityChange = () => {
    if (!terminal.value) return

    try {
      // 刷新终端显示以确保透明度正确应用
      if (terminal.value.rows > 0) {
        terminal.value.refresh(0, terminal.value.rows - 1)
      }
    } catch (error) {
      console.warn('Failed to refresh terminal on opacity change:', error)
    }
  }

  /**
   * 聚焦终端
   * 使终端获得焦点，允许用户输入
   */
  const focusTerminal = () => {
    try {
      if (terminal.value && terminal.value.element) {
        terminal.value.focus()
      }
    } catch {
      // ignore
    }
  }

  /**
   * 处理 wrapper 的 mousedown 事件
   * 阻止从 padding 区域拖拽时触发浏览器原生的元素选择（蓝色遮罩）
   */
  const handleWrapperMouseDown = (event: MouseEvent) => {
    // 只处理点击在 padding 区域（即直接点击 wrapper 本身）的情况
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
          } catch {
            // ignore
          }
        })
      })
    } catch {
      // ignore
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
    } catch {
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

      // 内部拖拽优先
      const internalPath = layoutStore.consumeDragPath()
      if (internalPath) {
        insertPathToTerminal(internalPath)
        return
      }

      // 外部文件拖拽
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

      // 如果有 terminalId，立即开始 loading 并设置超时
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
        } catch {
          terminalEnv.workingDirectory = '/tmp'
        }
      }

      if (terminalRef.value) {
        addDomListener(terminalRef.value, 'accept-completion', handleAcceptCompletionShortcut)
        addDomListener(terminalRef.value, 'clear-terminal', handleClearTerminal)
      }

      addDomListener(document, 'font-size-change', handleFontSizeChange)

      addDomListener(document, 'open-terminal-search', handleOpenTerminalSearchEvent)

      addDomListener(window, 'opacity-changed', handleOpacityChange)

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

    // 清理 loading 相关资源
    stopLoading()

    // 清理 Tauri drag drop 监听
    if (unlistenDragDrop) {
      unlistenDragDrop()
      unlistenDragDrop = null
    }

    // 刷新解码器尾部残留，避免丢字符
    const remaining = binaryDecoder.decode()
    if (remaining) {
      shellIntegration.processTerminalOutput(remaining)
      if (props.terminalId != null) {
        terminalStore.dispatchOutputForPaneId(props.terminalId, remaining)
      }
    }

    if (terminalRef.value) {
      terminalRef.value.removeEventListener('accept-completion', handleAcceptCompletionShortcut)
      terminalRef.value.removeEventListener('clear-terminal', handleClearTerminal)
    }

    document.removeEventListener('font-size-change', handleFontSizeChange)

    document.removeEventListener('open-terminal-search', handleOpenTerminalSearchEvent)

    if (resizeTimer) clearTimeout(resizeTimer)
    if (selectionRaf) cancelAnimationFrame(selectionRaf)
    if (cursorRaf) cancelAnimationFrame(cursorRaf)

    // 防止组件卸载后仍触发Shell Integration的异步调用
    try {
      shellIntegration.dispose()
    } catch {
      // ignore
    }

    // 取消 Tauri Channel 订阅，避免后端通道残留
    disposeChannelSubscription()
    subscribedPaneId = null
    isXtermReady = false
    fitRetryCount = 0
    if (keyListener) {
      try {
        keyListener.dispose()
      } catch (_) {
        // ignore
      }
      keyListener = null
    }

    if (terminal.value) {
      try {
        terminal.value.dispose()
      } catch {
        // ignore
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
        // xterm 未就绪，等待 onMounted 中的订阅
        return
      }

      if (typeof oldId === 'number' && typeof newId === 'number' && oldId !== newId) {
        try {
          terminal.value?.reset()
        } catch {
          // ignore
        }

        // 切换到新 pane 时重置解码器与 shell integration 状态，避免历史/提示符叠加
        try {
          binaryDecoder.decode()
        } catch {
          // ignore
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
        } catch {
          // ignore
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
