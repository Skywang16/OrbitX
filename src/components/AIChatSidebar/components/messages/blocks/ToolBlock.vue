<template>
  <!-- todowrite 特殊渲染：简单的进度列表 -->
  <div v-if="isTodoWrite" class="todo-block" :class="{ running: isRunning }">
    <div class="todo-header">
      <span class="todo-label">Todo</span>
      <span v-if="isRunning" class="todo-progress todo-progress-running">{{ todoProgress || '处理中…' }}</span>
      <span v-else class="todo-progress">{{ todoProgress }}</span>
    </div>
    <div v-if="todoItems.length > 0" class="todo-list">
      <div v-for="(item, idx) in todoItems" :key="idx" class="todo-item" :class="item.status">
        <span class="todo-icon">
          {{ item.status === 'completed' ? '✓' : item.status === 'in_progress' ? '▶' : '○' }}
        </span>
        <span class="todo-text">{{ item.content }}</span>
      </div>
    </div>
    <div v-else-if="isRunning" class="todo-empty">等待任务输出…</div>
  </div>

  <!-- Shell 工具：独立渲染 -->
  <div v-else-if="isShellTool" class="tool-block-shell">
    <div class="shell-header">
      <div class="shell-info">
        <svg
          class="shell-icon"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <polyline points="4 17 10 11 4 5"></polyline>
          <line x1="12" y1="19" x2="20" y2="19"></line>
        </svg>
        <span class="shell-command" :class="{ running: isRunning }" :title="shellCommandDisplay">
          {{ shellCommandDisplay }}
        </span>
      </div>
      <button v-if="shellPaneId !== null" class="shell-expand-btn" @click="openShellTerminal" title="Open in main tab">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="15 3 21 3 21 9"></polyline>
          <polyline points="9 21 3 21 3 15"></polyline>
          <line x1="21" y1="3" x2="14" y2="10"></line>
          <line x1="3" y1="21" x2="10" y2="14"></line>
        </svg>
      </button>
    </div>
    <!-- 有终端或正在运行时显示终端容器 -->
    <div v-if="shellPaneId !== null || isRunning" class="shell-terminal">
      <Terminal v-if="shellPaneId !== null" :terminal-id="shellPaneId" :is-active="false" />
    </div>
    <!-- 其他状态（错误/取消/完成但无终端）显示文本 -->
    <div v-else class="shell-content">
      <pre class="shell-output">{{ toolResult || 'No output' }}</pre>
    </div>
  </div>

  <!-- 其他工具的通用渲染 -->
  <div v-else class="tool-block">
    <!-- 工具状态行：所有工具统一使用 -->
    <div
      class="tool-line"
      :class="{ clickable: isExpandable, running: isRunning, error: isError, cancelled: isCancelled }"
      @click="toggleExpanded"
    >
      <span class="text" :class="{ running: isRunning }">
        <span v-if="toolPrefix" class="tool-prefix">{{ toolPrefix }}</span>
        <span class="tool-content">{{ getDisplayText() }}</span>
      </span>
      <span v-if="diffStats" class="diff-stats">
        <span class="add">+{{ diffStats.added }}</span>
        <span class="del">-{{ diffStats.removed }}</span>
      </span>
      <svg
        v-if="isExpandable"
        class="chevron"
        :class="{ expanded: isExpanded }"
        width="10"
        height="10"
        viewBox="0 0 10 10"
      >
        <path
          d="M3.5 2.5L6 5L3.5 7.5"
          stroke="currentColor"
          stroke-width="1"
          stroke-linecap="round"
          stroke-linejoin="round"
          fill="none"
        />
      </svg>
    </div>

    <!-- Shell 工具的内容区 -->
    <template v-if="isShellTool">
      <!-- 有终端时显示 -->
      <div v-if="shellPaneId !== null" class="shell-terminal">
        <Terminal :terminal-id="shellPaneId" :is-active="false" />
      </div>
      <!-- 正在运行（且没有终端）时显示 loading -->
      <div v-else-if="isRunning" class="shell-loading">
        <div class="shell-loading-spinner"></div>
      </div>
    </template>

    <!-- 其他工具：可展开的结果区 -->
    <transition v-else name="expand">
      <div v-if="isExpanded && hasResult" class="tool-result" :class="{ 'has-scroll': hasScroll }" @click.stop>
        <div ref="resultWrapperRef" class="result-wrapper" @scroll="checkScroll">
          <EditResult v-if="isEditResult" :editData="editData" />
          <pre
            v-else-if="shouldHighlight"
            ref="resultTextRef"
            class="result-text"
          ><code>{{ cleanToolResult }}</code></pre>
          <pre v-else class="result-text-plain">{{ cleanToolResult }}</pre>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
  import { ref, computed, nextTick, watch } from 'vue'
  import type { Block } from '@/types'
  import EditResult from './components/EditResult.vue'
  import Terminal from '@/components/terminal/Terminal.vue'
  import { useEditorStore } from '@/stores/Editor'
  import stripAnsi from 'strip-ansi'
  import hljs from 'highlight.js'
  import { getPathBasename } from '@/utils/path'

  interface EditResultData {
    file: string
    replacedCount: number
    affectedLines?: number[]
    useRegex: boolean
    ignoreCase: boolean
    startLine: number | null
    endLine: number | null
    previewOnly: boolean
    old: string
    new: string
  }

  const props = defineProps<{
    block: Extract<Block, { type: 'tool' }>
  }>()

  const isExpanded = ref(false)
  const resultTextRef = ref<HTMLPreElement | null>(null)
  const resultWrapperRef = ref<HTMLDivElement | null>(null)
  const hasScroll = ref(false)
  const editorStore = useEditorStore()

  // Extract tool information from step metadata
  const toolName = computed(() => {
    return props.block.name || ''
  })

  // MCP 工具检测和解析
  const isMcpTool = computed(() => toolName.value.startsWith('mcp__'))

  const mcpToolInfo = computed(() => {
    if (!isMcpTool.value) return null
    // 格式: mcp__{server}__{tool}
    const parts = toolName.value.split('__')
    if (parts.length >= 3) {
      return {
        server: parts[1],
        tool: parts.slice(2).join('__'), // 处理工具名中可能有 __ 的情况
      }
    }
    return null
  })

  // todowrite 特殊处理
  const isTodoWrite = computed(() => toolName.value === 'todowrite')

  interface TodoItem {
    content: string
    status: 'pending' | 'in_progress' | 'completed'
  }

  const todoItems = computed<TodoItem[]>(() => {
    if (!isTodoWrite.value) return []
    const input = props.block.input as { todos?: TodoItem[] } | undefined
    return input?.todos || []
  })

  const todoProgress = computed(() => {
    if (todoItems.value.length === 0) return ''
    const done = todoItems.value.filter(t => t.status === 'completed').length
    return `${done}/${todoItems.value.length}`
  })

  const toolParams = computed(() => {
    return (props.block.input as Record<string, unknown>) || {}
  })

  const toolResult = computed(() => {
    return props.block.output?.content || ''
  })

  const toolMetadata = computed(() => (props.block.output?.metadata as Record<string, unknown>) || null)

  // Shell 工具相关
  const isShellTool = computed(() => toolName.value === 'shell')
  const shellPaneId = computed(() => {
    if (!isShellTool.value) return null
    const paneId = toolMetadata.value?.paneId
    return typeof paneId === 'number' ? paneId : null
  })
  const shellTerminalId = computed(() => {
    if (!isShellTool.value) return null
    const terminalId = toolMetadata.value?.terminalId
    return typeof terminalId === 'string' ? terminalId : null
  })
  // 加载时从 input 取命令，完成后从 metadata 取
  const shellCommandDisplay = computed(() => {
    if (!isShellTool.value) return ''
    // 优先从 metadata 取（执行完成后）
    const metaCommand = toolMetadata.value?.command
    if (typeof metaCommand === 'string' && metaCommand) return metaCommand
    // 否则从 input 取（正在加载时）
    const inputCommand = toolParams.value?.command
    return typeof inputCommand === 'string' ? inputCommand : ''
  })

  const openShellTerminal = async () => {
    if (!shellTerminalId.value || shellPaneId.value == null) return
    await editorStore.openAgentTerminalTab({
      terminalId: shellTerminalId.value,
      paneId: shellPaneId.value,
      command: shellCommandDisplay.value || 'shell',
      activate: true,
    })
  }

  const isError = computed(() => {
    return props.block.status === 'error'
  })

  const hasResult = computed(() => {
    return props.block.status !== 'running' && props.block.status !== 'pending' && Boolean(toolResult.value)
  })

  const isEditResult = computed(() => {
    return toolName.value === 'edit_file'
  })

  const shouldHighlight = computed(() => {
    return (
      toolName.value === 'read_file' || toolName.value === 'read_terminal' || toolName.value === 'read_agent_terminal'
    )
  })

  const editData = computed(() => {
    if (!isEditResult.value) {
      return {
        file: '',
        replacedCount: 0,
        useRegex: false,
        ignoreCase: false,
        startLine: null,
        endLine: null,
        previewOnly: false,
        old: '',
        new: '',
      } as EditResultData
    }
    return (
      (props.block.output?.metadata as EditResultData) ||
      ({
        file: '',
        replacedCount: 0,
        useRegex: false,
        ignoreCase: false,
        startLine: null,
        endLine: null,
        previewOnly: false,
        old: '',
        new: '',
      } as EditResultData)
    )
  })

  const isExpandable = computed(() => {
    return toolName.value === 'edit_file' || hasResult.value
  })

  const isRunning = computed(() => {
    return props.block.status === 'running' || props.block.status === 'pending'
  })

  const isCancelled = computed(() => {
    return props.block.status === 'cancelled'
  })

  const diffStats = computed(() => {
    if (toolName.value !== 'edit_file' || !props.block.output?.metadata) return null
    const extInfo = props.block.output.metadata as EditResultData
    if (extInfo.old && extInfo.new) {
      const oldLines = extInfo.old.split('\n').length
      const newLines = extInfo.new.split('\n').length
      return {
        added: Math.max(0, newLines),
        removed: Math.max(0, oldLines),
      }
    }
    return null
  })

  const toolPrefix = computed(() => {
    // MCP 工具：显示 "MCP server_name "
    if (isMcpTool.value && mcpToolInfo.value) {
      return `MCP ${mcpToolInfo.value.server} `
    }
    switch (toolName.value) {
      case 'read_file':
        return 'Read '
      case 'read_terminal':
        return 'Read Terminal '
      case 'read_agent_terminal':
        return 'Read Agent Terminal '
      case 'orbit_search':
        return 'Searched '
      case 'shell':
        return 'Shell '
      case 'edit_file':
        return 'Edited '
      case 'syntax_diagnostics':
        return 'Diagnosed '
      case 'write_file':
      case 'write_to_file':
        return 'Wrote to '
      case 'insert_content':
        return 'Inserted to '
      case 'list_files':
        return 'Listed '
      case 'web_fetch':
        return 'Fetched '
      case 'web_search':
        return 'Searched '
      case 'skill':
        return 'Loaded skill '
      case 'apply_diff':
        return 'Applied diff to '
      default:
        return ''
    }
  })

  const getDisplayText = () => {
    const params = toolParams.value
    const extInfo = props.block.output?.metadata as Record<string, unknown> | undefined
    const cancelReason = props.block.output?.cancelReason

    let baseText = ''

    // MCP 工具：显示工具名
    if (isMcpTool.value && mcpToolInfo.value) {
      baseText = mcpToolInfo.value.tool
      if (isCancelled.value) {
        return cancelReason ? `${baseText} (${cancelReason})` : `${baseText} (cancelled)`
      }
      return baseText
    }

    switch (toolName.value) {
      case 'read_file': {
        const path = formatPath(params?.path as string)
        // 从 extInfo 读取行号(tool_result 才有)
        const startLine = extInfo?.startLine as number | undefined
        const endLine = extInfo?.endLine as number | undefined
        if (startLine !== undefined && endLine !== undefined) {
          return `${path} #L${startLine}-${endLine}`
        } else if (startLine !== undefined) {
          return `${path} #L${startLine}`
        }
        return path
      }
      case 'read_terminal': {
        const maxLines = params?.maxLines as number | undefined
        const returnedLines = extInfo?.returnedLines as number | undefined
        const totalLines = extInfo?.totalLines as number | undefined
        if (returnedLines && totalLines) {
          return `(${returnedLines}/${totalLines} lines)`
        } else if (maxLines) {
          return `(max ${maxLines} lines)`
        }
        return 'output'
      }
      case 'edit_file':
        baseText = formatPath(params?.path as string)
        break
      case 'write_file':
      case 'write_to_file':
        baseText = formatPath(params?.path as string)
        break
      case 'insert_content':
        baseText = formatPath(params?.path as string)
        break
      case 'shell':
        baseText = formatText(params?.command as string)
        break
      case 'orbit_search':
        baseText = formatText(params?.query as string)
        break
      case 'list_files':
        baseText = formatPath(params?.path as string) || 'files'
        break
      case 'web_fetch':
        baseText = formatUrl(params?.url as string)
        break
      case 'web_search':
        baseText = formatText(params?.query as string)
        break
      case 'skill':
        baseText = (params?.name as string) || 'unknown'
        break
      case 'apply_diff':
        baseText = `${(params?.files as { path: string }[])?.length || 0} files`
        break
      case 'syntax_diagnostics':
        baseText = `${(params?.paths as string[])?.length || 0} files`
        break
      default:
        baseText = toolName.value || 'Unknown'
    }

    if (isCancelled.value) {
      return cancelReason ? `${baseText} (${cancelReason})` : `${baseText} (cancelled)`
    }

    if (!baseText && isRunning.value) {
      const streaming = (params as Record<string, unknown>)?.__streaming
      const bytes = (params as Record<string, unknown>)?.__inputBytes
      if (streaming === true && typeof bytes === 'number') {
        return `(${formatBytes(bytes)} args)`
      }
      if (streaming === true) {
        return '(preparing args)'
      }
      return toolName.value || '...'
    }

    return baseText
  }

  const formatPath = (path: string) => {
    if (!path) return ''
    return getPathBasename(path)
  }

  const formatUrl = (url: string) => {
    if (!url) return ''
    try {
      const urlObj = new URL(url)
      return urlObj.hostname + (urlObj.pathname !== '/' ? urlObj.pathname : '')
    } catch {
      return url
    }
  }

  const formatText = (text: string) => {
    if (!text) return ''
    return text.length > 50 ? text.substring(0, 47) + '...' : text
  }

  const formatBytes = (bytes: number) => {
    if (!Number.isFinite(bytes) || bytes <= 0) return '0B'
    if (bytes < 1024) return `${bytes}B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`
    return `${(bytes / (1024 * 1024)).toFixed(1)}MB`
  }

  const cleanToolResult = computed(() => {
    const result = toolResult.value

    if (result && typeof result === 'object' && 'result' in result) {
      const text = result?.result
      return typeof text === 'string' ? stripAnsi(text) : text
    }
    if (result && typeof result === 'object' && 'error' in result) {
      const text = result.error
      return typeof text === 'string' ? stripAnsi(text) : text
    }
    if (typeof result === 'string') {
      return stripAnsi(result)
    }
    return result
  })

  const checkScroll = () => {
    if (resultWrapperRef.value) {
      hasScroll.value = resultWrapperRef.value.scrollHeight > resultWrapperRef.value.clientHeight
    }
  }

  const toggleExpanded = () => {
    if (isExpandable.value) {
      isExpanded.value = !isExpanded.value
      if (isExpanded.value) {
        nextTick(() => {
          highlightCode()
          checkScroll()
        })
      }
    }
  }

  const highlightCode = () => {
    if (shouldHighlight.value && resultTextRef.value) {
      hljs.highlightElement(resultTextRef.value)
    }
  }

  // 监听结果变化,自动高亮
  watch(
    () => [isExpanded.value, cleanToolResult.value],
    () => {
      if (isExpanded.value) {
        nextTick(() => {
          highlightCode()
          checkScroll()
        })
      }
    }
  )
</script>

<style scoped>
  /* Todo block 样式 */
  .todo-block {
    margin: 6px 0;
    font-size: 13px;
  }

  .todo-header {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-400);
    margin-bottom: 4px;
  }

  .todo-label {
    font-weight: 500;
  }

  .todo-progress {
    color: var(--text-500);
    font-size: 12px;
  }

  .todo-block.running .todo-label {
    color: var(--text-300);
  }

  .todo-progress-running {
    background: linear-gradient(
      90deg,
      var(--text-500) 0%,
      var(--text-500) 25%,
      var(--text-200) 50%,
      var(--text-500) 75%,
      var(--text-500) 100%
    );
    background-size: 300% 100%;
    background-clip: text;
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    animation: scan 2s linear infinite;
  }

  .todo-empty {
    font-size: 12px;
    color: var(--text-500);
    padding-left: 18px;
  }

  .todo-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .todo-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 0;
    color: var(--text-400);
  }

  .todo-item.completed {
    color: var(--text-500);
  }

  .todo-item.in_progress {
    color: var(--text-300);
  }

  .todo-icon {
    width: 14px;
    text-align: center;
    flex-shrink: 0;
  }

  .todo-item.completed .todo-icon {
    color: #10b981;
  }

  .todo-item.in_progress .todo-icon {
    color: #3b82f6;
  }

  .todo-text {
    flex: 1;
    min-width: 0;
  }

  /* Tool block 样式 */
  .tool-block {
    margin: 6px 0;
    font-size: 14px;
    line-height: 1.8;
  }

  .tool-line {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 0;
    color: var(--text-400);
    transition: all 0.15s ease;
    font-size: 14px;
  }

  .tool-line.clickable {
    cursor: pointer;
  }

  .tool-line.clickable:hover {
    color: var(--text-300);
  }

  .tool-line.clickable:hover .chevron {
    opacity: 1;
  }

  .tool-line.running .text,
  .text.running {
    background: linear-gradient(
      90deg,
      var(--text-500) 0%,
      var(--text-500) 25%,
      var(--text-200) 50%,
      var(--text-500) 75%,
      var(--text-500) 100%
    );
    background-size: 300% 100%;
    background-clip: text;
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    animation: scan 2s linear infinite;
  }

  @keyframes scan {
    0% {
      background-position: 100% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }

  .tool-line.error {
    color: var(--color-error);
  }

  .tool-line.cancelled {
    color: var(--text-500);
    opacity: 0.85;
  }

  .text {
    font-size: 14px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .tool-prefix {
    color: var(--text-400);
    font-weight: 400;
  }

  .tool-content {
    color: var(--text-500);
    font-weight: 400;
  }

  .diff-stats {
    display: flex;
    gap: 6px;
    font-size: 12px;
    font-weight: 500;
    flex-shrink: 0;
  }

  .diff-stats .add {
    color: #10b981;
  }

  .diff-stats .del {
    color: #ef4444;
  }

  .chevron {
    flex-shrink: 0;
    color: var(--text-500);
    transition: transform 0.2s ease;
    opacity: 0.5;
  }

  .chevron.expanded {
    transform: rotate(90deg);
  }

  .tool-result {
    margin-top: 8px;
    margin-left: 0;
    position: relative;
    max-height: 300px;
    overflow: hidden;
  }

  .tool-terminal-preview {
    margin-top: 8px;
    border: 1px solid var(--border-200);
    border-radius: 8px;
    overflow: hidden;
    background: var(--bg-200);
  }

  .tool-terminal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 8px;
    font-size: 12px;
    color: var(--text-400);
    background: var(--bg-300);
    border-bottom: 1px solid var(--border-200);
  }

  .tool-terminal-title {
    font-weight: 500;
  }

  .tool-terminal-open {
    border: none;
    background: none;
    color: var(--text-300);
    cursor: pointer;
    font-size: 12px;
    padding: 2px 6px;
    border-radius: 6px;
  }

  .tool-terminal-open:hover {
    background: var(--bg-500);
    color: var(--text-200);
  }

  .tool-terminal-preview :deep(.terminal-container) {
    height: 160px;
  }

  /* 只在有滚动条时显示渐变阴影 */
  .tool-result::before,
  .tool-result::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    height: 20px;
    pointer-events: none;
    z-index: 2;
    opacity: 0;
    transition: opacity 0.2s;
  }

  .tool-result.has-scroll::before,
  .tool-result.has-scroll::after {
    opacity: 1;
  }

  .tool-result::before {
    top: 0;
    background: linear-gradient(to bottom, var(--bg-50) 0%, transparent 100%);
  }

  .tool-result::after {
    bottom: 0;
    background: linear-gradient(to top, var(--bg-50) 0%, transparent 100%);
  }

  .result-wrapper {
    max-height: 300px;
    overflow-y: auto;
    overflow-x: auto;
    padding: 0;
    scrollbar-gutter: stable;
  }

  /* 和 MessageList 完全一致的滚动条样式 */
  .result-wrapper::-webkit-scrollbar {
    width: 8px;
  }

  .result-wrapper::-webkit-scrollbar-track {
    background: var(--bg-100);
    border-radius: 4px;
  }

  .result-wrapper::-webkit-scrollbar-thumb {
    background: var(--border-300);
    border-radius: 4px;
    transition: background-color 0.2s ease;
  }

  .result-wrapper::-webkit-scrollbar-thumb:hover {
    background: var(--border-400);
  }

  .result-text {
    margin: 0;
    padding: 0;
    font-family: 'SF Mono', 'Monaco', 'Menlo', 'Consolas', monospace;
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-400);
    white-space: pre-wrap;
    word-wrap: break-word;
    overflow-wrap: break-word;
    background: transparent;
  }

  .result-text code {
    font-family: inherit;
    font-size: inherit;
    line-height: inherit;
    background: transparent;
    padding: 0;
    margin: 0;
    display: block;
  }

  .result-text-plain {
    margin: 0;
    padding: 0;
    font-family: 'SF Mono', 'Monaco', 'Menlo', 'Consolas', monospace;
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-400);
    white-space: pre-wrap;
    word-wrap: break-word;
    overflow-wrap: break-word;
    background: transparent;
  }

  .tool-result::before,
  .expand-enter-active,
  .expand-leave-active {
    transition: all 0.2s ease;
    overflow: hidden;
  }

  .expand-enter-from,
  .expand-leave-to {
    max-height: 0;
    opacity: 0;
    margin-top: 0;
  }

  .expand-enter-to,
  .expand-leave-from {
    max-height: 300px;
    opacity: 1;
    margin-top: 8px;
  }

  /* Shell 工具样式 */
  .tool-block-shell {
    background: var(--bg-100);
    border: 1px solid var(--border-200);
    border-radius: 8px;
    overflow: hidden;
    margin: 6px 0;
  }

  .shell-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: linear-gradient(135deg, rgba(102, 126, 234, 0.08) 0%, rgba(118, 75, 162, 0.08) 100%);
    border-bottom: 1px solid var(--border-200);
  }

  .shell-info {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
  }

  .shell-icon {
    color: #667eea;
    flex-shrink: 0;
  }

  .shell-command {
    font-family: 'SF Mono', 'Monaco', 'Menlo', 'Consolas', monospace;
    font-size: 13px;
    color: var(--text-300);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .shell-command.running {
    background: linear-gradient(
      90deg,
      var(--text-500) 0%,
      var(--text-500) 25%,
      var(--text-200) 50%,
      var(--text-500) 75%,
      var(--text-500) 100%
    );
    background-size: 300% 100%;
    background-clip: text;
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    animation: scan 2s linear infinite;
  }

  .shell-expand-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    background: transparent;
    border: 1px solid var(--border-200);
    border-radius: 4px;
    color: var(--text-400);
    cursor: pointer;
    transition: all 0.2s;
    flex-shrink: 0;
  }

  .shell-expand-btn:hover {
    background: var(--bg-300);
    color: var(--text-300);
    border-color: #667eea;
  }

  .shell-terminal {
    height: 180px;
    overflow: hidden;
  }

  .shell-terminal :deep(.terminal-wrapper) {
    height: 100%;
    background: transparent;
    padding: 8px;
  }

  .shell-terminal :deep(.terminal-container) {
    background: transparent;
  }

  .shell-content {
    min-height: 180px;
    background: var(--bg-100);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .shell-output {
    margin: 0;
    padding: 12px;
    font-family: 'SF Mono', 'Monaco', 'Menlo', 'Consolas', monospace;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-400);
    white-space: pre-wrap;
    word-break: break-word;
    width: 100%;
  }
</style>
