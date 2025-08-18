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
  </div>
</template>

<script setup lang="ts">
  // Vue 核心功能
  import { nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'

  // 第三方库
  import { openPath } from '@tauri-apps/plugin-opener'
  import { FitAddon } from '@xterm/addon-fit'
  import { WebLinksAddon } from '@xterm/addon-web-links'
  import { Terminal } from '@xterm/xterm'

  // 项目内部模块
  import type { Theme } from '@/types/theme'
  import { windowApi } from '@/api'
  import { useTheme } from '@/composables/useTheme'
  import { TERMINAL_CONFIG } from '@/constants/terminal'
  import { useTerminalStore } from '@/stores/Terminal'
  import { XMessage } from '@/ui/components'
  import { convertThemeToXTerm, createDefaultXTermTheme } from '@/utils/themeConverter'
  import { invoke } from '@tauri-apps/api/core'
  import type { ITheme } from '@xterm/xterm'
  import TerminalCompletion from './TerminalCompletion.vue'

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
  const terminalStore = useTerminalStore() // 终端状态管理
  const themeStore = useTheme() // 主题管理

  // === 核心引用 ===
  const terminalRef = ref<HTMLElement | null>(null) // 终端容器DOM引用
  const terminal = ref<Terminal | null>(null) // XTerm.js 实例
  const completionRef = ref<{ hasCompletion: () => boolean; acceptCompletion: () => string } | null>(null)

  const fitAddon = ref<FitAddon | null>(null) // 终端自适应大小插件

  // 防止重复清理的标记
  let hasDisposed = false
  let keyListener: { dispose: () => void } | null = null

  // === 终端状态 ===
  // 合并输入相关状态
  const inputState = reactive({
    currentLine: '', // 当前输入行内容
    cursorCol: 0, // 光标列位置
    suggestion: '', // 当前补全建议
  })

  // 合并终端环境状态
  const terminalEnv = reactive({
    workingDirectory: '/tmp', // 当前工作目录
    cursorPosition: { x: 0, y: 0 }, // 终端光标屏幕坐标
    isMac: false, // 是否为Mac系统
  })

  // === UI 状态 ===

  // 提示消息状态
  const toast = reactive({
    visible: false, // 是否显示提示
    message: '', // 提示消息内容
    type: 'success' as 'success' | 'error', // 提示类型
  })

  // === 性能优化 ===
  // 合并定时器管理
  const timers = {
    resize: null as number | null,
    themeUpdate: null as number | null,
    outputFlush: null as number | null,
  }

  // 终端样式缓存，避免重复DOM查询
  const styleCache = ref<{
    charWidth: number
    lineHeight: number
    paddingLeft: number
    paddingTop: number
  } | null>(null)

  // === 输出缓冲优化 ===
  let outputBuffer = '' // 输出数据缓冲区，使用字符串而非数组提高性能
  const OUTPUT_FLUSH_INTERVAL = 16 // 16ms刷新间隔，约60fps
  const MAX_BUFFER_LENGTH = 8192 // 最大缓冲区长度，防止内存过度使用

  // === 输出缓冲处理函数 ===

  /**
   * 刷新输出缓冲区到终端
   * 将缓冲区中的所有数据一次性写入终端，减少DOM更新频率
   */
  const flushOutputBuffer = () => {
    if (outputBuffer.length === 0 || !terminal.value) return

    try {
      // 一次性写入终端
      terminal.value.write(outputBuffer)
      outputBuffer = '' // 清空缓冲区
    } catch {
      outputBuffer = '' // 出错时也要清空缓冲区
    }

    // 清除定时器
    if (timers.outputFlush) {
      clearTimeout(timers.outputFlush)
      timers.outputFlush = null
    }
  }

  /**
   * 调度输出缓冲区刷新
   * 使用防抖机制控制刷新频率
   */
  const scheduleOutputFlush = () => {
    // 如果已经有定时器在运行，不需要重新调度
    if (timers.outputFlush) return

    timers.outputFlush = window.setTimeout(() => {
      flushOutputBuffer()
    }, OUTPUT_FLUSH_INTERVAL)
  }

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
      const currentTheme = themeStore.currentThemeData.value
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
      terminal.value.loadAddon(
        new WebLinksAddon((event, uri) => {
          // 支持 Ctrl+点击（Windows/Linux）或 Cmd+点击（Mac）打开链接
          if (event.ctrlKey || event.metaKey) {
            openPath(uri).catch(() => {})
          }
        })
      ) // 链接点击插件
      terminal.value.open(terminalRef.value)

      // 设置核心事件监听
      terminal.value.onResize(({ rows, cols }) => emit('resize', rows, cols)) // 大小变化

      // 合并输入监听：既向外发出输入事件，也维护当前行与光标
      terminal.value.onData(data => {
        emit('input', data)
        if (data === '\r') {
          inputState.currentLine = ''
          inputState.cursorCol = 0
        } else if (data === '\x7f') {
          if (inputState.cursorCol > 0) {
            inputState.currentLine = inputState.currentLine.slice(0, -1)
            inputState.cursorCol--
          }
        } else if (data.length === 1 && data.charCodeAt(0) >= 32) {
          inputState.currentLine += data
          inputState.cursorCol++
        }
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
    () => themeStore.currentThemeData.value,
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
   * 处理补全建议变化
   */
  const handleSuggestionChange = (suggestion: string) => {
    inputState.suggestion = suggestion
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
            // 只在必要时清除缓存
            if (!styleCache.value) {
              styleCache.value = null
            }
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
   * 计算并更新光标在屏幕上的坐标位置
   */
  const updateTerminalCursorPosition = () => {
    try {
      if (!terminal.value || !terminalRef.value) return

      // 获取或计算终端样式信息
      if (!styleCache.value) {
        const computedStyle = window.getComputedStyle(terminalRef.value)
        const testElement = terminalRef.value.querySelector('.xterm-rows')

        if (testElement) {
          const testChar = testElement.querySelector('.xterm-row')?.querySelector('span')
          if (testChar) {
            const charRect = testChar.getBoundingClientRect()
            styleCache.value = {
              charWidth: charRect.width || 9,
              lineHeight: charRect.height || 17,
              paddingLeft: parseInt(computedStyle.paddingLeft) || 0,
              paddingTop: parseInt(computedStyle.paddingTop) || 0,
            }
          }
        }

        // 如果无法获取准确值，使用默认值
        if (!styleCache.value) {
          styleCache.value = {
            charWidth: 9,
            lineHeight: 17,
            paddingLeft: 0,
            paddingTop: 0,
          }
        }
      }

      const cache = styleCache.value
      const buffer = terminal.value.buffer.active
      const terminalRect = terminalRef.value.getBoundingClientRect()

      // 计算光标位置
      const x = terminalRect.left + cache.paddingLeft + buffer.cursorX * cache.charWidth
      const y = terminalRect.top + cache.paddingTop + buffer.cursorY * cache.lineHeight

      terminalEnv.cursorPosition = { x, y }
    } catch {
      // 设置默认位置
      terminalEnv.cursorPosition = { x: 0, y: 0 }
    }
  }

  /**
   * 在系统中打开路径
   */
  const handlePathOpen = async (path: string) => {
    try {
      await openPath(path.trim().replace(/^["']|["']$/g, '')) // 清理引号
      showToast('已打开路径', 'success')
    } catch {
      showToast('无法打开路径', 'error')
    }
  }

  /**
   * 切换到指定路径
   * 发送 cd 命令到终端
   */
  const handleGoToPath = (path: string) => {
    const cleanPath = path.trim().replace(/^["']|["']$/g, '')
    emit('input', `cd "${cleanPath}"\n`)
    showToast(`切换到: ${cleanPath}`, 'success')
  }

  /**
   * 处理文件拖拽到终端
   */
  const handleFileDrop = async (filePath: string) => {
    try {
      // 调用后端命令获取文件所在目录
      const directory = await invoke<string>('handle_file_open', { path: filePath })
      // 切换到该目录
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

  /**
   * 显示提示消息
   */
  const showToast = (message: string, type: 'success' | 'error' = 'success') => {
    toast.visible = true
    toast.message = message
    toast.type = type
    setTimeout(() => {
      toast.visible = false
    }, 3000) // 3秒后自动隐藏
  }

  /**
   * 关闭提示消息
   */
  const closeToast = () => {
    toast.visible = false
  }

  // === Event Handlers for Terminal ===
  const handleOutput = (data: string) => {
    try {
      if (terminal.value && typeof data === 'string') {
        // 将数据添加到缓冲区而不是立即写入
        outputBuffer += data

        // 检测工作目录变化
        detectWorkingDirectoryChange(data)

        // 如果缓冲区过大，立即刷新以防止内存溢出
        if (outputBuffer.length >= MAX_BUFFER_LENGTH) {
          flushOutputBuffer()
        } else {
          // 否则调度延迟刷新
          scheduleOutputFlush()
        }
      }
    } catch {
      // ignore
    }
  }

  /**
   * 检测工作目录变化
   * 使用简化的检测机制，只在特定条件下触发
   */
  const detectWorkingDirectoryChange = (data: string) => {
    // 只在包含路径分隔符且长度合理的数据中检测
    if (!data.includes('/') || data.length > 200) return

    try {
      // 简化的检测：只匹配明显的提示符格式
      const promptMatch = data.match(/([/\w\-.~]+)\s*[$#>]\s*$/)
      if (promptMatch) {
        const newPath = promptMatch[1]
        if (newPath && newPath.startsWith('/') && newPath !== terminalEnv.workingDirectory) {
          terminalEnv.workingDirectory = newPath
          terminalStore.updateTerminalCwd(props.terminalId, newPath)
        }
      }
    } catch {
      // 静默忽略错误
    }
  }

  const handleExit = (exitCode: number | null) => {
    try {
      if (terminal.value) {
        const message = `\r\n[进程已退出，退出码: ${exitCode ?? '未知'}]\r\n`
        terminal.value.write(message)
      }
    } catch {
      // ignore
    }
  }

  // === Lifecycle ===
  onMounted(() => {
    nextTick(async () => {
      // 初始化平台信息
      await initPlatformInfo()

      // 初始化主题系统
      try {
        await themeStore.initialize()
      } catch {
        // ignore
      }

      // 初始化终端（现在是异步的）
      await initXterm()

      // 初始化工作目录 - 优先使用终端状态中保存的工作目录
      const terminal = terminalStore.terminals.find(t => t.id === props.terminalId)
      if (terminal && terminal.cwd) {
        terminalEnv.workingDirectory = terminal.cwd
      } else {
        // 如果没有保存的工作目录，使用系统默认
        windowApi
          .getHomeDirectory()
          .then((dir: any) => {
            terminalEnv.workingDirectory = dir
          })
          .catch(() => {
            terminalEnv.workingDirectory = '/tmp'
          })
      }

      // 注册回调
      terminalStore.registerTerminalCallbacks(props.terminalId, {
        onOutput: handleOutput,
        onExit: handleExit,
      })

      // 注册到终端store的resize回调，避免每个终端都监听window resize
      terminalStore.registerResizeCallback(props.terminalId, resizeTerminal)
    })
  })

  onBeforeUnmount(() => {
    if (hasDisposed) return
    hasDisposed = true

    terminalStore.unregisterTerminalCallbacks(props.terminalId)

    // 清理主题监听器
    themeStore.cleanup()

    // 清理所有定时器和缓冲区
    if (timers.resize) clearTimeout(timers.resize)
    if (timers.themeUpdate) clearTimeout(timers.themeUpdate)
    if (timers.outputFlush) clearTimeout(timers.outputFlush)
    outputBuffer = '' // 清空输出缓冲区

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
