<script setup lang="ts">
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import type { DiffContent, FileChange } from '@/api/git/types'

  interface Props {
    diff: DiffContent
    file: FileChange
    isStaged: boolean
  }

  const props = defineProps<Props>()
  const { t } = useI18n()

  const title = computed(() => {
    const prefix = props.isStaged ? t('git.diff_staged') : t('git.diff_unstaged')
    return `${prefix}: ${props.file.path}`
  })

  const getMarker = (lineType: string, content: string) => {
    if (lineType === 'added') return '+'
    if (lineType === 'removed') return '-'
    if (lineType === 'context') return ' '
    if (content.startsWith('\\')) return ' '
    return ''
  }

  const getDisplayText = (lineType: string, content: string) => {
    if (lineType === 'added' || lineType === 'removed' || lineType === 'context') {
      return content.length > 0 ? content.slice(1) : ''
    }
    return content
  }
</script>

<template>
  <div class="diff git-selectable">
    <div class="diff__title">{{ title }}</div>

    <div v-if="diff.hunks.length === 0" class="diff__empty">
      {{ t('git.no_diff') }}
    </div>

    <div v-else class="diff__content">
      <div v-for="(h, idx) in diff.hunks" :key="`${h.header}:${idx}`" class="hunk">
        <div class="hunk__header">{{ h.header }}</div>
        <div class="hunk__lines">
          <div v-for="(l, lineIdx) in h.lines" :key="`${idx}:${lineIdx}`" class="line" :class="`line--${l.lineType}`">
            <div class="line__marker">{{ getMarker(l.lineType, l.content) }}</div>
            <div class="line__nums">
              <span class="num">{{ l.oldLineNumber ?? '' }}</span>
              <span class="num">{{ l.newLineNumber ?? '' }}</span>
            </div>
            <pre class="line__text">{{ getDisplayText(l.lineType, l.content) }}</pre>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .diff {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    padding: 8px 10px;
  }

  .diff__title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-200);
    margin-bottom: 6px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .diff__empty {
    font-size: 12px;
    color: var(--text-300);
    padding: 8px 0;
  }

  .diff__content {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .hunk__header {
    font-family: var(--font-mono, var(--font-family-mono));
    font-size: 11px;
    color: var(--text-300);
    margin-bottom: 4px;
  }

  .hunk__lines {
    border: 1px solid var(--border-200);
    border-radius: 8px;
    overflow: hidden;
    background: var(--bg-50);
  }

  .line {
    display: grid;
    grid-template-columns: 16px 56px 1fr;
    gap: 10px;
    padding: 2px 8px;
    align-items: start;
  }

  .line__marker {
    font-family: var(--font-mono, var(--font-family-mono));
    font-size: 11px;
    color: var(--text-300);
    user-select: none;
  }

  .line__nums {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
    font-family: var(--font-mono, var(--font-family-mono));
    font-size: 11px;
    color: var(--text-300);
    user-select: none;
  }

  .num {
    text-align: right;
  }

  .line__text {
    margin: 0;
    font-family: var(--font-mono, var(--font-family-mono));
    font-size: 11px;
    color: var(--text-100);
    white-space: pre;
    overflow: hidden;
  }

  .line--added {
    background: rgba(60, 200, 120, 0.12);
  }

  .line--removed {
    background: rgba(255, 80, 80, 0.12);
  }

  .line--header {
    background: rgba(120, 120, 120, 0.12);
  }
</style>
