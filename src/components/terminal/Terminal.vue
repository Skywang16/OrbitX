<template>
  <div class="terminal-wrapper">
    <TerminalLoading v-if="isLoading" />

    <div
      ref="terminalRef"
      class="terminal-container"
      :class="{ 'terminal-active': isActive, 'terminal-loading': isLoading }"
      @click="focusTerminal"
      @dragover="handleDragOver"
      @dragleave="handleDragLeave"
      @drop="handleDrop"
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
  import { useThemeStore } from '@/stores/theme'
  import { useTerminalSelection } from '@/composables/useTerminalSelection'
  import { useTerminalState } from '@/composables/useTerminalState'
  import { useTerminalSearch } from '@/composables/useTerminalSearch'
  import { useShellIntegration } from '@/composables/useShellIntegration'
  import { useTerminalOutput } from '@/composables/useTerminalOutput'
  import { TERMINAL_CONFIG } from '@/constants/terminal'
  import { useTerminalStore } from '@/stores/Terminal'
  import { createMessage } from '@/ui'
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
  const emit = defineEmits<{
    (e: 'input', data: string): void // 用户输入事件
    (e: 'resize', rows: number, cols: number): void // 终端大小变化事件
  }>()

  // === 状态管理 ===
  const terminalStore = useTerminalStore()
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
  const binaryDecoder = new TextDecoder('utf-8', { fatal: false })
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

  // Loading 状态管理
  // 如果有 terminalId，初始状态就显示 loading
  const isLoading = ref(typeof props.terminalId === 'number')
  let loadingTimer: number | null = null
  let hasReceivedData = false
  const LOADING_TIMEOUT = 5000 // 5秒超时

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
    if (!terminal.value || !props.isActive) {
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
    emit('resize', rows, cols)
  }

  const processBinaryChunk = (paneId: number, bytes: Uint8Array) => {
    if (paneId !== props.terminalId || !terminal.value) return

    // 首次收到数据时停止 loading
    if (!hasReceivedData && bytes.length > 0) {
      hasReceivedData = true
      stopLoading()
    }

    // 直接写入 xterm
    handleTerminalOutputBinary(terminal.value, bytes)

    // 可选：扩展功能（shell integration, 状态分发）
    const text = binaryDecoder.decode(bytes, { stream: true })
    if (text) {
      shellIntegration.processTerminalOutput(text)
      terminalStore.dispatchOutputForPaneId(paneId, text)
    }
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
      channelSub
        .unsubscribe()
        .catch(() => {})
        .finally(() => {
          channelSub = null
        })
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

    // 如果还没有开始 loading（例如从其他 pane 切换过来），则开始
    if (!isLoading.value) {
      startLoading()
    }

    try {
      // 更新 shell integration
      shellIntegration.updateTerminalId(paneId)
    } catch (error) {
      console.warn('Failed to update shell integration terminal id:', error)
    }

    try {
      // 🔑 订阅后自动接收 replay + 实时数据（后端已实现）
      channelSub = terminalChannelApi.subscribeBinary(paneId, bytes => {
        if (subscribedPaneId !== paneId) return
        processBinaryChunk(paneId, bytes)
      })
    } catch (e) {
      console.warn('Failed to subscribe terminal channel:', e)
      // 订阅失败时停止 loading
      stopLoading()
    }
  }

  // === 性能优化 ===
  let resizeTimer: number | null = null

  const MAX_SELECTION_LENGTH = 4096

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
      try {
        const ligaturesAddon = new LigaturesAddon()
        terminal.value.loadAddon(ligaturesAddon)
      } catch (e) {
        console.warn('Ligatures addon failed to load.', e)
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
          emit('input', data)
          updateInputLine(data)
          updateTerminalCursorPosition()
        })
      )

      trackDisposable(terminal.value.onKey(e => handleKeyDown(e.domEvent)))

      trackDisposable(terminal.value.onCursorMove(updateTerminalCursorPosition))
      // 移除 onScroll 事件监听，减少滚动时的性能开销

      trackDisposable(terminal.value.onSelectionChange(syncSelection))

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

      emit('input', completionText)

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
   * 调整终端大小
   * 根据容器大小自动调整终端尺寸
   */
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

      resizeTimer = window.setTimeout(() => {
        try {
          fitAddon.value?.fit()
          commitResize()
          logTerminalEvent('resizeTerminal:fit', {
            rows: terminal.value?.rows,
            cols: terminal.value?.cols,
          })
        } catch {
          // ignore
        }
      }, 50)
    } catch {
      // ignore
    }
  }

  /**
   * 更新终端光标位置
   */
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

      // 后备方案：手动计算光标位置
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

  const handleGoToPath = (path: string) => {
    const cleanPath = path.trim().replace(/^["']|["']$/g, '')
    emit('input', `cd "${cleanPath}"\n`)
    createMessage.success(`切换到: ${cleanPath}`)
  }

  const handleFileDrop = async (filePath: string) => {
    try {
      const directory = await windowApi.handleFileOpen(filePath)
      handleGoToPath(directory)
    } catch {
      createMessage.error('无法处理拖拽的文件')
    }
  }

  /**
   * 处理拖拽悬停事件
   */
  const handleDragOver = (event: DragEvent) => {
    event.preventDefault()
    event.dataTransfer!.dropEffect = 'copy'
  }

  /**
   * 处理拖拽离开事件
   */
  const handleDragLeave = (event: DragEvent) => {
    event.preventDefault()
  }

  /**
   * 处理文件拖拽放置事件
   */
  const handleDrop = async (event: DragEvent) => {
    event.preventDefault()

    const files = event.dataTransfer?.files

    if (files && files.length > 0) {
      const file = files[0]

      let filePath = ''
      if ('path' in file && file.path) {
        filePath = file.path as string
      } else {
        filePath = file.name
      }

      await handleFileDrop(filePath)
    }
  }

  // === Event Handlers for Terminal ===

  // === Lifecycle ===
  onMounted(() => {
    nextTick(async () => {
      logTerminalEvent('onMounted:init')

      // 如果有 terminalId，立即开始 loading 并设置超时
      if (typeof props.terminalId === 'number') {
        startLoading()
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

    // 防止组件卸载后仍触发Shell Integration的异步调用
    try {
      shellIntegration.dispose()
    } catch {
      // ignore
    }

    terminalStore.unregisterResizeCallback(props.terminalId)

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
        })
      } else {
        logTerminalEvent('watch:isActive->false')
      }
    },
    { immediate: true }
  )

  watch(
    () => props.terminalId,
    newId => {
      logTerminalEvent('watch:terminalId', { newId })

      if (!isXtermReady) {
        // xterm 未就绪，等待 onMounted 中的订阅
        return
      }

      if (typeof newId === 'number') {
        subscribeToPane(newId)
      } else {
        subscribeToPane(null)
        shellIntegration.resetState()
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
    padding: 10px 10px 0 10px;
    /* 透明背景 */
    background: transparent;
  }

  .terminal-container {
    height: 100%;
    width: 100%;
    /* 完全透明，继承窗口统一背景 */
    background: transparent;
    overflow: hidden;
  }

  .terminal-container :global(.xterm) {
    height: 100% !important;
  }

  .terminal-container :global(.xterm .xterm-viewport) {
    height: 100% !important;
    /* 优化滚动性能 */
    overscroll-behavior: contain;
    scroll-behavior: auto;
    background-color: transparent !important;
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
