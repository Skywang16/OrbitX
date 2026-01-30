<template>
  <EditorGroup v-if="node.type === 'leaf'" :groupId="node.groupId" />

  <div
    v-else
    ref="splitRef"
    class="editor-split"
    :class="node.direction === 'row' ? 'row' : 'column'"
    @pointermove="handlePointerMove"
    @pointerleave="clearCornerHover"
  >
    <div ref="firstPaneRef" class="editor-split__pane" :style="firstStyle">
      <EditorSplitNode :node="node.first" />
    </div>

    <div
      class="editor-split__divider"
      :class="[
        node.direction === 'row' ? 'col' : 'row',
        {
          corner: cornerHoverTarget !== null,
        },
      ]"
      @pointerdown="handleDividerPointerDown"
    ></div>

    <div ref="secondPaneRef" class="editor-split__pane" :style="secondStyle">
      <EditorSplitNode :node="node.second" />
    </div>
  </div>
</template>

<script setup lang="ts">
  import { computed, ref } from 'vue'
  import type { GroupNode, GroupSplitNode } from '@/types/domain/storage'
  import { useEditorStore } from '@/stores/Editor'
  import EditorGroup from '@/components/editor/EditorGroup.vue'

  defineOptions({ name: 'EditorSplitNode' })

  interface Props {
    node: GroupNode
  }

  const props = defineProps<Props>()
  const editorStore = useEditorStore()

  const splitRef = ref<HTMLElement | null>(null)
  const firstPaneRef = ref<HTMLElement | null>(null)
  const secondPaneRef = ref<HTMLElement | null>(null)

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

  /** 限制分割比例在 10%-90% 之间 */
  const clampRatio = (value: number) => Math.max(0.1, Math.min(0.9, value))

  /** 对齐到设备像素，避免分割线模糊 */
  const snapToDevicePixel = (raw: number) => {
    const dpr = window.devicePixelRatio || 1
    return Math.round(raw * dpr) / dpr
  }

  // Corner resize: 同时调整父子两个分割线（十字交叉点）
  type CornerTarget = { parentSplitId: string; childSplitId: string; pane: 'first' | 'second' }
  const cornerHoverTarget = ref<CornerTarget | null>(null)

  const clearCornerHover = () => {
    cornerHoverTarget.value = null
  }

  /** 检测鼠标是否在两条分割线的交叉点附近（10px 命中区域） */
  const getCornerHitTargets = (containerRect: DOMRect, x: number, y: number): CornerTarget | null => {
    const parent = splitNode.value
    if (!parent) return null

    const hitRadius = 10
    const isRow = parent.direction === 'row'
    const dividerPos = isRow
      ? containerRect.left + containerRect.width * parent.ratio
      : containerRect.top + containerRect.height * parent.ratio

    const nearParentDivider = isRow ? Math.abs(x - dividerPos) <= hitRadius : Math.abs(y - dividerPos) <= hitRadius
    if (!nearParentDivider) return null

    const checkPane = (pane: 'first' | 'second') => {
      const child = pane === 'first' ? parent.first : parent.second
      if (child.type !== 'split') return null
      if (child.direction === parent.direction) return null

      const paneEl = pane === 'first' ? firstPaneRef.value : secondPaneRef.value
      const paneRect = paneEl ? paneEl.getBoundingClientRect() : containerRect

      const childDividerPos =
        child.direction === 'row'
          ? paneRect.left + paneRect.width * child.ratio
          : paneRect.top + paneRect.height * child.ratio

      const nearChildDivider =
        child.direction === 'row'
          ? Math.abs(x - childDividerPos) <= hitRadius
          : Math.abs(y - childDividerPos) <= hitRadius

      if (!nearChildDivider) return null
      return { parentSplitId: parent.id, childSplitId: child.id, pane }
    }

    return checkPane('first') ?? checkPane('second')
  }

  const handlePointerMove = (event: PointerEvent) => {
    const container = splitRef.value
    if (!container) return
    const rect = container.getBoundingClientRect()
    cornerHoverTarget.value = getCornerHitTargets(rect, event.clientX, event.clientY)
  }

  const handleDividerPointerDown = (event: PointerEvent) => {
    if (cornerHoverTarget.value) {
      startCornerResize(event, cornerHoverTarget.value)
      return
    }
    startResize(event)
  }

  /** 启动拖拽：捕获指针并注册事件 */
  const setupDragHandlers = (event: PointerEvent, onMove: (e: PointerEvent) => void) => {
    event.preventDefault()
    const handle = event.currentTarget as HTMLElement | null
    if (handle) {
      try {
        handle.setPointerCapture(event.pointerId)
      } catch {
        // ignore - 某些情况下可能失败
      }
    }

    const handleUp = () => {
      window.removeEventListener('pointermove', onMove)
      window.removeEventListener('pointerup', handleUp)
    }

    window.addEventListener('pointermove', onMove)
    window.addEventListener('pointerup', handleUp, { once: true })
  }

  /** 单分割线拖拽 */
  const startResize = (event: PointerEvent) => {
    if (!splitNode.value) return
    const container = splitRef.value
    if (!container) return

    const rect = container.getBoundingClientRect()
    const isRow = splitNode.value.direction === 'row'
    const splitId = splitNode.value.id

    setupDragHandlers(event, (e: PointerEvent) => {
      const raw = isRow ? e.clientX - rect.left : e.clientY - rect.top
      const snapped = snapToDevicePixel(raw)
      const ratio = isRow ? snapped / rect.width : snapped / rect.height
      editorStore.updateSplitRatio(splitId, clampRatio(ratio))
    })
  }

  /** 十字交叉点拖拽：同时调整父子两个分割比例 */
  const startCornerResize = (event: PointerEvent, target: CornerTarget) => {
    const container = splitRef.value
    const parent = splitNode.value
    if (!container || !parent) return

    const childNode = target.pane === 'first' ? parent.first : parent.second
    if (childNode.type !== 'split') return

    const parentSplitId = parent.id
    const childSplitId = childNode.id

    setupDragHandlers(event, (e: PointerEvent) => {
      const containerRect = container.getBoundingClientRect()

      // 更新父分割比例
      const parentRaw = parent.direction === 'row' ? e.clientX - containerRect.left : e.clientY - containerRect.top
      const parentRatio =
        parent.direction === 'row'
          ? snapToDevicePixel(parentRaw) / containerRect.width
          : snapToDevicePixel(parentRaw) / containerRect.height
      editorStore.updateSplitRatio(parentSplitId, clampRatio(parentRatio))

      // 更新子分割比例
      const paneEl = target.pane === 'first' ? firstPaneRef.value : secondPaneRef.value
      const paneRect = paneEl ? paneEl.getBoundingClientRect() : containerRect
      const childRaw = childNode.direction === 'row' ? e.clientX - paneRect.left : e.clientY - paneRect.top
      const childRatio =
        childNode.direction === 'row'
          ? snapToDevicePixel(childRaw) / paneRect.width
          : snapToDevicePixel(childRaw) / paneRect.height
      editorStore.updateSplitRatio(childSplitId, clampRatio(childRatio))
    })
  }
</script>

<style scoped>
  .editor-split {
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    display: flex;
  }

  .editor-split.row {
    flex-direction: row;
  }

  .editor-split.column {
    flex-direction: column;
  }

  .editor-split__pane {
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    display: flex;
  }

  .editor-split__divider {
    flex-shrink: 0;
    background: var(--border-200);
  }

  .editor-split__divider.col {
    width: var(--split-divider-size);
    cursor: col-resize;
  }

  .editor-split__divider.row {
    height: var(--split-divider-size);
    cursor: row-resize;
  }

  .editor-split__divider:hover {
    background: var(--color-primary);
  }

  .editor-split__divider.corner {
    cursor: move;
  }
</style>
