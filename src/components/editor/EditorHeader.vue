<template>
  <div class="editor-header">
    <EditorHeaderNode :node="editorStore.workspace.root" />
  </div>
</template>

<script setup lang="ts">
  import { useEditorStore } from '@/stores/Editor'
  import EditorHeaderNode from '@/components/editor/EditorHeaderNode.vue'

  const editorStore = useEditorStore()
</script>

<style scoped>
  .editor-header {
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    display: flex;
    overflow: hidden;
  }
</style>
