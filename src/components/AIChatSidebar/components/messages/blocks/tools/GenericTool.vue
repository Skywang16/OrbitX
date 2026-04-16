<template>
  <div class="tool-block">
    <!-- Tool status line -->
    <div
      class="tool-line"
      :class="{ clickable: isExpandable, running: isRunning, error: isError, cancelled: isCancelled }"
      @click="toggleExpanded"
    >
      <span class="text" :class="{ running: isRunning }">
        <span v-if="toolPrefix" class="tool-prefix">{{ toolPrefix }}</span>
        <span class="tool-content">{{ displayText }}</span>
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

    <!-- Expandable result area -->
    <transition name="expand">
      <div v-if="isExpanded && hasResult" class="tool-result" :class="{ 'has-scroll': hasScroll }" @click.stop>
        <div ref="resultWrapperRef" class="result-wrapper" @scroll="checkScroll">
          <!-- lsp_query: structured LSP result -->
          <LspResult
            v-if="isLspTool"
            :action="lspAction"
            :metadata="toolMetadata"
            :result-text="typeof cleanResult === 'string' ? cleanResult : JSON.stringify(cleanResult, null, 2)"
          />
          <!-- web_search: favicon + title link list -->
          <div v-else-if="isWebSearchTool && webSearchEntries.length > 0" class="web-results">
            <a
              v-for="(entry, idx) in webSearchEntries"
              :key="idx"
              class="web-entry"
              :href="entry.url"
              target="_blank"
              rel="noopener noreferrer"
            >
              <img
                class="web-favicon"
                :src="getFavicon(entry.url)"
                alt=""
                width="14"
                height="14"
                :onerror="`this.src='${FALLBACK_FAVICON}'`"
              />
              <span class="web-title">{{ entry.title }}</span>
            </a>
          </div>
          <!-- read_file / read_terminal: syntax-highlighted -->
          <pre v-else-if="shouldHighlight" ref="resultTextRef" class="result-text"><code>{{ cleanResult }}</code></pre>
          <!-- everything else: plain text -->
          <pre v-else class="result-text-plain">{{ cleanResult }}</pre>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
  import type { Block } from '@/types'
  import { getPathBasename } from '@/utils/path'
  import hljs from 'highlight.js'
  import stripAnsi from 'strip-ansi'
  import { computed, nextTick, ref, watch } from 'vue'
  import LspResult from '../components/LspResult.vue'

  const props = defineProps<{
    block: Extract<Block, { type: 'tool' }>
    disableExpand?: boolean
  }>()

  // ─── state ───────────────────────────────────────────────────
  const isExpanded = ref(false)
  const resultTextRef = ref<HTMLPreElement | null>(null)
  const resultWrapperRef = ref<HTMLDivElement | null>(null)
  const hasScroll = ref(false)

  // ─── basic block accessors ────────────────────────────────────
  const toolName = computed(() => props.block.name || '')
  const toolParams = computed(() => (props.block.input as Record<string, unknown>) || {})
  const toolResult = computed(() => props.block.output?.content || '')
  const toolMetadata = computed(() => (props.block.output?.metadata as Record<string, unknown>) || null)

  // ─── MCP tool detection ───────────────────────────────────────
  const isMcpTool = computed(() => toolName.value.startsWith('mcp__'))
  const mcpToolInfo = computed(() => {
    if (!isMcpTool.value) return null
    const parts = toolName.value.split('__')
    if (parts.length >= 3) return { server: parts[1], tool: parts.slice(2).join('__') }
    return null
  })

  // ─── statuses ─────────────────────────────────────────────────
  const isRunning = computed(() => props.block.status === 'running' || props.block.status === 'pending')
  const isError = computed(() => props.block.status === 'error')
  const isCancelled = computed(() => props.block.status === 'cancelled')
  const hasResult = computed(
    () => props.block.status !== 'running' && props.block.status !== 'pending' && Boolean(toolResult.value)
  )

  // ─── special tool flags ───────────────────────────────────────
  const isLspTool = computed(() => toolName.value === 'lsp_query')
  const lspAction = computed(() => {
    const a = toolParams.value?.action
    return typeof a === 'string' ? a : ''
  })
  const isWebSearchTool = computed(() => toolName.value === 'web_search')
  const shouldHighlight = computed(() => ['read_file', 'read_terminal', 'read_agent_terminal'].includes(toolName.value))

  // ─── web search entries ───────────────────────────────────────
  interface WebEntry {
    title: string
    url: string
  }
  const FALLBACK_FAVICON = `data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='14' height='14' viewBox='0 0 24 24' fill='none' stroke='%23888' stroke-width='1.5'%3E%3Ccircle cx='12' cy='12' r='10'/%3E%3C/svg%3E`

  const webSearchEntries = computed<WebEntry[]>(() => {
    if (!isWebSearchTool.value) return []
    const meta = toolMetadata.value
    if (!Array.isArray(meta)) return []
    return meta.filter((e): e is WebEntry => typeof e === 'object' && e !== null && 'title' in e && 'url' in e)
  })

  const getFavicon = (url: string) => {
    try {
      return `https://www.google.com/s2/favicons?domain=${new URL(url).hostname}&sz=16`
    } catch {
      return ''
    }
  }

  // ─── expandability ────────────────────────────────────────────
  const isExpandable = computed(() => {
    if (props.disableExpand) return false
    return hasResult.value
  })

  // ─── prefix ───────────────────────────────────────────────────
  const toolPrefix = computed(() => {
    if (isMcpTool.value && mcpToolInfo.value) return `MCP ${mcpToolInfo.value.server} `
    const map: Record<string, string> = {
      read_file: 'Read ',
      read_terminal: 'Read Terminal ',
      read_agent_terminal: 'Read Agent Terminal ',
      orbitx_search: 'Searched ',
      grep: 'Grep ',
      glob: 'Glob ',
      semantic_search: 'Searched ',
      list_files: 'Listed ',
      web_fetch: 'Fetched ',
      web_search: 'Searched ',
      skill: 'Loaded skill ',
      apply_diff: 'Applied diff to ',
      syntax_diagnostics: 'Diagnosed ',
      lsp_query: 'LSP ',
      task: 'Task ',
    }
    return map[toolName.value] ?? ''
  })

  // ─── display text ─────────────────────────────────────────────
  const displayText = computed(() => {
    const p = toolParams.value
    const ext = props.block.output?.metadata as Record<string, unknown> | undefined
    const cancelReason = props.block.output?.cancelReason

    let base = ''

    if (isMcpTool.value && mcpToolInfo.value) {
      base = mcpToolInfo.value.tool
      return isCancelled.value ? `${base} (${cancelReason ?? 'cancelled'})` : base
    }

    switch (toolName.value) {
      case 'read_file': {
        const path = fmt.path(p?.path as string)
        const s = ext?.startLine as number | undefined
        const e = ext?.endLine as number | undefined
        if (s !== undefined && e !== undefined) return `${path} #L${s}-${e}`
        if (s !== undefined) return `${path} #L${s}`
        return path
      }
      case 'read_terminal':
      case 'read_agent_terminal': {
        const ret = ext?.returnedLines as number | undefined
        const tot = ext?.totalLines as number | undefined
        if (ret && tot) return `(${ret}/${tot} lines)`
        const max = p?.maxLines as number | undefined
        return max ? `(max ${max} lines)` : 'output'
      }
      case 'lsp_query': {
        const action = typeof p?.action === 'string' ? p.action : 'query'
        const path = typeof p?.path === 'string' ? fmt.path(p.path) : ''
        const query = typeof p?.query === 'string' ? fmt.text(p.query) : ''
        const line = typeof p?.line === 'number' ? p.line + 1 : null
        const char = typeof p?.character === 'number' ? p.character + 1 : null
        if (path && line !== null && char !== null && ['hover', 'definition', 'references'].includes(action))
          return `${action} ${path} @ ${line}:${char}`
        if (path) return `${action} ${path}`
        if (query) return `${action} ${query}`
        return action
      }
      case 'list_files':
        base = fmt.path(p?.path as string) || 'files'
        break
      case 'web_fetch':
        base = fmt.url(p?.url as string)
        break
      case 'web_search':
      case 'orbitx_search':
      case 'semantic_search':
        base = fmt.text(p?.query as string)
        break
      case 'grep':
      case 'glob':
        base = fmt.text(p?.pattern as string)
        break
      case 'skill':
        base = (p?.name as string) || 'unknown'
        break
      case 'apply_diff':
        base = `${(p?.files as { path: string }[])?.length || 0} files`
        break
      case 'syntax_diagnostics':
        base = `${(p?.paths as string[])?.length || 0} files`
        break
      case 'task':
        base = fmt.text(p?.description as string)
        break
      default:
        base = toolName.value || 'Unknown'
    }

    if (isCancelled.value) return `${base} (${cancelReason ?? 'cancelled'})`

    if (!base && isRunning.value) {
      const streaming = (p as Record<string, unknown>)?.__streaming
      const bytes = (p as Record<string, unknown>)?.__inputBytes
      if (streaming === true && typeof bytes === 'number') return `(${fmtBytes(bytes)} args)`
      if (streaming === true) return '(preparing args)'
      return toolName.value || '...'
    }

    return base
  })

  // ─── format helpers ────────────────────────────────────────────
  const fmt = {
    path: (v: string) => (v ? getPathBasename(v) : ''),
    text: (v: string) => (v ? (v.length > 50 ? v.substring(0, 47) + '...' : v) : ''),
    url: (v: string) => {
      if (!v) return ''
      try {
        const u = new URL(v)
        return u.hostname + (u.pathname !== '/' ? u.pathname : '')
      } catch {
        return v
      }
    },
  }

  const fmtBytes = (b: number) => {
    if (!Number.isFinite(b) || b <= 0) return '0B'
    if (b < 1024) return `${b}B`
    if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)}KB`
    return `${(b / (1024 * 1024)).toFixed(1)}MB`
  }

  // ─── clean result ─────────────────────────────────────────────
  const cleanResult = computed(() => {
    const r = toolResult.value
    if (r && typeof r === 'object' && 'result' in r) {
      const t = (r as { result: unknown }).result
      return typeof t === 'string' ? stripAnsi(t) : t
    }
    if (r && typeof r === 'object' && 'error' in r) {
      const t = (r as { error: unknown }).error
      return typeof t === 'string' ? stripAnsi(t) : t
    }
    return typeof r === 'string' ? stripAnsi(r) : r
  })

  // ─── expand / highlight ───────────────────────────────────────
  const checkScroll = () => {
    if (resultWrapperRef.value)
      hasScroll.value = resultWrapperRef.value.scrollHeight > resultWrapperRef.value.clientHeight
  }

  const highlightCode = () => {
    if (shouldHighlight.value && resultTextRef.value) hljs.highlightElement(resultTextRef.value)
  }

  const toggleExpanded = () => {
    if (!isExpandable.value) return
    isExpanded.value = !isExpanded.value
    if (isExpanded.value)
      nextTick(() => {
        highlightCode()
        checkScroll()
      })
  }

  watch(
    () => [isExpanded.value, cleanResult.value],
    () => {
      if (isExpanded.value)
        nextTick(() => {
          highlightCode()
          checkScroll()
        })
    }
  )
</script>

<style scoped>
  .tool-block {
    margin: 2px 0;
    font-size: 12px;
    line-height: 1.6;
  }

  .tool-line {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--text-400);
    transition: all 0.15s ease;
    font-size: 12px;
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
    font-size: 12px;
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
    font-weight: 500;
  }

  .tool-content {
    color: var(--text-500);
    font-weight: 400;
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

  /* Expandable result */
  .tool-result {
    margin-top: 2px;
    position: relative;
    max-height: 300px;
    overflow: hidden;
  }

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
    background: linear-gradient(to bottom, var(--bg-100) 0%, transparent 100%);
  }

  .tool-result::after {
    bottom: 0;
    background: linear-gradient(to top, var(--bg-100) 0%, transparent 100%);
  }

  .result-wrapper {
    max-height: 300px;
    overflow-y: auto;
    overflow-x: auto;
    padding: 0;
    scrollbar-width: none;
  }

  .result-wrapper::-webkit-scrollbar {
    display: none;
  }

  .result-text {
    margin: 0;
    padding: 0;
    font-family: var(--font-family-mono);
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-400);
    white-space: pre-wrap;
    word-wrap: break-word;
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
    font-family: var(--font-family-mono);
    font-size: 12px;
    line-height: 1.4;
    color: var(--text-400);
    white-space: pre-wrap;
    word-wrap: break-word;
    background: transparent;
  }

  /* Web search results */
  .web-results {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 2px 0;
  }

  .web-entry {
    display: flex;
    align-items: center;
    gap: 6px;
    text-decoration: none;
    border-radius: 3px;
    padding: 2px 3px;
    min-width: 0;
  }

  .web-entry:hover {
    background: color-mix(in srgb, var(--bg-200) 60%, transparent);
  }

  .web-entry:hover .web-title {
    color: var(--text-200);
  }

  .web-favicon {
    flex-shrink: 0;
    width: 14px;
    height: 14px;
    border-radius: 2px;
    opacity: 0.85;
  }

  .web-title {
    font-size: 11px;
    color: var(--text-400);
    line-height: 1.4;
    transition: color 0.1s ease;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  /* Transitions */
  .expand-enter-active,
  .expand-leave-active {
    transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
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
    margin-top: 2px;
  }
</style>
