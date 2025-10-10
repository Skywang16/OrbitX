<template>
  <div class="tool-block" v-if="step?.stepType === 'tool_use' || step?.stepType === 'tool_result'">
    <div
      class="tool-header"
      :class="{ expandable: isExpandable, 'non-expandable': !isExpandable }"
      @click="toggleExpanded"
    >
      <div class="tool-info">
        <div class="tool-icon" v-html="getToolIcon(toolName)"></div>
        <span class="tool-name">{{ toolName || 'Unknown Tool' }}</span>
        <span class="tool-param" v-if="toolParam">
          {{ toolParam }}
        </span>
      </div>
      <div
        class="status-dot"
        :class="{
          running: step.stepType === 'tool_use',
          completed: step.stepType === 'tool_result' && !isError,
          error: step.stepType === 'tool_result' && isError,
        }"
      ></div>
    </div>

    <div v-if="isExpanded && hasResult" class="tool-result" @click.stop>
      <EditResult v-if="isEditResult" :editData="editData" />
      <div v-else class="tool-result-content">{{ cleanToolResult }}</div>
    </div>
  </div>

  <div v-else class="tool-block error">
    <div class="tool-header non-expandable">
      <div class="tool-info">
        <div class="tool-icon" v-html="getToolIcon('unknown')"></div>
        <span class="tool-name">{{ t('tool.data_error') }}</span>
      </div>
      <div class="status-dot error"></div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import type { UiStep } from '@/api/agent/types'
  import EditResult from './components/EditResult.vue'
  import stripAnsi from 'strip-ansi'

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

  const { t } = useI18n()

  const props = defineProps<{
    step: UiStep
  }>()

  const isExpanded = ref(false)

  // Extract tool information from step metadata
  const toolName = computed(() => {
    return (props.step.metadata?.toolName as string) || ''
  })

  const toolParams = computed(() => {
    return (props.step.metadata?.params as Record<string, unknown>) || {}
  })

  const toolResult = computed(() => {
    return props.step.metadata?.result || ''
  })

  const isError = computed(() => {
    return Boolean(props.step.metadata?.isError)
  })

  const hasResult = computed(() => {
    return props.step.stepType === 'tool_result' && (toolResult.value || props.step.content)
  })

  const isEditResult = computed(() => {
    return toolName.value === 'edit_file'
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
      (props.step.metadata?.extInfo as EditResultData) ||
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

  const toolParam = computed(() => {
    return getToolParam({ name: toolName.value, params: toolParams.value })
  })

  const toolIcons = {
    read_file: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path d="M14 2H6C4.9 2 4 2.9 4 4V20C4 21.1 4.89 22 5.99 22H18C19.1 22 20 21.1 20 20V8L14 2Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M14 2V8H20" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M16 13H8" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M16 17H8" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M10 9H8" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>`,
    read_many_files: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path d="M15 2H6C4.9 2 4 2.9 4 4V20C4 21.1 4.89 22 5.99 22H18C19.1 22 20 21.1 20 20V7L15 2Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M15 2V7H20" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M9 15H15" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M9 11H15" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M2 6H4V18H2" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>`,

    edit_file: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path d="M11 4H4C3.46957 4 2.96086 4.21071 2.58579 4.58579C2.21071 4.96086 2 5.46957 2 6V20C2 20.5304 2.21071 21.0391 2.58579 21.4142C2.96086 21.7893 3.46957 22 4 22H18C18.5304 22 19.0391 21.7893 19.4142 21.4142C19.7893 21.0391 20 20.5304 20 20V13" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M18.5 2.50023C18.8978 2.1024 19.4374 1.87891 20 1.87891C20.5626 1.87891 21.1022 2.1024 21.5 2.50023C21.8978 2.89805 22.1213 3.43762 22.1213 4.00023C22.1213 4.56284 21.8978 5.1024 21.5 5.50023L12 15.0002L8 16.0002L9 12.0002L18.5 2.50023Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>`,
    shell: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <rect x="2" y="3" width="20" height="14" rx="2" ry="2" stroke="currentColor" stroke-width="2"/>
      <path d="M8 21L16 21" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
      <path d="M12 17L12 21" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
      <path d="M6 7L8 9L6 11" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M10 11H14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
    </svg>`,
    web_fetch: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2"/>
      <path d="M2 12H22" stroke="currentColor" stroke-width="2"/>
      <path d="M12 2C14.5013 4.73835 15.9228 8.29203 16 12C15.9228 15.708 14.5013 19.2616 12 22C9.49872 19.2616 8.07725 15.708 8 12C8.07725 8.29203 9.49872 4.73835 12 2Z" stroke="currentColor" stroke-width="2"/>
    </svg>`,
    orbit_search: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <circle cx="11" cy="11" r="8" stroke="currentColor" stroke-width="2"/>
      <path d="M21 21L16.65 16.65" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <circle cx="11" cy="11" r="3" stroke="currentColor" stroke-width="2"/>
      <path d="M8 11H14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
      <path d="M11 8V14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
    </svg>`,
    apply_diff: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path d="M14 2H6C4.9 2 4 2.9 4 4V20C4 21.1 4.89 22 5.99 22H18C19.1 22 20 21.1 20 20V8L14 2Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M14 2V8H20" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M8 13H16" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M8 17H12" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M16 17L18 15L16 13" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>`,
    list_files: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path d="M3 7V5C3 3.89543 3.89543 3 5 3H9L11 5H19C20.1046 5 21 5.89543 21 7V7" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M3 7H21V17C21 18.1046 20.1046 19 19 19H5C3.89543 19 3 18.1046 3 17V7Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M7 11H17" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
      <path d="M7 15H13" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
    </svg>`,
    list_code_definition_names: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path d="M16 18L22 12L16 6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M8 6L2 12L8 18" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M12 4L10 20" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>`,
    insert_content: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path d="M14 2H6C4.9 2 4 2.9 4 4V20C4 21.1 4.89 22 5.99 22H18C19.1 22 20 21.1 20 20V8L14 2Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M14 2V8H20" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M9 15H15" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M12 12V18" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>`,
    write_to_file: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path d="M14 2H6C4.9 2 4 2.9 4 4V20C4 21.1 4.89 22 5.99 22H18C19.1 22 20 21.1 20 20V8L14 2Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M14 2V8H20" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M10 12L12 14L16 10" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>`,

    unknown: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2"/>
      <path d="M9.09 9C9.3251 8.33167 9.78915 7.76811 10.4 7.40913C11.0108 7.05016 11.7289 6.91894 12.4272 7.03871C13.1255 7.15849 13.7588 7.52152 14.2151 8.06353C14.6713 8.60553 14.9211 9.29152 14.92 10C14.92 12 11.92 13 11.92 13" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M12 17H12.01" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>`,
  }

  const getToolIcon = (toolName: string) => {
    return toolIcons[toolName as keyof typeof toolIcons] || toolIcons.unknown
  }

  type AnyToolExecution = { name: string; params?: Record<string, unknown> }
  const getToolParam = (toolExecution: AnyToolExecution) => {
    const { name, params } = toolExecution

    switch (name) {
      case 'edit_file':
      case 'read_file':
      case 'write_to_file':
      case 'insert_content':
        return formatPath((params?.path as string) || '')

      case 'read_many_files':
        return `${(params?.paths as string[])?.length || 0} files`

      case 'web_fetch':
        return formatUrl((params?.url as string) || '')

      case 'orbit_search':
        return formatText((params?.query as string) || '')

      case 'shell':
        return formatText((params?.command as string) || '')

      case 'list_files':
        return formatPath((params?.path as string) || '')

      case 'list_code_definition_names':
        return formatPath((params?.path as string) || '')

      case 'apply_diff':
        return `${(params?.files as { path: string; hunks: unknown[] }[])?.length || 0} files`

      default:
        return ''
    }
  }

  const formatPath = (path: string) => {
    if (!path) return ''
    const parts = path.split('/')
    if (parts.length > 3) {
      return `.../${parts.slice(-2).join('/')}`
    }
    return path
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

  const toggleExpanded = () => {
    if (isExpandable.value) {
      isExpanded.value = !isExpanded.value
    }
  }

  const cleanToolResult = computed(() => {
    const result = toolResult.value || props.step.content

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
</script>

<style scoped>
  .tool-block {
    background: var(--bg-500);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius);
    font-size: 13px;
    max-width: 100%;
    margin-bottom: var(--spacing-xs, 4px);
  }

  .tool-header {
    padding: 8px 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    transition: background-color 0.2s ease;
  }

  .tool-header.expandable {
    cursor: pointer;
  }

  .tool-header.expandable:hover {
    background: var(--bg-600);
  }

  .tool-header.non-expandable {
    cursor: default;
    opacity: 0.8;
  }

  .tool-header.non-expandable:hover {
    background: transparent;
  }

  .tool-info {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .tool-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: var(--text-300);
  }

  .tool-icon svg {
    width: 14px;
    height: 14px;
  }

  .tool-name {
    color: var(--text-200);
    font-weight: 500;
    white-space: nowrap;
  }

  .tool-param {
    color: var(--text-300);
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 200px;
    background: transparent;
    margin-left: 8px;
  }

  .tool-command {
    color: var(--text-400);
    font-family: monospace;
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .status-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--color-success);
  }

  .status-dot.running {
    background: transparent;
    border: 2px solid var(--color-info);
    border-top-color: transparent;
    animation: spin 0.8s linear infinite;
  }

  .status-dot.error {
    background: var(--color-error);
  }

  .status-dot.completed {
    width: 6px;
    height: 6px;
  }

  .tool-result {
    padding: 8px;
    background: var(--bg-400);
    border-radius: var(--border-radius-sm);
  }

  .tool-result-content {
    font-family: monospace;
    color: var(--text-300);
    white-space: pre-wrap;
    word-wrap: break-word;
    font-size: 12px;
    line-height: 1.4;
  }

  @keyframes spin {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }
</style>
