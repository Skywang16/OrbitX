<template>
  <TodoTool v-if="isTodo" :block="block" />
  <ShellTool v-else-if="isShell" :block="block" />
  <EditResult v-else-if="isEdit" :edit-data="editData" />
  <GenericTool v-else :block="block" :disable-expand="disableExpand" />
</template>

<script setup lang="ts">
  import type { Block } from '@/types'
  import { computed } from 'vue'
  import EditResult from './components/EditResult.vue'
  import GenericTool from './tools/GenericTool.vue'
  import ShellTool from './tools/ShellTool.vue'
  import TodoTool from './tools/TodoTool.vue'

  const props = defineProps<{
    block: Extract<Block, { type: 'tool' }>
    disableExpand?: boolean
  }>()

  interface EditResultData {
    file: string
    old: string
    new: string
  }

  const toolName = computed(() => props.block.name || '')
  const isTodo = computed(() => toolName.value === 'todowrite')
  const isShell = computed(() => toolName.value === 'shell')
  const isEdit = computed(() => toolName.value === 'edit_file' || toolName.value === 'multi_edit_file')

  const editData = computed<EditResultData>(() => {
    if (!isEdit.value) return { file: '', old: '', new: '' }
    return (props.block.output?.metadata as EditResultData) || { file: '', old: '', new: '' }
  })
</script>
