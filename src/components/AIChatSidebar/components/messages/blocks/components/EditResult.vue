<template>
  <div class="edit-result">
    <div class="header">
      <div class="file-info">
        <svg class="file-icon" width="14" height="14" viewBox="0 0 14 14" fill="none">
          <path
            d="M2 2.5C2 2.22386 2.22386 2 2.5 2H8.5V4.5C8.5 4.77614 8.72386 5 9 5H11.5V11.5C11.5 11.7761 11.2761 12 11 12H2.5C2.22386 12 2 11.7761 2 11.5V2.5ZM3.5 3.5V10.5H10V3.5H3.5Z"
            fill="currentColor"
          />
        </svg>
        <span class="file-path">{{ formatFilePath(editData.file) }}</span>
      </div>
      <div class="stats">
        <span class="stat removed">
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path
              d="M3.5 3.5L8.5 8.5M8.5 3.5L3.5 8.5"
              stroke="currentColor"
              stroke-width="1.2"
              stroke-linecap="round"
            />
          </svg>
          -{{ getLineCount(editData.old) }}
        </span>
        <span class="stat added">
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path d="M6 3V9M3 6H9" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" />
          </svg>
          +{{ getLineCount(editData.new) }}
        </span>
      </div>
    </div>

    <div class="code-diff">
      <!-- Removed block -->
      <div class="diff-block removed">
        <div class="diff-gutter"></div>
        <div class="diff-lines">
          <template v-for="(line, index) in getLines(editData.old)" :key="`old-${index}`">
            <div class="diff-line removed">
              <span class="line-number">{{ index + 1 }}</span>
              <span class="line-marker">-</span>
              <code class="line-content">{{ line || ' ' }}</code>
            </div>
          </template>
          <div v-if="!editData.old" class="diff-line empty">
            <span class="line-number">1</span>
            <span class="line-marker">-</span>
            <code class="line-content empty-content">No content to remove</code>
          </div>
        </div>
      </div>

      <!-- Divider -->
      <div class="diff-divider"></div>

      <!-- Added block -->
      <div class="diff-block added">
        <div class="diff-gutter"></div>
        <div class="diff-lines">
          <template v-for="(line, index) in getLines(editData.new)" :key="`new-${index}`">
            <div class="diff-line added">
              <span class="line-number">{{ index + 1 }}</span>
              <span class="line-marker">+</span>
              <code class="line-content">{{ line || ' ' }}</code>
            </div>
          </template>
          <div v-if="!editData.new" class="diff-line empty">
            <span class="line-number">1</span>
            <span class="line-marker">+</span>
            <code class="line-content empty-content">No content to add</code>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
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

  defineProps<{
    editData: EditResultData
  }>()

  const getLineCount = (text: string) => {
    return text ? text.split('\n').length : 0
  }

  const getLines = (text: string) => {
    return text ? text.split('\n') : []
  }

  const formatFilePath = (path: string) => {
    if (!path) return 'unknown'
    // 只显示文件名，不显示完整路径
    const parts = path.split(/[/\\]/)
    return parts[parts.length - 1]
  }
</script>

<style scoped>
  .edit-result {
    background: var(--bg-300);
    border-radius: var(--border-radius);
    overflow: hidden;
    border: 1px solid var(--border-300);
    font-family: var(--font-mono);
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: var(--bg-200);
    border-bottom: 1px solid var(--border-200);
    min-height: 36px;
    gap: 12px;
  }

  .file-info {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    min-width: 0;
  }

  .file-icon {
    color: var(--text-400);
    flex-shrink: 0;
  }

  .file-path {
    font-size: 12px;
    color: var(--text-200);
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stats {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }

  .stat {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    font-weight: 500;
    padding: 2px 6px;
    border-radius: 3px;
  }

  .stat.added {
    background: color-mix(in srgb, var(--color-success, #4caf50) 15%, transparent);
    color: var(--color-success, #4caf50);
  }

  .stat.removed {
    background: color-mix(in srgb, var(--color-danger, #ef4444) 15%, transparent);
    color: var(--color-danger, #ef4444);
  }

  .stat svg {
    flex-shrink: 0;
  }

  .code-diff {
    display: flex;
    flex-wrap: wrap;
    background: var(--bg-300);
    gap: 8px;
  }

  .diff-block {
    flex: 1;
    display: flex;
    min-width: 0;
    border-radius: 4px;
  }

  .diff-block.removed {
    background: color-mix(in srgb, var(--color-danger, #ef4444) 8%, transparent);
  }

  .diff-block.added {
    background: color-mix(in srgb, var(--color-success, #4caf50) 8%, transparent);
  }

  .diff-gutter {
    width: 0;
    flex-shrink: 0;
  }

  .diff-lines {
    flex: 1;
    overflow-x: auto;
    overflow-y: hidden;
  }

  .diff-divider {
    width: 1px;
    background: var(--border-200);
    flex-shrink: 0;
  }

  .diff-line {
    display: flex;
    align-items: stretch;
    font-size: 12px;
    line-height: 20px;
    min-height: 20px;
    border-bottom: 1px solid color-mix(in srgb, var(--border-300) 40%, transparent);
  }

  .diff-line:last-child {
    border-bottom: none;
  }

  .diff-line:hover {
    background: color-mix(in srgb, var(--text-100) 6%, transparent);
  }

  .line-number {
    width: 36px;
    min-width: 36px;
    text-align: right;
    padding-right: 12px;
    color: var(--text-400);
    user-select: none;
    font-size: 11px;
    line-height: 20px;
    background: color-mix(in srgb, var(--bg-400) 40%, transparent);
  }

  .line-marker {
    width: 20px;
    min-width: 20px;
    text-align: center;
    user-select: none;
    font-size: 11px;
    line-height: 20px;
    font-weight: 600;
  }

  .diff-line.removed .line-marker {
    color: var(--color-danger, #ef4444);
  }

  .diff-line.added .line-marker {
    color: var(--color-success, #4caf50);
  }

  .line-content {
    flex: 1;
    padding: 0 12px 0 8px;
    white-space: pre-wrap;
    overflow-x: auto;
    line-height: 20px;
    color: var(--text-200);
  }

  .diff-line.removed .line-content {
    color: var(--color-danger, #ef4444);
    background: color-mix(in srgb, var(--color-danger, #ef4444) 12%, transparent);
  }

  .diff-line.added .line-content {
    color: var(--color-success, #4caf50);
    background: color-mix(in srgb, var(--color-success, #4caf50) 12%, transparent);
  }

  .diff-line.empty {
    opacity: 0.5;
  }

  .empty-content {
    color: var(--text-400) !important;
    background: transparent !important;
    font-style: italic;
  }

  .diff-lines::-webkit-scrollbar {
    height: 8px;
  }

  .diff-lines::-webkit-scrollbar-track {
    background: transparent;
  }

  .diff-lines::-webkit-scrollbar-thumb {
    background: var(--border-200);
    border-radius: 4px;
  }

  .diff-lines::-webkit-scrollbar-thumb:hover {
    background: var(--border-300);
  }

  @media (max-width: 640px) {
    .code-diff {
      flex-direction: column;
    }

    .diff-divider {
      display: none;
    }
  }
</style>
