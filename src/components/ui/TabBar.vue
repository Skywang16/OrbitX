<script setup lang="ts">
  import { computed, ref } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useTerminalStore } from '@/stores/Terminal'
  import { useTabManagerStore } from '@/stores/TabManager'
  import { TabType, type TabItem } from '@/types'
  import { showPopoverAt } from '@/ui/composables/popover-api'

  const { t } = useI18n()

  interface Props {
    tabs: TabItem[]
    activeTabId: number | string | null
  }

  interface Emits {
    (e: 'switch', id: number | string): void
    (e: 'close', id: number | string): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()

  // 使用store
  const terminalStore = useTerminalStore()
  const tabManagerStore = useTabManagerStore()

  // 获取标签样式类
  const getTabClass = (tab: TabItem): string[] => {
    const classes = ['tab']

    // 使用activeTabId作为激活状态的唯一判断依据，确保状态一致性
    if (tab.id === props.activeTabId) {
      classes.push('active')
    }

    if (tab.type === TabType.TERMINAL && tab.title === 'OrbitX') {
      classes.push('agent-tab')
    }

    return classes
  }

  /**
   * 从 TerminalStore 获取终端的 shell 名称
   * 数据单一来源：TerminalStore.terminals
   */
  const getTerminalShell = (tabId: number | string): string => {
    const terminal = terminalStore.terminals.find(t => t.id === tabId)
    return terminal?.shell || 'shell'
  }

  /**
   * 从 TerminalStore 获取终端的显示路径
   * 数据单一来源：TerminalStore.terminals + tabManagerStore.getDisplayPath
   */
  const getTerminalPath = (tabId: number | string): string => {
    const terminal = terminalStore.terminals.find(t => t.id === tabId)
    if (!terminal) return '~'
    return tabManagerStore.getDisplayPath(terminal.cwd)
  }

  // 获取标签提示信息
  const getTabTooltip = (tab: TabItem): string => {
    if (tab.type === TabType.TERMINAL) {
      // 从 TerminalStore 获取数据（单一数据源）
      const terminal = terminalStore.terminals.find(t => t.id === tab.id)
      const fullPath = terminal?.cwd || '~'
      const shell = terminal?.shell || 'shell'

      return `${shell} • ${fullPath}`
    }

    return tab.title || 'Tab'
  }

  const tabBarRef = ref<HTMLDivElement | null>(null)
  const tabBarWrapperRef = ref<HTMLDivElement | null>(null)

  // 简化的标签宽度配置
  const MIN_TAB_WIDTH = 60
  const MAX_TAB_WIDTH = 150

  // 简化的标签宽度计算
  const tabWidth = computed(() => {
    const tabCount = props.tabs.length
    if (tabCount === 0) return MAX_TAB_WIDTH

    const containerWidth = tabBarWrapperRef.value?.clientWidth || 400

    const paddingAndGaps = 6 + 4 + 34 + 6 * tabCount
    const availableWidth = containerWidth - paddingAndGaps
    const widthPerTab = availableWidth / tabCount

    return Math.max(MIN_TAB_WIDTH, Math.min(MAX_TAB_WIDTH, widthPerTab))
  })

  // 简化的滚动判断
  const needsScroll = computed(() => tabWidth.value <= MIN_TAB_WIDTH)

  // 判断标签页是否可以显示关闭按钮
  const canShowCloseButton = (tab: TabItem): boolean => {
    // 只要标签页是可关闭的，就显示关闭按钮
    return tab.closable ?? false
  }

  // 处理标签点击
  const handleTabClick = (id: number | string) => {
    if (id !== props.activeTabId) {
      emit('switch', id)
    }
  }

  // 处理关闭按钮点击
  const handleCloseClick = (event: MouseEvent, id: number | string) => {
    event.stopPropagation()
    emit('close', id)
  }

  // 获取 tab 标题的显示文本
  const getTabTitle = (tab: TabItem): string => {
    if (tab.type === TabType.SETTINGS && tab.title === 'settings') {
      return t('settings.title')
    }
    return tab.title || 'Tab'
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
  const handleTabContextMenu = async (event: MouseEvent, tabId: number | string) => {
    event.preventDefault()

    const tab = props.tabs.find(t => t.id === tabId)
    if (!tab) return

    const currentIndex = props.tabs.findIndex(tab => tab.id === tabId)
    const hasLeftTabs = currentIndex > 0 && props.tabs.slice(0, currentIndex).some(t => t.closable)
    const hasRightTabs =
      currentIndex < props.tabs.length - 1 && props.tabs.slice(currentIndex + 1).some(t => t.closable)
    const hasOtherTabs = props.tabs.filter(t => t.id !== tabId && t.closable).length > 0

    const menuItems = []

    // 只有可关闭的标签才显示关闭选项
    if (tab.closable) {
      menuItems.push({
        label: '关闭当前',
        onClick: () => emit('close', tabId),
      })
    }

    // 批量关闭选项（只有存在可关闭的标签时才显示）
    if (hasLeftTabs) {
      menuItems.push({
        label: '关闭左侧全部',
        onClick: () => tabManagerStore.closeLeftTabs(tabId),
      })
    }

    if (hasRightTabs) {
      menuItems.push({
        label: '关闭右侧全部',
        onClick: () => tabManagerStore.closeRightTabs(tabId),
      })
    }

    if (hasOtherTabs) {
      menuItems.push({
        label: '关闭其他',
        onClick: () => tabManagerStore.closeOtherTabs(tabId),
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
      await terminalStore.createTerminal()
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
      await terminalStore.createTerminalWithShell(shellName)
    } catch (error) {
      // 静默处理错误
    }
  }

  // 处理鼠标按下事件（中键关闭）
  const handleMouseDown = (event: MouseEvent, id: number | string) => {
    if (event.button === 1) {
      const tab = props.tabs.find(t => t.id === id)
      if (tab && canShowCloseButton(tab)) {
        event.preventDefault()
        emit('close', id)
      }
    }
  }
</script>

<template>
  <div ref="tabBarWrapperRef" class="tab-bar-wrapper">
    <div ref="tabBarRef" class="tab-bar" :class="{ scrollable: needsScroll }">
      <div
        v-for="tab in tabs"
        :key="tab.id"
        :class="getTabClass(tab)"
        :style="{ width: needsScroll ? `${MIN_TAB_WIDTH}px` : `${tabWidth}px` }"
        @mousedown="handleMouseDown($event, tab.id)"
        @click="handleTabClick(tab.id)"
        @contextmenu="handleTabContextMenu($event, tab.id)"
      >
        <div class="tab-content" :title="getTabTooltip(tab)">
          <template v-if="tab.type === TabType.TERMINAL">
            <div class="terminal-info">
              <span class="shell-badge">{{ getTerminalShell(tab.id) }}</span>
              <span class="path-info">{{ getTerminalPath(tab.id) }}</span>
            </div>
          </template>
          <template v-else>
            <span class="tab-title">{{ getTabTitle(tab) }}</span>
          </template>
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
        v-if="!needsScroll && tabWidth >= MAX_TAB_WIDTH"
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
      v-if="needsScroll || tabWidth < MAX_TAB_WIDTH"
      class="add-tab-btn fixed"
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
    height: 100%;
    padding-left: var(--spacing-sm);
    gap: var(--spacing-xs);
    position: relative;
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
    margin: 0 2px 0 0;
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

  .tab:not(.agent-tab) {
    background: var(--bg-400);
  }

  .tab:not(.agent-tab):hover {
    background: var(--bg-500);
  }

  .tab:not(.agent-tab).active {
    background: var(--color-primary-alpha);
  }

  .tab:not(.agent-tab).active::before {
    background: var(--color-primary);
    box-shadow: 0 -1px 4px var(--color-primary-alpha);
  }

  .tab.agent-tab {
    background: rgba(117, 190, 255, 0.1);
    position: relative;
  }

  .tab.agent-tab:hover {
    background: rgba(117, 190, 255, 0.15);
  }

  .tab.agent-tab.active {
    background: rgba(117, 190, 255, 0.15);
    border-color: transparent;
  }

  .tab.agent-tab.active::before {
    background: var(--color-info);
    box-shadow: 0 -1px 4px var(--color-primary-alpha);
  }

  .tab.agent-tab::after {
    content: '';
    position: absolute;
    top: 2px;
    right: 2px;
    width: 6px;
    height: 6px;
    background: var(--color-info);
    border-radius: 50%;
    box-shadow: 0 0 6px var(--color-primary-alpha);
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

  .tab-content {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    height: 100%;
  }

  .tab-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-200);
  }

  .terminal-info {
    display: flex;
    align-items: center;
    flex: 1;
    min-width: 0;
  }

  .shell-badge {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-200);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    flex-shrink: 0;
    line-height: 1.2;
    padding-right: 6px;
    position: relative;
  }

  .shell-badge::after {
    content: '';
    position: absolute;
    right: 0;
    top: 50%;
    transform: translateY(-50%);
    height: 10px;
    width: 1px;
    background-color: rgba(255, 255, 255, 0.2);
  }

  .path-info {
    font-size: 12px;
    font-weight: 400;
    color: var(--text-400);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
    flex: 1;
    padding-left: 6px;
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
    margin-left: var(--spacing-xs);
  }

  .add-tab-btn.fixed {
    margin-right: var(--spacing-sm);
  }
</style>
