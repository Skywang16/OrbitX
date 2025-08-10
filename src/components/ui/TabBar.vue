<script setup lang="ts">
  import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
  import { useTerminalStore } from '@/stores/Terminal'
  import { TabType, type TabItem } from '@/types'

  interface Props {
    tabs: TabItem[]
    activeTabId: string | null
  }

  interface Emits {
    (e: 'switch', id: string): void
    (e: 'close', id: string): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()

  // 使用store
  const terminalStore = useTerminalStore()
  // 移除未使用的 tabManagerStore 引用

  // 弹出菜单状态
  const showAddMenuPopover = ref(false)

  // 添加菜单项：仅显示可用 shell 名称
  const addMenuItems = computed(() => {
    const shells = terminalStore.shellManager.availableShells || []
    return shells.map(s => ({ label: s.name, value: s.name }))
  })

  // 获取标签样式类
  const getTabClass = (tab: TabItem): string[] => {
    const classes = ['tab']

    if (tab.isActive) {
      classes.push('active')
    }

    if (tab.type === TabType.TERMINAL && tab.title === 'OrbitX') {
      classes.push('agent-tab')
    }

    return classes
  }

  const tabBarRef = ref<HTMLDivElement | null>(null)
  const tabBarWrapperRef = ref<HTMLDivElement | null>(null)

  // 标签宽度配置
  const TAB_CONFIG = {
    minWidth: 60, // 最小宽度
    maxWidth: 200, // 最大宽度
    addBtnWidth: 32, // 添加按钮宽度
    margin: 6, // 标签右边距
    padding: 12, // 容器内边距
  }

  // 计算动态标签宽度
  const dynamicTabWidth = computed(() => {
    const tabCount = props.tabs.length
    if (tabCount === 0) return TAB_CONFIG.maxWidth

    const containerWidth = tabBarWrapperRef.value?.clientWidth || 400
    let availableWidth = containerWidth - TAB_CONFIG.padding * 2

    const totalMarginWidth = (tabCount - 1) * TAB_CONFIG.margin
    const inlineButtonWidth = TAB_CONFIG.addBtnWidth + 4
    const widthForTabs = availableWidth - totalMarginWidth - inlineButtonWidth
    const widthPerTab = widthForTabs / tabCount

    if (widthPerTab < TAB_CONFIG.maxWidth) {
      availableWidth = containerWidth - TAB_CONFIG.addBtnWidth - TAB_CONFIG.padding * 2 - 4
      const widthForTabsFixed = availableWidth - totalMarginWidth
      const widthPerTabFixed = widthForTabsFixed / tabCount
      return Math.max(TAB_CONFIG.minWidth, widthPerTabFixed)
    }

    return Math.max(TAB_CONFIG.minWidth, Math.min(TAB_CONFIG.maxWidth, widthPerTab))
  })

  // 判断是否需要滚动
  const needsScroll = computed(() => {
    return dynamicTabWidth.value <= TAB_CONFIG.minWidth
  })

  // 判断标签是否被压缩
  const isCompressed = computed(() => {
    return dynamicTabWidth.value < TAB_CONFIG.maxWidth && !needsScroll.value
  })

  // 处理标签点击
  const handleTabClick = (id: string) => {
    if (id !== props.activeTabId) {
      emit('switch', id)
    }
  }

  // 处理关闭按钮点击
  const handleCloseClick = (event: MouseEvent, id: string) => {
    event.stopPropagation()
    emit('close', id)
  }

  // 左键点击：直接新建默认终端
  const handleAddClick = async () => {
    try {
      await terminalStore.createTerminal()
    } catch (error) {
      // 静默处理
    }
  }

  // 右键点击：打开选择菜单
  const handleAddContextMenu = (event: MouseEvent) => {
    event.preventDefault()
    showAddMenuPopover.value = true
  }

  // 处理添加菜单项点击
  const handleAddMenuClick = async (item: { label: string; value: string }) => {
    showAddMenuPopover.value = false

    try {
      // value 直接为 shell 名称
      await terminalStore.createTerminalWithShell(item.value)
    } catch (error) {
      // 静默处理错误
    }
  }

  // 处理鼠标按下事件（中键关闭）
  const handleMouseDown = (event: MouseEvent, id: string) => {
    if (event.button === 1 && props.tabs.length > 1) {
      event.preventDefault()
      emit('close', id)
    }
  }

  // 处理鼠标滚轮事件（水平滚动）
  const handleWheel = (event: WheelEvent) => {
    const el = tabBarRef.value
    if (!el) return

    // 如果用户按住shift键，让浏览器处理原生水平滚动
    if (event.shiftKey) return

    // 阻止默认垂直滚动，改为水平滚动
    if (el.scrollWidth > el.clientWidth) {
      event.preventDefault()
      el.scrollLeft += event.deltaY
    }
  }

  // 处理窗口大小变化
  const handleResize = () => {
    // 触发重新计算，Vue的响应式系统会自动处理
  }

  onMounted(() => {
    tabBarRef.value?.addEventListener('wheel', handleWheel, { passive: false })
    window.addEventListener('resize', handleResize)
  })

  onBeforeUnmount(() => {
    tabBarRef.value?.removeEventListener('wheel', handleWheel)
    window.removeEventListener('resize', handleResize)
  })
</script>

<template>
  <div ref="tabBarWrapperRef" class="tab-bar-wrapper">
    <div ref="tabBarRef" class="tab-bar" :class="{ scrollable: needsScroll }">
      <div
        v-for="tab in tabs"
        :key="tab.id"
        :class="getTabClass(tab)"
        :style="{ width: needsScroll ? `${TAB_CONFIG.minWidth}px` : `${dynamicTabWidth}px` }"
        @mousedown="handleMouseDown($event, tab.id)"
        @click="handleTabClick(tab.id)"
      >
        <span class="tab-title" :title="tab.title">{{ tab.title }}</span>
        <button
          v-if="tab.closable && tabs.length > 1"
          class="close-btn"
          @click="handleCloseClick($event, tab.id)"
          title="关闭标签"
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

      <!-- 内联添加按钮 -->
      <x-popover
        v-if="!needsScroll && !isCompressed"
        v-model="showAddMenuPopover"
        placement="bottom-start"
        trigger="manual"
        :menu-items="addMenuItems"
        @menu-item-click="handleAddMenuClick"
      >
        <template #trigger>
          <button
            class="add-tab-btn inline"
            title="新建终端（左键） / 选择Shell（右键）"
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
        </template>
      </x-popover>
    </div>

    <!-- 固定添加按钮 -->
    <x-popover
      v-if="needsScroll || isCompressed"
      v-model="showAddMenuPopover"
      placement="bottom-end"
      trigger="manual"
      :menu-items="addMenuItems"
      @menu-item-click="handleAddMenuClick"
    >
      <template #trigger>
        <button
          class="add-tab-btn fixed"
          title="新建终端（左键） / 选择Shell（右键）"
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
      </template>
    </x-popover>
  </div>
</template>

<style scoped>
  .tab-bar-wrapper {
    display: flex;
    align-items: center;
    width: 100%;
    height: 100%;
    padding-left: var(--spacing-sm);
    gap: var(--spacing-xs);
  }

  .tab-bar {
    display: flex;
    align-items: center;
    flex: 1;
    min-width: 0;
    height: 100%;
    overflow-x: hidden;
    -ms-overflow-style: none;
    scrollbar-width: none;
  }

  .tab-bar.scrollable {
    overflow-x: auto;
  }

  .tab-bar::-webkit-scrollbar {
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
    margin: 0 6px 0 0;
    padding: 0 8px;
    border-radius: var(--border-radius-md);
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    flex-shrink: 0;
    border: 1px solid transparent;
  }

  .tab:hover {
    background-color: var(--color-hover);
    color: var(--text-secondary);
  }

  .tab.active {
    background-color: var(--color-background);
    color: var(--text-primary);
    border-color: var(--color-border);
    box-shadow: none; /* 仅移除阴影，其他保持不变 */
  }

  .tab.active::before {
    content: '';
    position: absolute;
    bottom: 0;
    left: 50%;
    transform: translateX(-50%);
    width: 40%;
    height: 2px;
    background: linear-gradient(90deg, var(--color-primary), #4fc3f7);
    border-radius: 2px 2px 0 0;
    box-shadow: 0 -1px 4px rgba(0, 122, 204, 0.3);
  }

  /* 终端Tab样式 */
  .tab:not(.agent-tab) {
    background: linear-gradient(135deg, rgba(59, 130, 246, 0.1), rgba(37, 99, 235, 0.08));
    border: 1px solid rgba(59, 130, 246, 0.2);
  }

  .tab:not(.agent-tab):hover {
    background: linear-gradient(135deg, rgba(59, 130, 246, 0.15), rgba(37, 99, 235, 0.12));
    border-color: rgba(59, 130, 246, 0.3);
  }

  .tab:not(.agent-tab).active {
    background: linear-gradient(135deg, rgba(59, 130, 246, 0.08), rgba(37, 99, 235, 0.06));
    border-color: rgba(59, 130, 246, 0.4);
    box-shadow: none; /* 移除阴影 */
  }

  .tab:not(.agent-tab).active::before {
    background: linear-gradient(90deg, #3b82f6, #2563eb, #1d4ed8);
    box-shadow: 0 -1px 4px rgba(59, 130, 246, 0.4);
  }

  /* Agent专属终端Tab样式 */
  .tab.agent-tab {
    background: linear-gradient(135deg, rgba(124, 58, 237, 0.1), rgba(99, 102, 241, 0.08));
    border: 1px solid rgba(124, 58, 237, 0.2);
    position: relative;
  }

  .tab.agent-tab:hover {
    background: linear-gradient(135deg, rgba(124, 58, 237, 0.15), rgba(99, 102, 241, 0.12));
    border-color: rgba(124, 58, 237, 0.3);
  }

  .tab.agent-tab.active {
    background: linear-gradient(135deg, rgba(124, 58, 237, 0.08), rgba(99, 102, 241, 0.06));
    border-color: rgba(124, 58, 237, 0.4);
  }

  .tab.agent-tab.active::before {
    background: linear-gradient(90deg, #7c3aed, #6366f1, #3b82f6);
    box-shadow: 0 -1px 4px rgba(124, 58, 237, 0.4);
  }

  /* Agent终端Tab的特殊标识 */
  .tab.agent-tab::after {
    content: '';
    position: absolute;
    top: 2px;
    right: 2px;
    width: 6px;
    height: 6px;
    background: linear-gradient(45deg, #7c3aed, #6366f1);
    border-radius: 50%;
    box-shadow: 0 0 6px rgba(124, 58, 237, 0.6);
    animation: pulse-glow 2s infinite;
  }

  @keyframes pulse-glow {
    0%,
    100% {
      opacity: 0.8;
      transform: scale(1);
    }
    50% {
      opacity: 1;
      transform: scale(1.1);
    }
  }

  .tab-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--font-size-sm);
    font-weight: 500;
    flex: 1;
    text-align: left;
    min-width: 0;
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    padding: 0;
    border: none;
    background-color: transparent;
    color: var(--text-muted);
    border-radius: var(--border-radius-sm);
    transition: all 0.2s ease;
    cursor: pointer;
    flex-shrink: 0;
    margin-left: 4px;
    opacity: 0;
  }

  .tab:hover .close-btn {
    opacity: 1;
    color: var(--text-secondary);
  }

  .close-btn:hover {
    background-color: var(--color-hover);
    color: var(--text-primary);
  }

  .add-tab-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: var(--titlebar-element-height);
    border: none;
    background-color: transparent;
    color: var(--text-muted);
    border-radius: var(--border-radius-md);
    cursor: pointer;
    transition: all 0.2s ease;
    opacity: 0.8;
    flex-shrink: 0;
  }

  .add-tab-btn.inline {
    margin-left: var(--spacing-xs);
  }

  .add-tab-btn.fixed {
    margin-right: var(--spacing-sm);
  }

  .add-tab-btn:hover {
    background-color: var(--color-hover);
    color: var(--text-primary);
    opacity: 1;
    transform: scale(1.05);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  }
</style>
