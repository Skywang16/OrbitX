<template>
  <div ref="headerRef" class="editor-header-group" @pointerdown.capture="handleActivateGroup">
    <div class="editor-header-group__inner" :style="headerPaddingStyle">
      <TabBar :groupId="groupId" :tabs="group.tabs" :activeTabId="group.activeTabId" />
    </div>
  </div>
</template>

<script setup lang="ts">
  import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
  import TabBar from '@/components/ui/TabBar.vue'
  import { useEditorStore } from '@/stores/Editor'
  import type { GroupId } from '@/types/domain/storage'
  import type { EditorDragPayload } from '@/types/domain/ui'
  import { EDITOR_TAB_DRAG_EVENT } from '@/types/domain/ui'
  import { getTopGroupIdsInLayout } from '@/utils/editorLayout'

  interface Props {
    groupId: GroupId
  }

  const props = defineProps<Props>()
  const editorStore = useEditorStore()

  const group = computed(() => editorStore.groups[props.groupId])
  const headerRef = ref<HTMLElement | null>(null)

  const topGroupIds = computed(() => getTopGroupIdsInLayout(editorStore.workspace.root))
  const isLeftmost = computed(() => topGroupIds.value[0] === props.groupId)
  const isRightmost = computed(() => topGroupIds.value[topGroupIds.value.length - 1] === props.groupId)

  const headerPaddingStyle = computed(() => {
    return {
      paddingLeft: isLeftmost.value ? 'var(--orbitx-titlebar-left-gutter, 0px)' : '0px',
      paddingRight: isRightmost.value ? 'var(--orbitx-titlebar-right-gutter, 0px)' : '0px',
    }
  })

  const handleActivateGroup = () => {
    if (document.body.classList.contains('orbitx-tab-dragging')) return
    editorStore.setActiveGroup(props.groupId)
  }

  /** Titlebar 区域只接受 center 落点（移动 tab），不触发分屏 */
  const isInsideHeader = (x: number, y: number): boolean => {
    const el = headerRef.value
    if (!el) return false
    const rect = el.getBoundingClientRect()
    return x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom
  }

  const handleDragEvent = async (payload: EditorDragPayload) => {
    if (!isInsideHeader(payload.x, payload.y)) return
    if (payload.sourceGroupId === props.groupId) return
    if (payload.phase !== 'end') return

    await editorStore.moveTab({ tabId: payload.tabId, targetGroupId: props.groupId, activate: true })
  }

  const onWindowDragEvent = (event: Event) => {
    const payload = (event as CustomEvent<EditorDragPayload>).detail
    if (!payload) return
    handleDragEvent(payload).catch(() => {})
  }

  onMounted(() => {
    window.addEventListener(EDITOR_TAB_DRAG_EVENT, onWindowDragEvent)
  })

  onBeforeUnmount(() => {
    window.removeEventListener(EDITOR_TAB_DRAG_EVENT, onWindowDragEvent)
  })
</script>

<style scoped>
  .editor-header-group {
    width: 100%;
    height: 100%;
    min-width: 0;
    display: flex;
    align-items: center;
    position: relative;
    background: var(--bg-200);
  }

  .editor-header-group__inner {
    width: 100%;
    min-width: 0;
    height: 100%;
    display: flex;
    align-items: center;
  }
</style>
