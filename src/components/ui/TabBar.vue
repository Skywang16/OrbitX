<script setup lang="ts">
  import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useTerminalStore } from '@/stores/Terminal'
  import { useEditorStore } from '@/stores/Editor'
  import type { GroupId, TabId, TabState, TerminalTabState } from '@/types/domain/storage'
  import { showPopoverAt } from '@/ui/composables/popover-api'
  import { getTabDefinition } from '@/tabs/registry'

  const { t } = useI18n()

  interface Props {
    groupId: GroupId
    tabs: TabState[]
    activeTabId: TabId | null
  }

  const props = defineProps<Props>()

  const terminalStore = useTerminalStore()
  const editorStore = useEditorStore()

  const getTabClass = (tab: TabState): string[] => {
    const classes = ['tab']
    if (tab.id === props.activeTabId && props.groupId === editorStore.activeGroupId) {
      classes.push('active')
    }
    return classes
  }

  const getTerminalCwd = (paneId: number): string => {
    const terminal = terminalStore.terminals.find(t => t.id === paneId)
    return terminal?.cwd ?? ''
  }

  const getPresentation = (tab: TabState) => {
    return getTabDefinition(tab.type).getPresentation(tab as never, {
      t,
      getTerminalCwd,
    })
  }

  const tabStripRef = ref<HTMLDivElement | null>(null)

  const MIN_TAB_WIDTH = 60
  const MAX_TAB_WIDTH = 150
  const TAB_GAP = 4
  const ADD_BUTTON_WIDTH = 28
  const ADD_BUTTON_RESERVED = ADD_BUTTON_WIDTH + TAB_GAP

  const stripWidth = ref(0)
  let resizeObserver: ResizeObserver | null = null

  const updateStripWidth = () => {
    const el = tabStripRef.value
    if (!el) return
    stripWidth.value = el.clientWidth
  }

  onMounted(() => {
    updateStripWidth()
    const el = tabStripRef.value
    if (!el) return
    resizeObserver = new ResizeObserver(() => updateStripWidth())
    resizeObserver.observe(el)
  })

  onBeforeUnmount(() => {
    resizeObserver?.disconnect()
    resizeObserver = null
  })

  const tabLayout = computed(() => {
    const tabCount = props.tabs.length
    const containerWidth = stripWidth.value || tabStripRef.value?.clientWidth || 0

    if (tabCount === 0 || containerWidth <= 0) {
      return { tabWidth: MAX_TAB_WIDTH, needsScroll: false, showInlineAdd: true }
    }

    const totalGaps = TAB_GAP * Math.max(0, tabCount - 1)

    const compute = (reserved: number) => {
      const minTotal = tabCount * MIN_TAB_WIDTH + totalGaps + reserved
      const needsScroll = minTotal > containerWidth

      const availableWidth = containerWidth - totalGaps - reserved
      const widthPerTab = availableWidth / tabCount
      const tabWidth = widthPerTab <= MIN_TAB_WIDTH ? MIN_TAB_WIDTH : Math.min(MAX_TAB_WIDTH, widthPerTab)

      return { tabWidth, needsScroll }
    }

    const base = compute(0)
    const withInline = compute(ADD_BUTTON_RESERVED)
    const showInlineAdd = !withInline.needsScroll && withInline.tabWidth >= MAX_TAB_WIDTH
    const final = showInlineAdd ? withInline : base

    return { tabWidth: final.tabWidth, needsScroll: final.needsScroll, showInlineAdd }
  })

  const tabWidth = computed(() => tabLayout.value.tabWidth)
  const needsScroll = computed(() => tabLayout.value.needsScroll)
  const showInlineAdd = computed(() => tabLayout.value.showInlineAdd)

  const canShowCloseButton = (tab: TabState): boolean => {
    return getTabDefinition(tab.type).isClosable(tab as never)
  }

  const isTerminalTab = (tab: TabState): tab is TerminalTabState => {
    return tab.type === 'terminal'
  }

  const handleTabClick = (id: TabId) => {
    if (id !== props.activeTabId) {
      editorStore.setActiveTab(props.groupId, id)
    }
  }

  const handleCloseClick = (event: MouseEvent, id: TabId) => {
    event.stopPropagation()
    editorStore.closeTab(props.groupId, id)
  }

  type DragPhase = 'start' | 'move' | 'end'
  type DragPayload = { phase: DragPhase; tabId: string; sourceGroupId: string; x: number; y: number }

  const ORBITX_EDITOR_TAB_DRAG_EVENT = 'orbitx-editor-tab-drag'

  const dispatchTerminalTabDragEvent = (payload: DragPayload) => {
    window.dispatchEvent(new CustomEvent<DragPayload>(ORBITX_EDITOR_TAB_DRAG_EVENT, { detail: payload }))
  }

  const DRAG_START_THRESHOLD = 6

  let dragPreviewEl: HTMLDivElement | null = null

  const updateDragPreview = (x: number, y: number) => {
    if (!dragPreviewEl) return
    dragPreviewEl.style.transform = `translate(${x + 12}px, ${y + 12}px)`
  }

  const removeDragPreview = () => {
    if (!dragPreviewEl) return
    dragPreviewEl.remove()
    dragPreviewEl = null
  }

  const createDragPreview = (tabId: string, x: number, y: number) => {
    removeDragPreview()
    const el = document.createElement('div')
    el.className = 'orbitx-drag-preview'

    const escapedTabId = tabId.replace(/"/g, '\\"')
    const original = tabStripRef.value?.querySelector(`[data-tab-id="${escapedTabId}"]`)
    if (original) {
      el.appendChild(original.cloneNode(true))
    } else {
      const tab = props.tabs.find(t => t.id === tabId)
      if (!tab) return
      el.textContent = getPresentation(tab).title
    }

    document.body.appendChild(el)
    dragPreviewEl = el
    updateDragPreview(x, y)
  }

  const dragState = ref<{
    tabId: string | null
    isDragging: boolean
    startX: number
    startY: number
    lastX: number
    lastY: number
  }>({
    tabId: null,
    isDragging: false,
    startX: 0,
    startY: 0,
    lastX: 0,
    lastY: 0,
  })

  const resetDragState = () => {
    dragState.value = {
      tabId: null,
      isDragging: false,
      startX: 0,
      startY: 0,
      lastX: 0,
      lastY: 0,
    }
    document.body.classList.remove('orbitx-tab-dragging')
    removeDragPreview()
  }

  const handlePointerMove = (event: PointerEvent) => {
    const tabId = dragState.value.tabId
    if (!tabId) return
    event.preventDefault()

    dragState.value.lastX = event.clientX
    dragState.value.lastY = event.clientY

    const dx = event.clientX - dragState.value.startX
    const dy = event.clientY - dragState.value.startY
    const distance = Math.sqrt(dx * dx + dy * dy)

    if (!dragState.value.isDragging && distance >= DRAG_START_THRESHOLD) {
      dragState.value.isDragging = true
      document.body.classList.add('orbitx-tab-dragging')
      createDragPreview(tabId, event.clientX, event.clientY)
      dispatchTerminalTabDragEvent({
        phase: 'start',
        tabId,
        sourceGroupId: props.groupId,
        x: event.clientX,
        y: event.clientY,
      })
    }

    if (!dragState.value.isDragging) return

    updateDragPreview(event.clientX, event.clientY)
    dispatchTerminalTabDragEvent({
      phase: 'move',
      tabId,
      sourceGroupId: props.groupId,
      x: event.clientX,
      y: event.clientY,
    })
  }

  const handlePointerUp = (event: PointerEvent) => {
    const tabId = dragState.value.tabId
    const wasDragging = dragState.value.isDragging

    window.removeEventListener('pointermove', handlePointerMove)
    window.removeEventListener('pointerup', handlePointerUp)

    if (tabId && wasDragging) {
      dispatchTerminalTabDragEvent({
        phase: 'end',
        tabId,
        sourceGroupId: props.groupId,
        x: event.clientX,
        y: event.clientY,
      })
    }

    resetDragState()
  }

  const handleTabPointerDown = (event: PointerEvent, tab: TabState) => {
    if (event.button !== 0) return

    const target = event.target as HTMLElement | null
    if (target?.closest('.close-btn')) return
    event.preventDefault()

    dragState.value.tabId = tab.id
    dragState.value.startX = event.clientX
    dragState.value.startY = event.clientY
    dragState.value.lastX = event.clientX
    dragState.value.lastY = event.clientY
    dragState.value.isDragging = false

    window.addEventListener('pointermove', handlePointerMove)
    window.addEventListener('pointerup', handlePointerUp)
  }

  // 添加菜单项：仅显示可用 shell 名称
  const addMenuItems = computed(() => {
    const shells = terminalStore.shellManager.availableShells || []
    return shells.map(s => ({
      label: s.name,
      value: s.name,
      onClick: () => handleAddMenuClick(s.name),
    }))
  })

  // 处理右键菜单
  const handleTabContextMenu = async (event: MouseEvent, tabId: TabId) => {
    event.preventDefault()

    const tab = props.tabs.find(t => t.id === tabId)
    if (!tab) return

    const currentIndex = props.tabs.findIndex(tab => tab.id === tabId)
    const hasLeftTabs = currentIndex > 0 && props.tabs.slice(0, currentIndex).some(t => canShowCloseButton(t))
    const hasRightTabs =
      currentIndex < props.tabs.length - 1 && props.tabs.slice(currentIndex + 1).some(t => canShowCloseButton(t))
    const hasOtherTabs = props.tabs.filter(t => t.id !== tabId && canShowCloseButton(t)).length > 0

    const menuItems = []

    if (isTerminalTab(tab)) {
      const cwd = getTerminalCwd(tab.context.paneId)
      menuItems.push({
        label: '复制',
        disabled: !cwd,
        onClick: async () => {
          if (!cwd) return
          try {
            await editorStore.createTerminalTab({ directory: cwd, activate: true })
          } catch (error) {
            // 静默处理
          }
        },
      })
    }

    // 只有可关闭的标签才显示关闭选项
    if (canShowCloseButton(tab)) {
      menuItems.push({
        label: '关闭当前',
        onClick: () => editorStore.closeTab(props.groupId, tabId),
      })
    }

    // 批量关闭选项（只有存在可关闭的标签时才显示）
    if (hasLeftTabs) {
      menuItems.push({
        label: '关闭左侧全部',
        onClick: () => editorStore.closeLeftTabs(props.groupId, tabId),
      })
    }

    if (hasRightTabs) {
      menuItems.push({
        label: '关闭右侧全部',
        onClick: () => editorStore.closeRightTabs(props.groupId, tabId),
      })
    }

    if (hasOtherTabs) {
      menuItems.push({
        label: '关闭其他',
        onClick: () => editorStore.closeOtherTabs(props.groupId, tabId),
      })
    }

    // 如果没有任何菜单项，至少显示一个提示
    if (menuItems.length === 0) {
      menuItems.push({
        label: '无可用操作',
        disabled: true,
        onClick: () => {},
      })
    }

    await showPopoverAt(event.clientX, event.clientY, menuItems)
  }

  // 左键点击：直接新建默认终端
  const handleAddClick = async () => {
    try {
      await editorStore.createTerminalTab({ activate: true })
    } catch (error) {
      // 静默处理
    }
  }

  // 右键点击：打开选择菜单
  const handleAddContextMenu = async (event: MouseEvent) => {
    event.preventDefault()
    await showPopoverAt(event.clientX, event.clientY, addMenuItems.value)
  }

  // 处理添加菜单项点击
  const handleAddMenuClick = async (shellName: string) => {
    try {
      await editorStore.createTerminalTabWithShell({ shellName, activate: true })
    } catch (error) {
      // 静默处理错误
    }
  }

  // 处理中键关闭
  const handleMouseDown = (event: MouseEvent, id: TabId) => {
    if (event.button !== 1) return
    const tab = props.tabs.find(t => t.id === id)
    if (!tab || !canShowCloseButton(tab)) return
    event.preventDefault()
    editorStore.closeTab(props.groupId, id)
  }
</script>

<template>
  <div class="tab-bar-wrapper">
    <div ref="tabStripRef" class="tab-strip" :class="{ scrollable: needsScroll }">
      <div
        v-for="tab in tabs"
        :key="tab.id"
        :class="getTabClass(tab)"
        :style="{ width: needsScroll ? `${MIN_TAB_WIDTH}px` : `${tabWidth}px` }"
        :data-tab-id="tab.id"
        @mousedown="handleMouseDown($event, tab.id)"
        @pointerdown="handleTabPointerDown($event, tab)"
        @click="handleTabClick(tab.id)"
        @contextmenu="handleTabContextMenu($event, tab.id)"
      >
        <div class="tab-content" :title="getPresentation(tab).tooltip">
          <div class="tab-info">
            <span
              v-if="getPresentation(tab).badge"
              class="tab-badge"
              :class="`tab-badge--${getPresentation(tab).badge?.variant}`"
            >
              {{ getPresentation(tab).badge?.text }}
            </span>
            <span class="tab-title">{{ getPresentation(tab).title }}</span>
          </div>
        </div>
        <button
          v-if="canShowCloseButton(tab)"
          class="close-btn"
          @click="handleCloseClick($event, tab.id)"
          :title="t('ui.close_tab')"
        >
          <svg
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="3"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      </div>

      <button
        v-if="showInlineAdd"
        class="add-tab-btn inline"
        :title="t('ui.new_terminal_tip')"
        @click="handleAddClick"
        @contextmenu="handleAddContextMenu"
      >
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="3"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <line x1="12" y1="5" x2="12" y2="19"></line>
          <line x1="5" y1="12" x2="19" y2="12"></line>
        </svg>
      </button>
    </div>

    <button
      v-if="!showInlineAdd"
      class="add-tab-btn"
      :title="t('ui.new_terminal_tip')"
      @click="handleAddClick"
      @contextmenu="handleAddContextMenu"
    >
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="3"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <line x1="12" y1="5" x2="12" y2="19"></line>
        <line x1="5" y1="12" x2="19" y2="12"></line>
      </svg>
    </button>
  </div>
</template>

<style scoped>
  .tab-bar-wrapper {
    display: flex;
    align-items: center;
    width: 100%;
    padding-top: calc((var(--titlebar-height) - var(--titlebar-element-height)) / 2);
    padding-bottom: calc((var(--titlebar-height) - var(--titlebar-element-height)) / 2);
    padding-left: var(--spacing-sm);
    padding-right: var(--spacing-sm);
    gap: var(--spacing-xs);
    position: relative;
  }

  .tab-strip {
    display: flex;
    align-items: center;
    flex: 1;
    min-width: 0;
    height: 100%;
    overflow-x: hidden;
    -ms-overflow-style: none;
    scrollbar-width: none;
    gap: var(--spacing-xs);
  }

  .tab-strip.scrollable {
    overflow-x: auto;
  }

  .tab-strip::-webkit-scrollbar {
    display: none;
  }

  .tab {
    gap: 4px;
    position: relative;
    display: flex;
    align-items: center;
    height: var(--titlebar-element-height);
    min-width: 60px;
    max-width: 200px;
    margin: 0;
    padding: 0 12px;
    border-radius: var(--border-radius-md);
    color: var(--text-400);
    cursor: pointer;
    transition: all 0.15s cubic-bezier(0.4, 0, 0.2, 1);
    flex-shrink: 0;
    border: 1px solid transparent;
    background: transparent;
    will-change: background-color, color;
  }

  .tab:hover {
    background-color: var(--bg-300);
    color: var(--text-300);
  }

  .tab.active {
    color: var(--text-100);
    position: relative;
    border: 1px solid transparent;
  }

  .tab.active::before {
    content: '';
    position: absolute;
    bottom: 0;
    left: 50%;
    transform: translateX(-50%);
    width: 40%;
    height: 2px;
    background: var(--color-primary);
    border-radius: 2px 2px 0 0;
    box-shadow: 0 -1px 4px var(--color-primary-alpha);
  }

  .tab {
    background: var(--bg-400);
  }

  .tab:hover {
    background: var(--bg-500);
  }

  .tab.active {
    background: var(--color-primary-alpha);
  }

  .tab.active::before {
    background: var(--color-primary);
    box-shadow: 0 -1px 4px var(--color-primary-alpha);
  }

  .tab-content {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    height: 100%;
  }

  .tab-info {
    display: flex;
    align-items: center;
    flex: 1;
    min-width: 0;
  }

  .tab-badge {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    flex-shrink: 0;
    line-height: 1.2;
    padding-right: 6px;
    position: relative;
  }

  .tab-badge::after {
    content: '';
    position: absolute;
    right: 0;
    top: 50%;
    transform: translateY(-50%);
    height: 10px;
    width: 1px;
    background-color: rgba(255, 255, 255, 0.2);
  }

  .tab-title {
    font-size: 12px;
    font-weight: 400;
    color: var(--text-200);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
    flex: 1;
    padding-left: 6px;
  }

  .tab-badge--shell {
    color: var(--text-200);
  }

  .tab-badge--split {
    font-size: 11px;
    font-weight: 700;
    color: rgba(59, 130, 246, 0.95);
    background: rgba(59, 130, 246, 0.14);
    padding: 1px 5px;
    border-radius: 999px;
    margin-right: 6px;
  }

  .tab-badge--diff {
    font-size: 10px;
    font-weight: 700;
    color: #eab308;
    background: rgba(234, 179, 8, 0.15);
    padding: 1px 4px;
    border-radius: 3px;
    margin-right: 6px;
  }

  .close-btn {
    display: none;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    padding: 0;
    border: none;
    background: none;
    border-radius: var(--border-radius-sm);
    transition: all 0.2s ease;
    cursor: pointer;
    flex-shrink: 0;
  }

  .tab:hover .close-btn {
    display: block;
    color: var(--text-500);
  }
  .close-btn:hover {
    color: var(--text-200) !important;
  }
  .add-tab-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: var(--titlebar-element-height);
    border: none;
    background: none;
    color: var(--text-400);
    border-radius: var(--border-radius-sm);
    cursor: pointer;
    transition: all 0.2s ease;
    opacity: 0.8;
    flex-shrink: 0;
  }

  .add-tab-btn.inline {
    flex: 0 0 auto;
  }
</style>

<style>
  body.orbitx-tab-dragging {
    cursor: grabbing;
  }
</style>
