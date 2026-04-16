<template>
  <div class="edit-result">
    <div class="edit-header" @click="toggleCollapsed">
      <div class="header-left">
        <img v-if="!iconError" :src="fileIconUrl" class="file-icon" width="14" height="14" @error="iconError = true" />
        <svg v-else class="file-icon" width="14" height="14" viewBox="0 0 16 16" fill="none">
          <path
            d="M3 1h6.5L13 4.5V14a1 1 0 01-1 1H4a1 1 0 01-1-1V2a1 1 0 011-1z"
            stroke="var(--text-500)"
            stroke-width="1"
            fill="var(--bg-200)"
          />
          <path d="M9.5 1v3.5H13" stroke="var(--text-500)" stroke-width="1" fill="none" />
        </svg>
        <span class="file-path">{{ editData.file }}</span>
      </div>
      <div class="header-right">
        <span class="diff-stats">
          <span class="stat added">+{{ diffStats.added }}</span>
          <span class="stat removed">-{{ diffStats.removed }}</span>
        </span>
        <svg class="chevron" :class="{ collapsed: isCollapsed }" width="12" height="12" viewBox="0 0 12 12">
          <path d="M4 3l3 3-3 3" stroke="currentColor" stroke-width="1.5" fill="none" />
        </svg>
      </div>
    </div>

    <div v-if="!isCollapsed" ref="diffWrapper" class="diff-wrapper" :class="{ expanded: isFullyExpanded }">
      <div class="diff-content">
        <div v-for="(line, i) in oldLines" :key="`old-${i}`" class="diff-line removed">
          <span class="line-marker">-</span>
          <pre class="line-code">{{ line }}</pre>
        </div>
        <div v-for="(line, i) in newLines" :key="`new-${i}`" class="diff-line added">
          <span class="line-marker">+</span>
          <pre class="line-code">{{ line }}</pre>
        </div>
      </div>
      <div v-if="showExpandBtn" class="expand-overlay" @click.stop="isFullyExpanded = !isFullyExpanded">
        <svg v-if="!isFullyExpanded" width="20" height="20" viewBox="0 0 20 20">
          <path d="M6 10l4 4 4-4" stroke="currentColor" stroke-width="2" fill="none" />
        </svg>
        <svg v-else width="20" height="20" viewBox="0 0 20 20">
          <path d="M6 10l4-4 4 4" stroke="currentColor" stroke-width="2" fill="none" />
        </svg>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { computed, nextTick, onMounted, ref } from 'vue'
  import { getIconForFile } from 'vscode-icons-js'

  interface EditResultData {
    file: string
    old: string
    new: string
  }

  const props = defineProps<{
    editData: EditResultData
  }>()

  const isCollapsed = ref(false)
  const isFullyExpanded = ref(false)
  const diffWrapper = ref<HTMLElement>()
  const showExpandBtn = ref(false)

  const toggleCollapsed = () => {
    isCollapsed.value = !isCollapsed.value
  }

  const oldLines = computed(() => (props.editData.old || '').split('\n'))
  const newLines = computed(() => (props.editData.new || '').split('\n'))

  const iconError = ref(false)

  const fileIconUrl = computed(() => {
    const filename = props.editData.file.split('/').pop() || ''
    const iconName = getIconForFile(filename) || 'default_file.svg'
    return `/icons/files/${iconName}`
  })

  const diffStats = computed(() => {
    const removed = oldLines.value.length
    const added = newLines.value.length
    return { removed, added }
  })

  onMounted(() => {
    nextTick(() => {
      if (diffWrapper.value) {
        showExpandBtn.value = diffWrapper.value.scrollHeight > 200
      }
    })
  })
</script>

<style scoped>
  .edit-result {
    margin: 6px 0;
    border: 1px solid var(--border-200);
    border-radius: 10px;
    overflow: hidden;
    background: var(--bg-200);
  }

  .edit-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: var(--bg-300);
    cursor: pointer;
    user-select: none;
    transition: background 0.15s ease;
    gap: 12px;
  }

  .edit-header:hover {
    background: var(--bg-400);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .file-icon {
    flex-shrink: 0;
  }

  .file-path {
    font-size: 12px;
    color: var(--text-300);
    font-family: var(--font-family-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .diff-stats {
    display: flex;
    gap: 6px;
    font-size: 11px;
    font-weight: 500;
  }

  .stat.added {
    color: #4caf50;
  }

  .stat.removed {
    color: #ef4444;
  }

  .chevron {
    color: var(--text-500);
    transition: transform 0.2s ease;
    transform: rotate(90deg);
    opacity: 0.5;
  }

  .edit-header:hover .chevron {
    opacity: 1;
  }

  .chevron.collapsed {
    transform: rotate(0deg);
  }

  .diff-wrapper {
    position: relative;
    max-height: 200px;
    overflow: hidden;
    transition: max-height 0.3s ease;
  }

  .diff-wrapper.expanded {
    max-height: 600px;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .diff-content {
    background: var(--bg-100);
  }

  .diff-line {
    display: flex;
    font-size: 11px;
    line-height: 18px;
    padding: 0 8px;
  }

  .diff-line.removed {
    background: color-mix(in srgb, #ef4444 3%, transparent);
  }

  .diff-line.added {
    background: color-mix(in srgb, #4caf50 3%, transparent);
  }

  .line-marker {
    width: 16px;
    color: var(--text-500);
    user-select: none;
    flex-shrink: 0;
  }

  .diff-line.removed .line-marker {
    color: #ef4444;
  }

  .diff-line.added .line-marker {
    color: #4caf50;
  }

  .line-code {
    flex: 1;
    margin: 0;
    font-family: var(--font-family-mono);
    color: var(--text-400);
    white-space: pre-wrap;
    word-break: break-all;
    min-width: 0;
  }

  .diff-line.removed .line-code {
    color: color-mix(in srgb, #ef4444 80%, var(--text-400));
  }

  .diff-line.added .line-code {
    color: color-mix(in srgb, #4caf50 80%, var(--text-400));
  }

  .expand-overlay {
    position: sticky;
    bottom: 0;
    left: 0;
    right: 0;
    height: 60px;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding-bottom: 8px;
    background: linear-gradient(to top, var(--bg-100) 40%, transparent);
    cursor: pointer;
    color: var(--text-500);
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .diff-wrapper:hover .expand-overlay {
    opacity: 1;
  }

  .expand-overlay:hover {
    color: var(--text-300);
  }
</style>
