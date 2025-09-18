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
  import { WebglAddon } from '@xterm/addon-webgl'
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
  import SearchBox from '@/components/SearchBox.vue'

  // XTerm.js 样式
  import '@xterm/xterm/css/xterm.css'

  // === 组件接口定义 ===
  interface Props {
    terminalId: string // 终端唯一标识符
    backendId: number | null // 后端进程ID
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
  const { handleOutput: handleTerminalOutput, handleExit, cleanup: cleanupOutput } = useTerminalOutput()

  // === 核心引用 ===
  const terminalRef = ref<HTMLElement | null>(null)
  const terminal = ref<Terminal | null>(null)
  const completionRef = ref<{ hasCompletion: () => boolean; acceptCompletion: () => string } | null>(null)

  const fitAddon = ref<FitAddon | null>(null)
  const searchAddon = ref<SearchAddon | null>(null)

  let hasDisposed = false
  let keyListener: { dispose: () => void } | null = null
  let channelSub: { unsubscribe: () => Promise<void> } | null = null

  // === 性能优化 ===
  const timers = {
    resize: null as number | null,
    themeUpdate: null as number | null,
  }

  const styleCache = ref<{
    charWidth: number
    lineHeight: number
    paddingLeft: number
    paddingTop: number
  } | null>(null)

  // Shell Integration 设置
  const shellIntegration = useShellIntegration({
    terminalId: props.terminalId,
    backendId: props.backendId,
    workingDirectory: terminalEnv.workingDirectory,
    onCwdUpdate: (cwd: string) => {
      terminalEnv.workingDirectory = cwd
    },
    onTerminalCwdUpdate: terminalStore.updateTerminalCwd,
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

      // 尝试启用 WebGL 渲染器以减少闪烁并提升性能，若不支持则自动回退
      try {
        const webglAddon = new WebglAddon()
        terminal.value.loadAddon(webglAddon)
      } catch (e) {
        console.warn('WebGL addon failed to load, falling back to default renderer.', e)
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

      // 加载插件与 open 之后，重新应用主题并强制刷新以确保 WebGL 下颜色正确
      try {
        terminal.value.options.theme = xtermTheme
        if (terminal.value.rows > 0) {
          terminal.value.refresh(0, terminal.value.rows - 1)
        }
      } catch {
        // ignore
      }

      terminal.value.onResize(({ rows, cols }) => emit('resize', rows, cols)) // 大小变化

      terminal.value.onData(data => {
        emit('input', data)
        updateInputLine(data)
        updateTerminalCursorPosition()
      })

      keyListener = terminal.value.onKey(e => handleKeyDown(e.domEvent))

      const viewportElement = terminalRef.value.querySelector('.xterm-viewport')
      if (viewportElement) {
        viewportElement.addEventListener('scroll', updateTerminalCursorPosition)
      }

      terminal.value.onCursorMove(updateTerminalCursorPosition)
      terminal.value.onScroll(updateTerminalCursorPosition)

      terminal.value.onSelectionChange(() => {
        const selectedText = terminal.value?.getSelection()

        if (!selectedText?.trim()) {
          terminalSelection.clearSelection()
          return
        }

        const selection = terminal.value?.getSelectionPosition()
        const startLine = selection ? selection.start.y + 1 : 1 // xterm行号从0开始
        const endLine = selection ? selection.end.y + 1 : undefined

        terminalSelection.setSelectedText(selectedText, startLine, endLine, terminalEnv.workingDirectory)
      })

      resizeTerminal()
      focusTerminal()
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
      if (timers.themeUpdate) {
        clearTimeout(timers.themeUpdate)
      }

      timers.themeUpdate = window.setTimeout(() => {
        updateTerminalTheme(newTheme)
      }, 16) // 16ms 防抖，与输出刷新频率保持一致
    },
    { immediate: true } // 移除深度监听，只在主题对象引用变化时更新
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
      if (terminal.value && fitAddon.value && terminalRef.value) {
        if (timers.resize) {
          clearTimeout(timers.resize)
        }

        timers.resize = window.setTimeout(() => {
          try {
            fitAddon.value?.fit()
            styleCache.value = null
          } catch {
            // ignore
          }
        }, 50) // 减少防抖时间，提高响应性
      }
    } catch {
      // ignore
    }
  }

  /**
   * 更新终端光标位置
   * 使用更精确的方法计算光标在屏幕上的坐标位置
   */
  const updateTerminalCursorPosition = () => {
    try {
      if (!terminal.value || !terminalRef.value) return

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
  const handleOutput = (data: string) => {
    handleTerminalOutput(terminal.value, data, shellIntegration.processTerminalOutput)
  }

  // === Lifecycle ===
  onMounted(() => {
    nextTick(async () => {
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
        terminalRef.value.addEventListener('accept-completion', handleAcceptCompletionShortcut)
        terminalRef.value.addEventListener('clear-terminal', handleClearTerminal)
      }

      document.addEventListener('font-size-change', handleFontSizeChange)

      document.addEventListener('open-terminal-search', () =>
        handleOpenTerminalSearch(props.isActive, searchAddon.value)
      )

      await shellIntegration.initShellIntegration(terminal.value)

      // Subscribe to terminal output via Tauri Channel (binary streaming)
      if (props.backendId != null) {
        try {
          channelSub = terminalChannelApi.subscribe(props.backendId, text => {
            handleOutput(text)
            // 同时分发给已注册的回调（如 ShellTool）
            terminalStore.dispatchOutputForBackendId(props.backendId, text)
          })
        } catch (e) {
          console.warn('Failed to subscribe terminal channel:', e)
        }
      }
    })
  })

  onBeforeUnmount(() => {
    if (hasDisposed) return
    hasDisposed = true

    if (terminalRef.value) {
      terminalRef.value.removeEventListener('accept-completion', handleAcceptCompletionShortcut)
      terminalRef.value.removeEventListener('clear-terminal', handleClearTerminal)
    }

    document.removeEventListener('font-size-change', handleFontSizeChange)

    document.removeEventListener('open-terminal-search', () =>
      handleOpenTerminalSearch(props.isActive, searchAddon.value)
    )

    if (timers.resize) clearTimeout(timers.resize)
    if (timers.themeUpdate) clearTimeout(timers.themeUpdate)

    cleanupOutput()

    // 防止组件卸载后仍触发Shell Integration的异步调用
    try {
      shellIntegration.dispose()
    } catch {
      // ignore
    }

    terminalStore.unregisterResizeCallback(props.terminalId)

    // 取消 Tauri Channel 订阅，避免后端通道残留
    if (channelSub) {
      channelSub
        .unsubscribe()
        .catch(() => {})
        .finally(() => {
          channelSub = null
        })
    }

    if (keyListener) {
      try {
        keyListener.dispose()
      } catch (_) {
        // ignore
      }
      keyListener = null
    }

    const viewportElement = terminalRef.value?.querySelector('.xterm-viewport')
    if (viewportElement) {
      viewportElement.removeEventListener('scroll', updateTerminalCursorPosition)
    }

    if (terminal.value) {
      try {
        terminal.value.dispose()
      } catch {
        // ignore
      }
      terminal.value = null
    }

    fitAddon.value = null
    styleCache.value = null
  })

  // === Watchers ===
  watch(
    () => props.isActive,
    isActive => {
      if (isActive) {
        nextTick(() => {
          focusTerminal()
          resizeTerminal()
        })
      }
    },
    { immediate: true }
  )

  // Re-subscribe when backendId changes
  watch(
    () => props.backendId,
    newId => {
      // cleanup previous
      if (channelSub) {
        channelSub
          .unsubscribe()
          .catch(() => {})
          .finally(() => {
            channelSub = null
          })
      }
      // subscribe new
      if (newId != null) {
        try {
          channelSub = terminalChannelApi.subscribe(newId, text => {
            handleOutput(text)
          })
        } catch (e) {
          console.warn('Failed to subscribe terminal channel:', e)
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
    contain: layout style;
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
  }

  :global(.xterm-link-layer a) {
    text-decoration: underline !important;
    text-decoration-style: dotted !important;
    text-decoration-color: var(--text-400) !important;
  }
</style>
