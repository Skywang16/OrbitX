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

    <!-- 补全组件 -->
    <TerminalCompletion
      ref="completionRef"
      :input="inputState.currentLine"
      :working-directory="terminalEnv.workingDirectory"
      :terminal-element="terminalRef"
      :terminal-cursor-position="terminalEnv.cursorPosition"
      :is-mac="terminalEnv.isMac"
      @suggestion-change="handleSuggestionChange"
    />

    <!-- 提示消息 -->
    <XMessage :visible="toast.visible" :message="toast.message" :type="toast.type" @close="closeToast" />

    <!-- 搜索组件 -->
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
  // Vue 核心功能
  import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'

  // 第三方库
  import { openUrl } from '@tauri-apps/plugin-opener'
  import { FitAddon } from '@xterm/addon-fit'
  import { WebLinksAddon } from '@xterm/addon-web-links'
  import { SearchAddon } from '@xterm/addon-search'
  import { Terminal } from '@xterm/xterm'

  // 项目内部模块
  import type { Theme } from '@/types'
  import { windowApi } from '@/api'
  import { useThemeStore } from '@/stores/theme'
  import { useTerminalSelection } from '@/composables/useTerminalSelection'
  import { useTerminalState } from '@/composables/useTerminalState'
  import { useTerminalSearch } from '@/composables/useTerminalSearch'
  import { useShellIntegration } from '@/composables/useShellIntegration'
  import { useTerminalOutput } from '@/composables/useTerminalOutput'
  import { useTerminalEvents } from '@/composables/useTerminalEvents'
  import { TERMINAL_CONFIG } from '@/constants/terminal'
  import { useTerminalStore } from '@/stores/Terminal'
  import { XMessage } from '@/ui/components'
  import { convertThemeToXTerm, createDefaultXTermTheme } from '@/utils/themeConverter'

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

  // 使用新的composables
  const { inputState, terminalEnv, toast, showToast, closeToast, updateInputLine, handleSuggestionChange } =
    useTerminalState()
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
        // 容器缺失，放弃初始化
        return
      }

      // 获取当前主题
      const currentTheme = themeStore.currentTheme
      const xtermTheme = currentTheme ? convertThemeToXTerm(currentTheme) : createDefaultXTermTheme()

      // 创建终端实例，应用配置和主题
      terminal.value = new Terminal({
        ...TERMINAL_CONFIG,
        // 明确指定数值以匹配 XTerm 的 FontWeight 类型
        fontWeight: 400,
        fontWeightBold: 700,
        theme: xtermTheme,
      })

      // 创建并加载插件
      fitAddon.value = new FitAddon() // 创建自适应大小插件实例
      terminal.value.loadAddon(fitAddon.value) // 自适应大小插件

      searchAddon.value = new SearchAddon() // 创建搜索插件实例
      terminal.value.loadAddon(searchAddon.value) // 搜索插件

      terminal.value.loadAddon(
        new WebLinksAddon((event, uri) => {
          // 支持 Ctrl+点击（Windows/Linux）或 Cmd+点击（Mac）打开链接
          if (event.ctrlKey || event.metaKey) {
            openUrl(uri).catch(() => {})
          }
        })
      ) // 链接点击插件
      terminal.value.open(terminalRef.value)

      // 设置核心事件监听
      terminal.value.onResize(({ rows, cols }) => emit('resize', rows, cols)) // 大小变化

      // 输入监听
      terminal.value.onData(data => {
        emit('input', data)
        updateInputLine(data)
        updateTerminalCursorPosition()
      })

      // 使用 XTerm 的 onKey 处理补全快捷键
      keyListener = terminal.value.onKey(e => handleKeyDown(e.domEvent))

      // 监听终端滚动事件，实时更新光标位置
      const viewportElement = terminalRef.value.querySelector('.xterm-viewport')
      if (viewportElement) {
        viewportElement.addEventListener('scroll', updateTerminalCursorPosition)
      }

      // 监听终端内容变化，确保光标位置准确
      terminal.value.onCursorMove(updateTerminalCursorPosition)
      terminal.value.onScroll(updateTerminalCursorPosition)

      // 监听文本选择事件 - 简化逻辑
      terminal.value.onSelectionChange(() => {
        const selectedText = terminal.value?.getSelection()

        if (!selectedText?.trim()) {
          terminalSelection.clearSelection()
          return
        }

        // 尝试获取选择位置信息
        const selection = terminal.value?.getSelectionPosition()
        const startLine = selection ? selection.start.y + 1 : 1 // xterm行号从0开始
        const endLine = selection ? selection.end.y + 1 : undefined

        terminalSelection.setSelectedText(selectedText, startLine, endLine, terminalEnv.workingDirectory)
      })

      // 初始化终端状态
      resizeTerminal()
      focusTerminal()
    } catch {
      // 清理可能已创建的资源（注意与卸载生命周期的竞争条件）
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
        // 如果没有主题数据，使用默认主题
        xtermTheme = createDefaultXTermTheme()
      }

      // 更新主题选项
      terminal.value.options.theme = xtermTheme

      // 简单刷新，避免频繁刷新导致闪烁
      if (terminal.value.rows > 0) {
        terminal.value.refresh(0, terminal.value.rows - 1)
      }
    } catch {
      // ignore
    }
  }

  // 监听主题变化 - 使用防抖优化，减少频繁更新
  watch(
    () => themeStore.currentTheme,
    newTheme => {
      // 清除之前的定时器
      if (timers.themeUpdate) {
        clearTimeout(timers.themeUpdate)
      }

      // 使用防抖，避免频繁更新
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
      // 降级到浏览器检测
      terminalEnv.isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0
    }
  }

  /**
   * 处理键盘事件，专门处理补全快捷键
   * Mac系统使用 Cmd + 右箭头键，其他系统使用 Ctrl + 右箭头键
   */
  const handleKeyDown = (event: KeyboardEvent) => {
    // 根据操作系统检查相应的修饰键组合
    const isCompletionShortcut = terminalEnv.isMac
      ? event.metaKey && event.key === 'ArrowRight' // Mac: Cmd + 右箭头
      : event.ctrlKey && event.key === 'ArrowRight' // Windows/Linux: Ctrl + 右箭头

    if (isCompletionShortcut) {
      try {
        // 检查补全组件是否存在且有可用的补全建议
        if (completionRef.value?.hasCompletion()) {
          event.preventDefault() // 阻止默认行为
          event.stopPropagation() // 阻止事件传播

          // 调用补全组件的接受方法
          const completionText = completionRef.value.acceptCompletion()
          if (completionText && completionText.trim()) {
            acceptCompletion(completionText)
          }
        }
        // 如果没有补全建议，让事件正常传播，不做任何处理
      } catch {
        // 发生错误时不阻止默认行为，让键盘事件正常处理
      }
    }
  }

  /**
   * 接受补全建议，将补全文本插入到当前输入行
   */
  const acceptCompletion = (completionText: string) => {
    // 边界情况检查
    if (!completionText || !completionText.trim() || !terminal.value) {
      return
    }

    try {
      // 更新当前输入行状态
      inputState.currentLine += completionText
      inputState.cursorCol += completionText.length

      // 将补全文本发送到终端，这会显示在终端中
      emit('input', completionText)

      // 更新光标位置
      updateTerminalCursorPosition()

      // 可选：显示简短的成功反馈（可以根据需要启用）
      // showToast('补全已接受', 'success')
    } catch {
      // 发生错误时尝试恢复状态
      // 这里可以添加状态恢复逻辑，但通常不需要
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
      // 增大字体
      const currentFontSize = terminal.value.options.fontSize || 12
      const newFontSize = Math.min(currentFontSize + 1, 24)
      terminal.value.options.fontSize = newFontSize
      nextTick(() => {
        fitAddon.value?.fit()
      })
    } else if (action === 'decrease') {
      // 减小字体
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
        // 使用防抖避免频繁调整大小
        if (timers.resize) {
          clearTimeout(timers.resize)
        }

        timers.resize = window.setTimeout(() => {
          try {
            fitAddon.value?.fit()
            // 尺寸变化后无条件清空缓存，避免使用旧的字符宽高数据
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

      // 尝试直接从XTerm的DOM结构获取光标元素
      const cursorElement = terminalRef.value.querySelector('.xterm-cursor')
      if (cursorElement) {
        const cursorRect = cursorElement.getBoundingClientRect()
        terminalEnv.cursorPosition = {
          x: cursorRect.left,
          y: cursorRect.top,
        }
        return
      }

      // 如果没有光标元素，使用更精确的字符尺寸计算
      const xtermScreen = terminalRef.value.querySelector('.xterm-screen')
      if (!xtermScreen) return

      // 计算字符尺寸 - 使用终端实际尺寸除以行列数
      const terminalCols = terminal.value.cols
      const terminalRows = terminal.value.rows
      const screenRect = xtermScreen.getBoundingClientRect()

      const charWidth = screenRect.width / terminalCols
      const lineHeight = screenRect.height / terminalRows

      // 计算光标位置，基于屏幕区域而不是整个容器
      const x = screenRect.left + buffer.cursorX * charWidth
      const y = screenRect.top + buffer.cursorY * lineHeight

      terminalEnv.cursorPosition = { x, y }
    } catch {
      // 设置默认位置
      terminalEnv.cursorPosition = { x: 0, y: 0 }
    }
  }

  const handleGoToPath = (path: string) => {
    const cleanPath = path.trim().replace(/^["']|["']$/g, '')
    emit('input', `cd "${cleanPath}"\n`)
    showToast(`切换到: ${cleanPath}`, 'success')
  }

  const handleFileDrop = async (filePath: string) => {
    try {
      const directory = await windowApi.handleFileOpen(filePath)
      handleGoToPath(directory)
    } catch {
      showToast('无法处理拖拽的文件', 'error')
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
      // 处理第一个文件
      const file = files[0]

      // 在 Tauri 中，文件对象有 path 属性
      let filePath = ''
      if ('path' in file && file.path) {
        filePath = file.path as string
      } else {
        // 降级到文件名（可能不是完整路径）
        filePath = file.name
      }

      await handleFileDrop(filePath)
    }
  }

  // === Event Handlers for Terminal ===
  const handleOutput = (data: string) => {
    handleTerminalOutput(terminal.value, data, shellIntegration.processTerminalOutput)
  }

  // 使用 Composable 自动管理事件监听器的生命周期
  useTerminalEvents(props.terminalId, {
    onOutput: handleOutput,
    onExit: (exitCode: number | null) => handleExit(terminal.value, exitCode),
  })

  // === Lifecycle ===
  onMounted(() => {
    nextTick(async () => {
      // 初始化平台信息
      await initPlatformInfo()

      // 初始化终端（主题系统已在应用启动时初始化）
      await initXterm()

      // 初始化工作目录 - 优先使用终端状态中保存的工作目录
      const tmeta = terminalStore.terminals.find(t => t.id === props.terminalId)
      if (tmeta && tmeta.cwd) {
        terminalEnv.workingDirectory = tmeta.cwd
      } else {
        // 如果没有保存的工作目录，使用系统默认
        windowApi
          .getHomeDirectory()
          .then((dir: string) => {
            terminalEnv.workingDirectory = dir
          })
          .catch(() => {
            terminalEnv.workingDirectory = '/tmp'
          })
      }

      // 注册到终端store的resize回调，避免每个终端都监听window resize
      terminalStore.registerResizeCallback(props.terminalId, resizeTerminal)

      // 添加快捷键事件监听
      if (terminalRef.value) {
        terminalRef.value.addEventListener('accept-completion', handleAcceptCompletionShortcut)
        terminalRef.value.addEventListener('clear-terminal', handleClearTerminal)
      }

      // 添加全局字体大小变化监听
      document.addEventListener('font-size-change', handleFontSizeChange)

      // 添加终端搜索事件监听
      document.addEventListener('open-terminal-search', () =>
        handleOpenTerminalSearch(props.isActive, searchAddon.value)
      )

      // 初始化shell integration（静默模式）
      await shellIntegration.initShellIntegration(terminal.value)
    })
  })

  onBeforeUnmount(() => {
    if (hasDisposed) return
    hasDisposed = true

    // 清理快捷键事件监听
    if (terminalRef.value) {
      terminalRef.value.removeEventListener('accept-completion', handleAcceptCompletionShortcut)
      terminalRef.value.removeEventListener('clear-terminal', handleClearTerminal)
    }

    // 清理全局字体大小变化监听
    document.removeEventListener('font-size-change', handleFontSizeChange)

    // 清理终端搜索事件监听
    document.removeEventListener('open-terminal-search', () =>
      handleOpenTerminalSearch(props.isActive, searchAddon.value)
    )

    // 清理主题监听器
    // 主题监听器为全局单例，不在组件层面清理，避免影响其他实例

    // 清理所有定时器
    if (timers.resize) clearTimeout(timers.resize)
    if (timers.themeUpdate) clearTimeout(timers.themeUpdate)

    // 清理输出处理
    cleanupOutput()

    // 从终端store注销resize回调
    terminalStore.unregisterResizeCallback(props.terminalId)

    // 清理键盘事件监听器
    if (keyListener) {
      try {
        keyListener.dispose()
      } catch (_) {
        // ignore
      }
      keyListener = null
    }

    // 清理滚动事件监听器
    const viewportElement = terminalRef.value?.querySelector('.xterm-viewport')
    if (viewportElement) {
      viewportElement.removeEventListener('scroll', updateTerminalCursorPosition)
    }

    // 安全地清理终端实例
    if (terminal.value) {
      try {
        terminal.value.dispose()
      } catch {
        // ignore
      }
      terminal.value = null
    }

    // 清理插件引用
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
          resizeTerminal() // resize会触发必要的重绘，不需要额外的refresh
        })
      }
    },
    { immediate: true }
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
    /* 确保为绝对定位的补全组件提供定位上下文 */
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
