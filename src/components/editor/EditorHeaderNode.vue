<template>
  <EditorHeaderGroup v-if="node.type === 'leaf'" :groupId="node.groupId" />

  <EditorHeaderNode v-else-if="node.direction === 'column'" :node="node.first" />

  <div v-else class="editor-header-split row">
    <div class="editor-header-split__pane" :style="firstStyle">
      <EditorHeaderNode :node="node.first" />
    </div>

    <div class="editor-header-split__divider col"></div>

    <div class="editor-header-split__pane" :style="secondStyle">
      <EditorHeaderNode :node="node.second" />
    </div>
  </div>
</template>

<script setup lang="ts">
  import { computed } from 'vue'
  import type { GroupNode, GroupSplitNode } from '@/types/domain/storage'
  import EditorHeaderGroup from '@/components/editor/EditorHeaderGroup.vue'

  defineOptions({ name: 'EditorHeaderNode' })

  interface Props {
    node: GroupNode
  }

  const props = defineProps<Props>()

  const splitNode = computed<GroupSplitNode | null>(() => {
    if (props.node.type !== 'split') return null
    return props.node
  })

  const firstStyle = computed(() => {
    if (!splitNode.value) return {}
    return { flex: `${splitNode.value.ratio} 1 0` }
  })

  const secondStyle = computed(() => {
    if (!splitNode.value) return {}
    return { flex: `${1 - splitNode.value.ratio} 1 0` }
  })
</script>

<style scoped>
  .editor-header-split {
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    display: flex;
  }

  .editor-header-split__pane {
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    display: flex;
  }

  .editor-header-split.row > .editor-header-split__pane:first-child {
    padding-right: var(--spacing-xs);
  }

  .editor-header-split.row > .editor-header-split__pane:last-child {
    padding-left: var(--spacing-xs);
  }

  .editor-header-split__divider {
    flex: 0 0 auto;
    background: var(--border-200);
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.25);
  }

  .editor-header-split__divider.col {
    width: var(--split-divider-size);
  }
</style>
