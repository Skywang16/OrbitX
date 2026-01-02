<template>
  <div class="tool-block">
    <div class="tool-line" :class="{ clickable: isExpandable, running: isRunning, error: isError }">
      <span class="text" :class="{ clickable: isExpandable }" @click="toggleExpanded">
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
        @click="toggleExpanded"
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

    <transition name="expand">
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
  import stripAnsi from 'strip-ansi'
  import hljs from 'highlight.js'

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

  // Extract tool information from step metadata
  const toolName = computed(() => {
    return props.block.name || ''
  })

  const toolParams = computed(() => {
    return (props.block.input as Record<string, unknown>) || {}
  })

  const toolResult = computed(() => {
    return props.block.output?.content || ''
  })

  const isError = computed(() => {
    return props.block.status === 'error' || Boolean(props.block.output?.isError)
  })

  const hasResult = computed(() => {
    return props.block.status !== 'running' && Boolean(toolResult.value)
  })

  const isEditResult = computed(() => {
    return toolName.value === 'edit_file'
  })

  const shouldHighlight = computed(() => {
    return toolName.value === 'read_file' || toolName.value === 'read_terminal'
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
      (props.block.output?.ext as EditResultData) ||
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
    return props.block.status === 'running'
  })

  const diffStats = computed(() => {
    if (toolName.value !== 'edit_file' || !props.block.output?.ext) return null
    const extInfo = props.block.output.ext as EditResultData
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
    switch (toolName.value) {
      case 'read_file':
        return 'Read '
      case 'read_terminal':
        return 'Read Terminal '
      case 'orbit_search':
        return 'Searched '
      case 'shell':
        return 'Shell '
      case 'edit_file':
        return 'Edited '
      case 'write_to_file':
        return 'Wrote to '
      case 'insert_content':
        return 'Inserted to '
      case 'list_files':
        return 'Listed '
      case 'web_fetch':
        return 'Fetched '
      case 'apply_diff':
        return 'Applied diff to '
      default:
        return ''
    }
  })

  const getDisplayText = () => {
    const params = toolParams.value
    const extInfo = props.block.output?.ext as Record<string, unknown> | undefined

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
        return formatPath(params?.path as string)
      case 'write_to_file':
        return formatPath(params?.path as string)
      case 'insert_content':
        return formatPath(params?.path as string)
      case 'shell':
        return formatText(params?.command as string)
      case 'orbit_search':
        return formatText(params?.query as string)
      case 'list_files':
        return formatPath(params?.path as string) || 'files'
      case 'web_fetch':
        return formatUrl(params?.url as string)
      case 'apply_diff':
        return `${(params?.files as { path: string }[])?.length || 0} files`
      default:
        return toolName.value || 'Unknown'
    }
  }

  const formatPath = (path: string) => {
    if (!path) return ''
    const parts = path.split('/')
    // 只返回文件名
    return parts[parts.length - 1] || path
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

  .tool-line.running .text {
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

  .text.clickable {
    cursor: pointer;
  }

  .text.clickable:hover {
    color: var(--text-300);
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
    cursor: pointer;
  }

  .chevron:hover,
  .text.clickable:hover ~ .chevron {
    opacity: 1;
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
    background: linear-gradient(to bottom, var(--bg-200) 0%, transparent 100%);
  }

  .tool-result::after {
    bottom: 0;
    background: linear-gradient(to top, var(--bg-200) 0%, transparent 100%);
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
    background: var(--bg-200);
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
</style>
