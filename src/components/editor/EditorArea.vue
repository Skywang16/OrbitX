<template>
  <div class="editor-area">
    <EditorSplitNode :node="editorStore.workspace.root" />
  </div>
</template>

<script setup lang="ts">
  import { useEditorStore } from '@/stores/Editor'
  import EditorSplitNode from '@/components/editor/EditorSplitNode.vue'

  const editorStore = useEditorStore()
</script>

<style scoped>
  .editor-area {
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    display: flex;
  }
</style>
