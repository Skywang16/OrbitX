<template>
  <div class="terminal-wrapper">
    <div
      ref="terminalRef"
      class="terminal-container"
      :class="{ 'terminal-active': isActive }"
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
  import { terminalInitialBuffer } from '@/stores/terminalInitialBuffer'
  import { createMessage } from '@/ui'
  import { convertThemeToXTerm, createDefaultXTermTheme } from '@/utils/themeConverter'
  import { terminalChannelApi } from '@/api/channel/terminal'
  import { terminalApi } from '@/api/terminal'

  import type { ITheme } from '@xterm/xterm'
  import TerminalCompletion from './TerminalCompletion.vue'
  import SearchBox from '@/components/SearchBox.vue'

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
  const MAX_SNAPSHOT_CHARS = 512 * 1024

  let isXtermReady = false
  let initialBinaryQueue: Uint8Array[] = []
  let subscribedPaneId: number | null = null
  // VSCode-like hydration gating
  let isHydrating = false
  const PROMPT_SUPPRESS_MS = 600
  const promptSuppressUntil = new Map<number, number>()
  let lastEmittedResize: { rows: number; cols: number } | null = null
  let fitRetryCount = 0
  let hasRestoredHistory = false

  const paneSnapshots = new Map<number, string>()

  const logTerminalEvent = (...args: unknown[]) => {
    if (import.meta.env.DEV) {
      // eslint-disable-next-line no-console
      console.debug(`[Terminal ${props.terminalId ?? 'unknown'}]`, ...args)
    }
  }

  let hasDisposed = false
  let channelSub: { unsubscribe: () => Promise<void> } | null = null
  let keyListener: { dispose: () => void } | null = null

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

  const applyHistoryContent = (content: string) => {
    logTerminalEvent('applyHistoryContent:start', {
      length: content?.length ?? 0,
      hasTerminal: Boolean(terminal.value),
    })
    if (!terminal.value) {
      return
    }

    if (!content) {
      logTerminalEvent('applyHistoryContent:skip-empty')
      resizeTerminal()
      return
    }

    try {
      terminal.value.clear()
    } catch {
      // ignore
    }

    const normalized = content.replace(/\r?\n/g, '\r\n')
    terminal.value.write(normalized)
    if (terminal.value.rows > 0) {
      terminal.value.refresh(0, terminal.value.rows - 1)
    }
    terminal.value.scrollToBottom()
    resizeTerminal()
    logTerminalEvent('applyHistoryContent:done', {
      rows: terminal.value.rows,
      cols: terminal.value.cols,
    })
  }

  const updatePaneSnapshot = (paneId: number, text: string, mode: 'append' | 'replace' = 'append') => {
    logTerminalEvent('updatePaneSnapshot', {
      paneId,
      mode,
      incomingLength: text?.length ?? 0,
    })
    if (mode === 'replace') {
      if (!text) {
        paneSnapshots.set(paneId, '')
        return
      }

      const trimmed = text.length > MAX_SNAPSHOT_CHARS ? text.slice(text.length - MAX_SNAPSHOT_CHARS) : text
      paneSnapshots.set(paneId, trimmed)
      return
    }

    if (!text) {
      return
    }

    const existing = paneSnapshots.get(paneId) ?? ''
    let combined = existing + text
    if (combined.length > MAX_SNAPSHOT_CHARS) {
      combined = combined.slice(combined.length - MAX_SNAPSHOT_CHARS)
    }
    paneSnapshots.set(paneId, combined)
    logTerminalEvent('updatePaneSnapshot:stored', {
      paneId,
      length: combined.length,
    })
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
    if (paneId !== props.terminalId) {
      return
    }

    const text = binaryDecoder.decode(bytes, { stream: true })
    if (text) {
      const stripped = stripAnsi(text).trimEnd()
      const hasNewline = /\r|\n/.test(stripped)

      // Drop prompt-only chunk within suppress window (right after hydration when history already has a prompt)
      const now = Date.now()
      const until = promptSuppressUntil.get(paneId) ?? 0
      if (now < until && !hasNewline && isPromptLike(stripped)) {
        logTerminalEvent('processBinaryChunk:skip-prompt-in-window', { paneId, prompt: stripped.slice(0, 50) })
        return
      }

      // Extra guard: if chunk equals the last non-empty line and looks like prompt, skip
      if (!hasNewline && isPromptLike(stripped)) {
        const lastLine = getLastNonEmptyLine(paneSnapshots.get(paneId) ?? '')
        const lastStripped = stripAnsi(lastLine).trimEnd()
        if (lastStripped && lastStripped === stripped) {
          logTerminalEvent('processBinaryChunk:skip-duplicate-prompt', { paneId, prompt: stripped.slice(0, 50) })
          return
        }
      }

      shellIntegration.processTerminalOutput(text)
      terminalStore.dispatchOutputForPaneId(paneId, text)
      updatePaneSnapshot(paneId, text)
      logTerminalEvent('processBinaryChunk', {
        paneId,
        byteLength: bytes.length,
        decodedLength: text.length,
      })
    }

    if (terminal.value) {
      handleTerminalOutputBinary(terminal.value, bytes)
    }
  }

  const flushQueuedBinaryChunks = () => {
    logTerminalEvent('flushQueuedBinaryChunks:start', {
      isXtermReady,
      hasTerminal: Boolean(terminal.value),
      queued: initialBinaryQueue.length,
      subscribedPaneId,
      currentPaneId: props.terminalId,
      hasRestoredHistory,
    })
    if (!isXtermReady || !terminal.value || subscribedPaneId !== props.terminalId) {
      if (subscribedPaneId !== props.terminalId) {
        initialBinaryQueue = []
      }
      return
    }

    // Only flush if we haven't restored history, or local queue has data (VSCode pattern)
    if (hasRestoredHistory && initialBinaryQueue.length === 0) {
      logTerminalEvent('flushQueuedBinaryChunks:skip-already-hydrated')
      return
    }

    // Prefer draining the global VSCode-like initial buffer to avoid duplicates
    const globalChunks =
      typeof props.terminalId === 'number' ? terminalInitialBuffer.takeAndClear(props.terminalId) : []
    let processedCount = 0
    if (globalChunks.length > 0) {
      // Clear local queue to avoid replaying the same chunks twice
      initialBinaryQueue = []
      for (const chunk of globalChunks) {
        processBinaryChunk(props.terminalId, chunk)
        processedCount += 1
      }
    } else if (initialBinaryQueue.length > 0) {
      const chunks = initialBinaryQueue
      initialBinaryQueue = []
      for (const chunk of chunks) {
        processBinaryChunk(props.terminalId, chunk)
        processedCount += 1
      }
    } else {
      return
    }

    resizeTerminal()
    logTerminalEvent('flushQueuedBinaryChunks:done', {
      processed: processedCount,
    })
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
      initialBinaryQueue = []
      return
    }

    subscribedPaneId = paneId
    initialBinaryQueue = []

    try {
      channelSub = terminalChannelApi.subscribeBinary(paneId, bytes => {
        if (subscribedPaneId !== paneId) {
          return
        }

        // During hydration or when xterm isn't ready, buffer bytes and return
        if (isHydrating || !terminal.value || !isXtermReady) {
          initialBinaryQueue.push(bytes)
          // Also store in a global buffer with TTL, emulating VSCode's _initialDataEvents
          try {
            terminalInitialBuffer.append(paneId, bytes)
          } catch (e) {
            // ignore append errors
          }
          logTerminalEvent('subscribeBinary:buffering', {
            paneId,
            byteLength: bytes.length,
            queueSize: initialBinaryQueue.length,
            isHydrating,
            isXtermReady,
            hasTerminal: Boolean(terminal.value),
          })
          return
        }

        processBinaryChunk(paneId, bytes)
      })
    } catch (e) {
      console.warn('Failed to subscribe terminal channel:', e)
    }
  }

  // === ANSI & Prompt Utils (for deduplication) ===
  // eslint-disable-next-line no-control-regex
  const ANSI_RE = /\x1B\[[0-9;?]*[ -/]*[@-~]|\x1B\][^\x07]*(?:\x07|\x1B\\)|\x1B\(\)[0-2AB]|[\x00-\x08\x0B-\x1F\x7F]/g
  const stripAnsi = (s: string) => s.replace(ANSI_RE, '')
  const isPromptLike = (line: string) => {
    const p = stripAnsi(line).trimEnd()
    if (!p) return false
    const endsWithPromptChar = /(?:^|\s)[%$#>]\s?$/.test(p) || /➜\s?$/.test(p)
    const hasContext = /@/.test(p) || /\b~\b/.test(p) || /\//.test(p)
    return endsWithPromptChar && hasContext
  }
  const getLastNonEmptyLine = (text: string) => {
    const ls = text.split(/\r?\n/)
    for (let i = ls.length - 1; i >= 0; i--) {
      if (ls[i].trim() !== '') return ls[i]
    }
    return ''
  }

  const sanitizeHistory = (data: string) => {
    const lines = data.split(/\r?\n/)

    // 1) 去除尾部空白行
    let lastIndex = lines.length - 1
    while (lastIndex >= 0 && lines[lastIndex].trim() === '') {
      lastIndex -= 1
    }

    if (lastIndex < 0) {
      return ''
    }

    // 2) 去除尾部重复行（不少主题会回显最后一行多次）
    const lastLine0 = lines[lastIndex]
    let cursor = lastIndex - 1
    while (cursor >= 0 && lines[cursor] === lastLine0) {
      lines.splice(cursor, 1)
      cursor -= 1
      lastIndex -= 1
    }

    // 3) 组装并保持原始结尾换行行为（不强制加换行，不移除提示符）
    const sanitized = lines.slice(0, Math.max(0, lastIndex + 1)).join('\r\n')
    const keepNewline = /\r?\n$/.test(data)
    return keepNewline ? `${sanitized}\r\n` : sanitized
  }

  const restoreTerminalBuffer = async (paneId: number, force = false) => {
    logTerminalEvent('restoreTerminalBuffer:start', {
      paneId,
      hasRestoredHistory,
      hasTerminal: Boolean(terminal.value),
    })
    if (hasRestoredHistory && !force) {
      return
    }

    if (!terminal.value) {
      return
    }

    try {
      isHydrating = true
      // 1) 拉取后端历史
      const rawContent = await terminalApi.getTerminalBuffer(paneId)
      const history = rawContent ? sanitizeHistory(rawContent) : ''
      logTerminalEvent('restoreTerminalBuffer:history', {
        paneId,
        length: history.length,
      })

      // 2) 取出 xterm 未 ready 期间的预缓存（二进制）并清空
      const preChunks = terminalInitialBuffer.takeAndClear(paneId)
      const preText = (() => {
        if (!preChunks.length) return ''
        // 独立解码器，避免污染主解码器状态
        const total = preChunks.reduce((s, b) => s + b.length, 0)
        const buf = new Uint8Array(total)
        let off = 0
        for (const c of preChunks) {
          buf.set(c, off)
          off += c.length
        }
        return new TextDecoder('utf-8', { fatal: false }).decode(buf)
      })()
      logTerminalEvent('restoreTerminalBuffer:preText', { bytes: preChunks.length, length: preText.length })

      // 3) 智能去重：只有当 preText 包含新提示符时，才删除历史末尾的旧提示符
      let historyDedup = history
      const lastLineOfHistory = getLastNonEmptyLine(history)
      const isHistoryEndsWithPrompt = lastLineOfHistory && isPromptLike(lastLineOfHistory)

      // 检查 preText 是否以提示符开头
      const firstLineOfPre = preText ? preText.split(/\r?\n/)[0] : ''
      const isPreStartsWithPrompt = firstLineOfPre && isPromptLike(firstLineOfPre)

      // 只有当两边都有提示符时才去重（删除历史末尾的旧提示符，保留 preText 的新提示符）
      if (isHistoryEndsWithPrompt && isPreStartsWithPrompt) {
        const lines = history.split(/\r?\n/)
        let lastIdx = lines.length - 1
        while (lastIdx >= 0 && lines[lastIdx].trim() === '') {
          lastIdx -= 1
        }
        if (lastIdx >= 0) {
          lines.splice(lastIdx, 1)
        }
        const historyEndsWithNewline = /\r?\n$/.test(history)
        historyDedup = lines.join('\r\n') + (historyEndsWithNewline ? '\r\n' : '')
        logTerminalEvent('restoreTerminalBuffer:dedup-both-have-prompt', {
          historyPrompt: stripAnsi(lastLineOfHistory).slice(0, 50),
          prePrompt: stripAnsi(firstLineOfPre).slice(0, 50),
        })
      } else if (isHistoryEndsWithPrompt && !isPreStartsWithPrompt) {
        // 历史有提示符但 preText 没有（或为空），保留历史的提示符
        logTerminalEvent('restoreTerminalBuffer:keep-history-prompt', {
          hasPreText: preText.length > 0,
        })
      }

      // 确保历史末尾有换行，避免preText粘连
      if (historyDedup && preText && !/\r?\n$/.test(historyDedup)) {
        historyDedup += '\r\n'
      }

      const combined = historyDedup + preText
      applyHistoryContent(combined)
      updatePaneSnapshot(paneId, combined, 'replace')
      // 清空本地队列，避免刚才合并过的字节再次 flush
      initialBinaryQueue = []

      // If combined ends with a prompt, suppress the very next prompt-only chunk within a short window
      const lastLine = getLastNonEmptyLine(combined)
      if (lastLine && isPromptLike(lastLine)) {
        promptSuppressUntil.set(paneId, Date.now() + PROMPT_SUPPRESS_MS)
      } else {
        promptSuppressUntil.delete(paneId)
      }
    } catch (error) {
      console.warn('Failed to restore terminal buffer:', error)
    } finally {
      isHydrating = false
      hasRestoredHistory = true
      resizeTerminal()
      flushQueuedBinaryChunks()
      logTerminalEvent('restoreTerminalBuffer:done', {
        snapshotLength: paneSnapshots.get(paneId)?.length ?? 0,
      })
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
      await initPlatformInfo()
      subscribeToPane(typeof props.terminalId === 'number' ? props.terminalId : null)
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

      if (typeof props.terminalId === 'number') {
        const cachedSnapshot = paneSnapshots.get(props.terminalId)
        if (cachedSnapshot) {
          applyHistoryContent(sanitizeHistory(cachedSnapshot))
        }
      }

      await shellIntegration.initShellIntegration(terminal.value)
      await nextTick()

      if (typeof props.terminalId === 'number') {
        hasRestoredHistory = false
        try {
          await restoreTerminalBuffer(props.terminalId)
        } finally {
          flushQueuedBinaryChunks()
        }
      } else {
        flushQueuedBinaryChunks()
      }
    })
  })

  onBeforeUnmount(() => {
    if (hasDisposed) return
    hasDisposed = true
    logTerminalEvent('onBeforeUnmount')

    // 刷新解码器尾部残留，避免丢字符
    const remaining = binaryDecoder.decode()
    if (remaining) {
      shellIntegration.processTerminalOutput(remaining)
      if (props.terminalId != null) {
        terminalStore.dispatchOutputForPaneId(props.terminalId, remaining)
        updatePaneSnapshot(props.terminalId, remaining)
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
    initialBinaryQueue = []
    isXtermReady = false
    fitRetryCount = 0
    hasRestoredHistory = false
    if (props.terminalId != null) {
      paneSnapshots.delete(props.terminalId)
      try {
        terminalInitialBuffer.clear(props.terminalId)
      } catch {
        // ignore
      }
    }

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
          const snapshot = typeof props.terminalId === 'number' ? paneSnapshots.get(props.terminalId) : undefined
          if (!hasRestoredHistory) {
            if (snapshot) {
              logTerminalEvent('watch:isActive->true:applySnapshotOnce', { length: snapshot.length })
              applyHistoryContent(sanitizeHistory(snapshot))
            } else if (typeof props.terminalId === 'number') {
              // 快照为空时，强制尝试一次历史恢复，确保能立即显示提示符/历史
              logTerminalEvent('watch:isActive->true:forceRestore')
              void (async () => {
                try {
                  await restoreTerminalBuffer(props.terminalId, true)
                } finally {
                  flushQueuedBinaryChunks()
                }
              })()
            }
          }
          focusTerminal()
          resizeTerminal()
          // 只在未恢复过时或没有内容时 flush，一般切回无需重复 flush
          if (!hasRestoredHistory) {
            flushQueuedBinaryChunks()
          }
        })
      } else {
        logTerminalEvent('watch:isActive->false')
      }
    },
    { immediate: true }
  )

  // Re-subscribe when paneId changes
  watch(
    () => props.terminalId,
    (newId, oldId) => {
      logTerminalEvent('watch:terminalId', { newId, oldId })
      disposeChannelSubscription()

      const remaining = binaryDecoder.decode()
      if (remaining) {
        shellIntegration.processTerminalOutput(remaining)
        if (typeof oldId === 'number') {
          terminalStore.dispatchOutputForPaneId(oldId, remaining)
          updatePaneSnapshot(oldId, remaining)
        }
      }

      if (typeof newId === 'number') {
        shellIntegration.updateTerminalId(newId)
        subscribedPaneId = newId
        initialBinaryQueue = []
        subscribeToPane(newId)
        const cachedSnapshot = paneSnapshots.get(newId)
        if (cachedSnapshot) {
          applyHistoryContent(sanitizeHistory(cachedSnapshot))
        }
        hasRestoredHistory = false
        if (isXtermReady) {
          void (async () => {
            try {
              await nextTick()
              await restoreTerminalBuffer(newId)
            } finally {
              flushQueuedBinaryChunks()
            }
          })()
        }
      } else {
        subscribedPaneId = null
        initialBinaryQueue = []
        shellIntegration.resetState()
        hasRestoredHistory = false
      }

      lastEmittedResize = null
      fitRetryCount = 0

      // Clear global initial buffer for the old pane to avoid leaks and duplicates
      if (typeof oldId === 'number') {
        try {
          terminalInitialBuffer.clear(oldId)
        } catch {
          // ignore
        }
      }
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
  }

  .terminal-container {
    height: 100%;
    width: 100%;
    background: var(--bg-100);
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
  }

  :global(.xterm-link-layer a) {
    text-decoration: underline !important;
    text-decoration-style: dotted !important;
    text-decoration-color: var(--text-400) !important;
  }
</style>
