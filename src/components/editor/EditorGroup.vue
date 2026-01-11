<template>
  <div ref="groupRef" class="editor-group" :data-group-id="groupId" @pointerdown.capture="handleActivateGroup">
    <div v-if="!isInTitlebar" class="editor-group__tabbar">
      <TabBar :groupId="groupId" :tabs="group.tabs" :activeTabId="group.activeTabId" />
    </div>

    <div class="editor-group__content" @dragover.prevent>
      <EmptyState v-if="group.tabs.length === 0" />

      <component
        v-else-if="activeTab"
        :is="getTabDefinition(activeTab.type).component"
        v-bind="getTabDefinition(activeTab.type).getComponentProps(activeTab as never)"
      />
    </div>

    <div v-if="dropState.visible" class="editor-group__drop-overlay">
      <div v-if="dropState.zone === 'left'" class="editor-group__drop-hint left"></div>
      <div v-if="dropState.zone === 'right'" class="editor-group__drop-hint right"></div>
      <div v-if="dropState.zone === 'top'" class="editor-group__drop-hint top"></div>
      <div v-if="dropState.zone === 'bottom'" class="editor-group__drop-hint bottom"></div>
      <div v-if="dropState.zone === 'center'" class="editor-group__drop-hint center"></div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
  import TabBar from '@/components/ui/TabBar.vue'
  import { useEditorStore } from '@/stores/Editor'
  import EmptyState from '@/views/EmptyState/EmptyStateView.vue'
  import { getTabDefinition } from '@/tabs/registry'
  import type { GroupId } from '@/types/domain/storage'
  import type { EditorDragPayload, EditorDropZone } from '@/types/domain/ui'
  import { EDITOR_TAB_DRAG_EVENT } from '@/types/domain/ui'
  import { getTopGroupIdsInLayout } from '@/utils/editorLayout'

  interface Props {
    groupId: GroupId
  }

  const props = defineProps<Props>()

  const editorStore = useEditorStore()

  const groupRef = ref<HTMLElement | null>(null)

  const group = computed(() => {
    return editorStore.groups[props.groupId]
  })

  const isInTitlebar = computed(() => {
    return getTopGroupIdsInLayout(editorStore.workspace.root).includes(props.groupId)
  })

  const activeTab = computed(() => {
    const id = group.value.activeTabId
    if (!id) return null
    return group.value.tabs.find(t => t.id === id) ?? null
  })

  const dropState = ref<{ visible: boolean; zone: EditorDropZone | null; payload: EditorDragPayload | null }>({
    visible: false,
    zone: null,
    payload: null,
  })

  /**
   * 根据鼠标位置判断落入哪个区域
   * 边缘阈值：水平方向 28% (72-320px)，垂直方向 34% (88-320px)
   */
  const pickZone = (x: number, y: number): EditorDropZone | null => {
    const el = groupRef.value
    if (!el) return null
    const rect = el.getBoundingClientRect()
    if (rect.width <= 0 || rect.height <= 0) return null
    if (x < rect.left || x > rect.right || y < rect.top || y > rect.bottom) return null

    const localX = x - rect.left
    const localY = y - rect.top

    const left = localX
    const right = rect.width - localX
    const top = localY
    const bottom = rect.height - localY

    const thresholdX = Math.min(320, Math.max(72, rect.width * 0.28))
    const thresholdY = Math.min(320, Math.max(88, rect.height * 0.34))

    const candidates: Array<{ zone: EditorDropZone; dist: number; ok: boolean }> = [
      { zone: 'left', dist: left, ok: left <= thresholdX },
      { zone: 'right', dist: right, ok: right <= thresholdX },
      { zone: 'top', dist: top, ok: top <= thresholdY },
      { zone: 'bottom', dist: bottom, ok: bottom <= thresholdY },
    ]

    const edge = candidates.filter(c => c.ok).sort((a, b) => a.dist - b.dist)[0]
    return edge?.zone ?? 'center'
  }

  const handleActivateGroup = () => {
    if (document.body.classList.contains('orbitx-tab-dragging')) return
    editorStore.setActiveGroup(props.groupId)
  }

  const handleDragEvent = async (payload: EditorDragPayload) => {
    const zone = pickZone(payload.x, payload.y)
    if (!zone) {
      dropState.value = { visible: false, zone: null, payload: null }
      return
    }

    const isSameGroup = payload.sourceGroupId === props.groupId
    const canSplitSelfNow = isSameGroup && group.value.tabs.length > 1

    // 同分区拖动：只允许拖到边缘分屏，且必须保证源分区仍有 tab
    if (isSameGroup && (zone === 'center' || !canSplitSelfNow)) {
      dropState.value = { visible: false, zone: null, payload: null }
      return
    }

    if (payload.phase === 'start' || payload.phase === 'move') {
      dropState.value = { visible: true, zone, payload }
      return
    }

    if (payload.phase !== 'end') return

    const final = dropState.value
    dropState.value = { visible: false, zone: null, payload: null }

    if (!final.payload || !final.zone) return

    if (final.zone === 'center') {
      if (final.payload.sourceGroupId === props.groupId) return
      await editorStore.moveTab({ tabId: final.payload.tabId, targetGroupId: props.groupId, activate: true })
      return
    }

    if (final.payload.sourceGroupId === props.groupId && group.value.tabs.length <= 1) return

    await editorStore.splitGroupWithTab({
      tabId: final.payload.tabId,
      targetGroupId: props.groupId,
      zone: final.zone,
    })
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
  .editor-group {
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    position: relative;
    background: var(--bg-200);
  }

  .editor-group__tabbar {
    flex: 0 0 auto;
    height: var(--titlebar-height);
    background: var(--bg-200);
    border-bottom: 1px solid var(--border-200);
  }

  .editor-group__content {
    position: relative;
    flex: 1;
    background: var(--bg-200);
  }

  .editor-group__drop-overlay {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 50;
  }

  .editor-group__drop-hint {
    position: absolute;
    border: 2px solid rgba(59, 130, 246, 0.65);
    background: rgba(59, 130, 246, 0.12);
    border-radius: 10px;
  }

  .editor-group__drop-hint.left {
    top: 8px;
    bottom: 8px;
    left: 8px;
    width: calc(60% - 12px);
  }

  .editor-group__drop-hint.right {
    top: 8px;
    bottom: 8px;
    right: 8px;
    width: calc(60% - 12px);
  }

  .editor-group__drop-hint.top {
    left: 8px;
    right: 8px;
    top: 8px;
    height: calc(60% - 12px);
  }

  .editor-group__drop-hint.bottom {
    left: 8px;
    right: 8px;
    bottom: 8px;
    height: calc(60% - 12px);
  }

  .editor-group__drop-hint.center {
    top: 10px;
    bottom: 10px;
    left: 10px;
    right: 10px;
  }
</style>
