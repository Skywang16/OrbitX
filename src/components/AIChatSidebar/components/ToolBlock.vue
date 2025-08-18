<template>
  <div class="tool-block">
    <div
      class="tool-header"
      :class="{ expandable: isExpandable, 'non-expandable': !isExpandable }"
      @click="toggleExpanded"
    >
      <div class="tool-info">
        <div class="tool-icon" v-html="getToolIcon(step.metadata?.toolName || 'read_file')"></div>
        <span class="tool-name">{{ step.metadata?.toolName || '工具调用' }}</span>
        <span class="tool-command" v-if="step.metadata?.toolCommand">{{ step.metadata.toolCommand }}</span>
        <span class="tool-url" v-if="getToolUrl()">{{ getToolUrl() }}</span>
      </div>
      <div
        class="status-dot"
        :class="{
          running: step.metadata?.status === 'running',
          error: step.metadata?.status === 'error',
        }"
      ></div>
    </div>

    <div v-if="isExpanded && step.metadata?.toolResult" class="tool-result" @click.stop>
      <!-- 特殊渲染edit_file工具的结果 -->
      <EditResult v-if="isEditResult(step.metadata.toolResult)" :editData="getEditData(step.metadata.toolResult)" />
      <!-- 普通工具结果 -->
      <div v-else class="tool-result-content">{{ step.metadata.toolResult }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, computed } from 'vue'
  import type { AIOutputStep } from '@/types/features/ai/chat'
  import EditResult from './EditResult.vue'
  import type { SimpleEditResult } from '@/eko/tools/toolList/edit-file'

  const props = defineProps<{
    step: AIOutputStep
  }>()

  const isExpanded = ref(false)

  // 计算属性：判断是否为可展开的工具（只有edit_file工具可以展开）
  const isExpandable = computed(() => {
    return props.step.metadata?.toolName === 'edit_file'
  })

  // 工具图标映射
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
    read_directory: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path d="M22 19C22 19.5304 21.7893 20.0391 21.4142 20.4142C21.0391 20.7893 20.5304 21 20 21H4C3.46957 21 2.96086 20.7893 2.58579 20.4142C2.21071 20.0391 2 19.5304 2 19V5C2 4.46957 2.21071 3.96086 2.58579 3.58579C2.96086 3.21071 3.46957 3 4 3H9L11 6H20C20.5304 6 21.0391 6.21071 21.4142 6.58579C21.7893 6.96086 22 7.46957 22 8V19Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>`,
    create_file: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path d="M14 2H6C4.9 2 4 2.9 4 4V20C4 21.1 4.89 22 5.99 22H18C19.1 22 20 21.1 20 20V8L14 2Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M14 2V8H20" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M12 18V12" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M9 15H15" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
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
  }

  // 获取工具图标
  const getToolIcon = (toolName: string) => {
    return toolIcons[toolName as keyof typeof toolIcons] || toolIcons.read_file
  }

  const toggleExpanded = () => {
    // 只有可展开的工具才能切换展开状态
    if (isExpandable.value) {
      isExpanded.value = !isExpanded.value
    }
  }

  // 判断是否为edit_file工具的结果
  const isEditResult = (result: any): boolean => {
    return result?.content?.[0]?.data?.file
  }

  // 获取EditData
  const getEditData = (result: any): SimpleEditResult => {
    return result?.content?.[0]?.data
  }

  // 获取工具URL（仅web_fetch工具）
  const getToolUrl = (): string => {
    if (props.step.metadata?.toolName === 'web_fetch') {
      // 尝试从toolCommand中解析URL
      const command = props.step.metadata?.toolCommand
      if (command) {
        try {
          const params = JSON.parse(command)
          return params.url || ''
        } catch {
          // 如果解析失败，直接返回command作为URL
          return command
        }
      }
    }
    return ''
  }
</script>

<style scoped>
  .tool-block {
    background: var(--bg-500);
    border: 1px solid var(--border-300);
    border-radius: 6px;
    font-size: 13px;
    max-width: 100%;
  }

  .tool-header {
    padding: 8px 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    transition: background-color 0.2s ease;
  }

  /* 可展开的工具样式 */
  .tool-header.expandable {
    cursor: pointer;
  }

  .tool-header.expandable:hover {
    background: var(--bg-600);
  }

  /* 不可展开的工具样式 */
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
    min-width: 0; /* 允许flex子元素收缩 */
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

  .tool-command {
    color: var(--text-400);
    font-family: monospace;
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .tool-url {
    color: var(--text-300);
    font-size: 11px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 200px;
    background: var(--bg-500);
    padding: 2px 6px;
    border-radius: 3px;
    border: 1px solid var(--border-300);
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-success);
  }

  .status-dot.running {
    background: var(--color-info);
    animation: pulse 1.5s infinite;
  }

  .status-dot.error {
    background: var(--color-error);
  }

  .tool-result {
    padding: 8px;
    background: var(--bg-400);
    border-radius: 4px;
  }

  .tool-result-content {
    font-family: monospace;
    color: var(--text-300);
    white-space: pre-wrap;
    word-wrap: break-word;
    font-size: 12px;
    line-height: 1.4;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.5;
    }
  }
</style>
