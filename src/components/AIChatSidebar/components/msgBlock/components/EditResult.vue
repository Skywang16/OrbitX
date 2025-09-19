<template>
  <div class="edit-result">
    <div class="stats">
      <span class="stat removed">-{{ getLineCount(editData.old) }}</span>
      <span class="stat added">+{{ getLineCount(editData.new) }}</span>
    </div>

    <div class="code-diff">
      <div class="diff-line removed">
        <span class="line-prefix">-</span>
        <span class="line-content">{{ editData.old }}</span>
      </div>
      <div class="diff-line added">
        <span class="line-prefix">+</span>
        <span class="line-content">{{ editData.new }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { computed } from 'vue'

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
    editData: EditResultData
  }>()

  const getLineCount = (text: string) => {
    return text ? text.split('\n').length : 0
  }
</script>

<style scoped>
  .edit-result {
    background: var(--bg-400);
    border-radius: var(--border-radius-sm);
    overflow: hidden;
  }

  .stats {
    padding: 8px 12px;
    display: flex;
    gap: 12px;
    background: var(--bg-300);
    border-bottom: 1px solid var(--border-200);
  }

  .stat {
    font-size: 11px;
    font-weight: 400;
    padding: 2px 6px;
    border-radius: var(--border-radius-xs);
  }

  .stat.added {
    background: rgba(34, 197, 94, 0.1);
    color: #16a34a;
  }

  .stat.removed {
    background: rgba(239, 68, 68, 0.1);
    color: #dc2626;
  }

  .code-diff {
    overflow: hidden;
  }

  .diff-line {
    display: flex;
    align-items: flex-start;
    font-family: 'SF Mono', 'Monaco', 'Inconsolata', 'Roboto Mono', monospace;
    font-size: 12px;
    line-height: 1.4;
    padding: 4px 0;
  }

  .diff-line.added {
    background: rgba(34, 197, 94, 0.05);
  }

  .diff-line.removed {
    background: rgba(239, 68, 68, 0.05);
  }

  .line-prefix {
    color: var(--text-400);
    padding: 0 12px;
    min-width: 30px;
    text-align: center;
    flex-shrink: 0;
    user-select: none;
    font-weight: 500;
  }

  .line-content {
    color: var(--text-200);
    padding-right: 12px;
    white-space: pre-wrap;
    word-break: break-all;
    flex: 1;
  }

  .diff-line.added .line-prefix,
  .diff-line.added .line-content {
    color: #16a34a;
  }

  .diff-line.removed .line-prefix,
  .diff-line.removed .line-content {
    color: #dc2626;
  }
</style>
