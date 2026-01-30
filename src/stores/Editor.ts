import { defineStore } from 'pinia'
import { computed } from 'vue'
import { useSessionStore } from './session'
import { useTerminalStore } from './Terminal'
import { dockApi } from '@/api'
import { getTabDefinition } from '@/tabs/registry'
import type {
  AgentTerminalTabState,
  DiffTabState,
  GroupId,
  TabGroupState,
  TabId,
  TabState,
  TerminalTabState,
} from '@/types/domain/storage'
import { createGroupId, createTabId } from '@/types/domain/storage'
import { getPathBasename } from '@/utils/path'
import {
  createGroupLeafNode,
  createGroupSplitNode,
  getGroupIdsInLayout,
  removeLeafByGroupId,
  replaceLeafByGroupId,
  updateSplitRatio,
} from '@/utils/editorLayout'
import { fnv1aHash } from '@/utils/hash'

export type DropZone = 'left' | 'right' | 'top' | 'bottom' | 'center'

export const useEditorStore = defineStore('Editor', () => {
  const sessionStore = useSessionStore()
  const terminalStore = useTerminalStore()
  let subscribedToTerminalRuntime = false

  const workspace = computed(() => sessionStore.workspaceState)
  const groups = computed(() => workspace.value.groups)
  const activeGroupId = computed(() => workspace.value.activeGroupId)
  const activeGroup = computed<TabGroupState | null>(() => groups.value[activeGroupId.value] ?? null)
  const activeTabId = computed<TabId | null>(() => activeGroup.value?.activeTabId ?? null)
  const activeTab = computed<TabState | null>(() => {
    const group = activeGroup.value
    if (!group) return null
    const id = group.activeTabId
    if (!id) return null
    return group.tabs.find(t => t.id === id) ?? null
  })

  const updateWorkspace = (next: typeof workspace.value) => {
    sessionStore.updateWorkspaceState(next)
    updateDockMenu()
  }

  const normalizeGroupActive = (group: TabGroupState): TabGroupState => {
    const activeId = group.activeTabId
    return {
      ...group,
      tabs: group.tabs.map(t => ({ ...t, isActive: t.id === activeId })),
    }
  }

  const pickActiveTabId = (currentId: TabId | null, tabs: TabState[]): TabId | null => {
    if (currentId && tabs.some(t => t.id === currentId)) return currentId
    const first = tabs[0]
    return first ? first.id : null
  }

  const setActiveGroup = (groupId: GroupId) => {
    if (!(groupId in workspace.value.groups)) return
    const group = workspace.value.groups[groupId]
    if (group?.activeTabId) {
      void setActiveTab(groupId, group.activeTabId)
      return
    }
    updateWorkspace({ ...workspace.value, activeGroupId: groupId })
  }

  const ensureGroupExists = (groupId: GroupId) => {
    if (groupId in workspace.value.groups) return
    updateWorkspace({
      ...workspace.value,
      groups: {
        ...workspace.value.groups,
        [groupId]: { id: groupId, tabs: [], activeTabId: null },
      },
    })
  }

  const setActiveTab = async (groupId: GroupId, tabId: TabId) => {
    const group = workspace.value.groups[groupId]
    if (!group) return
    const tab = group.tabs.find(t => t.id === tabId)
    if (!tab) return

    const nextGroups: typeof workspace.value.groups = {}
    for (const g of Object.values(workspace.value.groups)) {
      nextGroups[g.id] = {
        ...g,
        activeTabId: g.id === groupId ? tabId : g.activeTabId,
        tabs: g.tabs.map(t => ({ ...t, isActive: g.id === groupId && t.id === tabId })),
      }
    }

    updateWorkspace({
      ...workspace.value,
      groups: nextGroups,
      activeGroupId: groupId,
    })

    await getTabDefinition(tab.type).activate(tab as never, {
      setActiveTabId: () => {},
      setActiveTerminalPane: terminalStore.setActiveTerminal,
      closeTerminalPane: terminalStore.closeTerminal,
    })
  }

  const addTabToGroup = async (groupId: GroupId, tab: TabState, options?: { activate?: boolean }) => {
    ensureGroupExists(groupId)
    const group = workspace.value.groups[groupId]

    const nextTabs = [...group.tabs, tab]
    const shouldActivate = options?.activate !== false
    const activeTabId = shouldActivate ? tab.id : group.activeTabId

    updateWorkspace({
      ...workspace.value,
      groups: {
        ...workspace.value.groups,
        [groupId]: {
          ...group,
          tabs: nextTabs.map(t => ({ ...t, isActive: t.id === activeTabId })),
          activeTabId: activeTabId ?? null,
        },
      },
      activeGroupId: shouldActivate ? groupId : workspace.value.activeGroupId,
    })

    if (shouldActivate) {
      await setActiveTab(groupId, tab.id)
    }
  }

  const removeTabFromGroup = (groupId: GroupId, tabId: TabId) => {
    const group = workspace.value.groups[groupId]
    if (!group) return

    const index = group.tabs.findIndex(t => t.id === tabId)
    if (index === -1) return

    const remaining = group.tabs.filter(t => t.id !== tabId)
    const fallbackIndex = index > 0 ? index - 1 : 0
    const nextActiveId = group.activeTabId === tabId ? (remaining[fallbackIndex]?.id ?? null) : group.activeTabId

    const nextGroup: TabGroupState = {
      ...group,
      tabs: remaining.map(t => ({ ...t, isActive: t.id === nextActiveId })),
      activeTabId: nextActiveId,
    }

    updateWorkspace({
      ...workspace.value,
      groups: {
        ...workspace.value.groups,
        [groupId]: nextGroup,
      },
    })
  }

  /**
   * 清理空的 group 节点
   * 当 group 没有 tab 时，从布局树中移除该 leaf 节点，并删除对应的 group 数据
   * 至少保留一个 group（避免布局树为空）
   */
  const cleanupEmptyGroups = () => {
    const current = workspace.value
    let root = current.root
    let groupsMap: typeof current.groups = { ...current.groups }
    let changed = false

    const groupIdsInLayout = getGroupIdsInLayout(root)
    const canRemove = groupIdsInLayout.length > 1
    if (!canRemove) return

    const emptyGroupIds = groupIdsInLayout.filter(id => (groupsMap[id]?.tabs.length ?? 0) === 0)
    if (emptyGroupIds.length === 0) return

    for (const groupId of emptyGroupIds) {
      const remainingGroupIds = getGroupIdsInLayout(root).filter(id => id !== groupId)
      if (remainingGroupIds.length === 0) break

      const removed = removeLeafByGroupId(root, groupId)
      if (!removed.removed || !removed.node) continue

      root = removed.node
      const nextGroupsMap = { ...groupsMap }
      delete nextGroupsMap[groupId]
      groupsMap = nextGroupsMap
      changed = true
    }

    if (!changed) return

    const nextGroupIds = getGroupIdsInLayout(root)
    const activeGroupStillExists = current.activeGroupId in groupsMap
    const activeGroupId = activeGroupStillExists
      ? current.activeGroupId
      : (nextGroupIds[0] ?? Object.keys(groupsMap)[0]!)

    updateWorkspace({
      ...current,
      root,
      groups: groupsMap,
      activeGroupId,
    })
  }

  const createTerminalTab = async (args?: {
    directory?: string
    activate?: boolean
    groupId?: GroupId
  }): Promise<number> => {
    const groupId = args?.groupId ?? activeGroupId.value
    const paneId = await terminalStore.createTerminalPane(args?.directory)
    const runtime = terminalStore.terminals.find(t => t.id === paneId)

    const tab: TerminalTabState = {
      type: 'terminal',
      id: createTabId('terminal'),
      isActive: false,
      context: { kind: 'terminal', paneId },
      data: {
        cwd: runtime?.cwd,
        shellName: runtime?.shell,
      },
    }

    await addTabToGroup(groupId, tab, { activate: args?.activate })
    return paneId
  }

  const findAgentTerminalTab = (terminalId: string): { groupId: GroupId; tab: AgentTerminalTabState } | null => {
    for (const group of Object.values(workspace.value.groups)) {
      const tab = group.tabs.find(t => t.type === 'agent_terminal' && t.context.terminalId === terminalId) as
        | AgentTerminalTabState
        | undefined
      if (tab) {
        return { groupId: group.id, tab }
      }
    }
    return null
  }

  const openAgentTerminalTab = async (args: {
    terminalId: string
    paneId: number
    command: string
    label?: string
    activate?: boolean
    groupId?: GroupId
  }): Promise<TabId> => {
    const existing = findAgentTerminalTab(args.terminalId)
    if (existing) {
      // Keep the existing tab in sync with the latest terminal metadata (command/label)
      // and ensure the paneId is updated in case the backend recreated the pane.
      if (!terminalStore.terminals.some(t => t.id === args.paneId)) {
        terminalStore.registerRuntimeTerminal({
          id: args.paneId,
          cwd: '~',
          shell: 'agent',
        })
      }

      const current = workspace.value
      const group = current.groups[existing.groupId]
      if (group) {
        const nextTabs = group.tabs.map(t => {
          if (t.id !== existing.tab.id || t.type !== 'agent_terminal') return t
          return {
            ...t,
            context: { ...t.context, paneId: args.paneId },
            data: { ...t.data, command: args.command, label: args.label },
          }
        })

        updateWorkspace({
          ...current,
          groups: {
            ...current.groups,
            [existing.groupId]: {
              ...group,
              tabs: nextTabs,
            },
          },
        })
      }

      if (args.activate !== false) {
        await setActiveTab(existing.groupId, existing.tab.id)
      }
      return existing.tab.id
    }

    // The agent terminal pane may exist in the backend, but not yet be registered in the frontend runtime list.
    // If we try to activate a missing pane, TerminalStore.setActiveTerminal will no-op and the first render may appear blank.
    if (!terminalStore.terminals.some(t => t.id === args.paneId)) {
      terminalStore.registerRuntimeTerminal({
        id: args.paneId,
        cwd: '~',
        shell: 'agent',
      })
    }

    const groupId = args.groupId ?? activeGroupId.value
    const tab: AgentTerminalTabState = {
      type: 'agent_terminal',
      id: createTabId('agent_terminal'),
      isActive: false,
      context: { kind: 'agent_terminal', paneId: args.paneId, terminalId: args.terminalId },
      data: {
        command: args.command,
        label: args.label,
      },
    }

    await addTabToGroup(groupId, tab, { activate: args.activate ?? true })
    return tab.id
  }

  const createTerminalTabWithShell = async (args: {
    shellName: string
    directory?: string
    activate?: boolean
    groupId?: GroupId
  }): Promise<number> => {
    const groupId = args.groupId ?? activeGroupId.value
    const paneId = await terminalStore.createTerminalPane(args.directory, { shellName: args.shellName })
    const runtime = terminalStore.terminals.find(t => t.id === paneId)

    const tab: TerminalTabState = {
      type: 'terminal',
      id: createTabId('terminal'),
      isActive: false,
      context: { kind: 'terminal', paneId },
      data: {
        cwd: runtime?.cwd,
        shellName: runtime?.shell,
      },
    }

    await addTabToGroup(groupId, tab, { activate: args.activate })
    return paneId
  }

  const updateTerminalTabsRuntime = (paneId: number, cwd: string) => {
    const current = workspace.value
    let changed = false

    const nextGroups: typeof current.groups = {}
    for (const group of Object.values(current.groups)) {
      let groupChanged = false
      const nextTabs = group.tabs.map(tab => {
        if (tab.type !== 'terminal') return tab
        if (tab.context.paneId !== paneId) return tab
        if (tab.data.cwd === cwd) return tab
        groupChanged = true
        return {
          ...tab,
          data: { ...tab.data, cwd },
        }
      })

      if (groupChanged) {
        changed = true
        nextGroups[group.id] = normalizeGroupActive({ ...group, tabs: nextTabs })
      } else {
        nextGroups[group.id] = group
      }
    }

    if (!changed) return
    updateWorkspace({ ...current, groups: nextGroups })
  }

  const closeTerminalTabsByPaneId = (paneId: number) => {
    const current = workspace.value
    let changed = false

    const nextGroups: typeof current.groups = {}
    for (const group of Object.values(current.groups)) {
      const remaining = group.tabs.filter(
        tab => !((tab.type === 'terminal' || tab.type === 'agent_terminal') && tab.context.paneId === paneId)
      )
      if (remaining.length === group.tabs.length) {
        nextGroups[group.id] = group
        continue
      }

      changed = true
      const nextActive =
        group.activeTabId && remaining.some(t => t.id === group.activeTabId)
          ? group.activeTabId
          : (remaining[0]?.id ?? null)
      nextGroups[group.id] = normalizeGroupActive({ ...group, tabs: remaining, activeTabId: nextActive })
    }

    if (!changed) return
    updateWorkspace({ ...current, groups: nextGroups })
    cleanupEmptyGroups()
  }

  const closeTab = async (groupId: GroupId, tabId: TabId) => {
    const group = workspace.value.groups[groupId]
    if (!group) return
    const tab = group.tabs.find(t => t.id === tabId)
    if (!tab) return

    const def = getTabDefinition(tab.type)
    if (!def.isClosable(tab as never)) return

    if (def.dispose) {
      await def.dispose(tab as never, {
        setActiveTabId: () => {},
        setActiveTerminalPane: terminalStore.setActiveTerminal,
        closeTerminalPane: terminalStore.closeTerminal,
      })
    }

    removeTabFromGroup(groupId, tabId)
    cleanupEmptyGroups()

    const nextGroup = workspace.value.groups[groupId]
    if (nextGroup?.activeTabId) {
      await setActiveTab(groupId, nextGroup.activeTabId)
    }
  }

  const closeLeftTabs = async (groupId: GroupId, currentTabId: TabId) => {
    const group = workspace.value.groups[groupId]
    if (!group) return
    const currentIndex = group.tabs.findIndex(t => t.id === currentTabId)
    if (currentIndex <= 0) return
    const ids = group.tabs
      .slice(0, currentIndex)
      .filter(t => getTabDefinition(t.type).isClosable(t as never))
      .map(t => t.id)
    for (const id of ids) {
      await closeTab(groupId, id)
    }
  }

  const closeRightTabs = async (groupId: GroupId, currentTabId: TabId) => {
    const group = workspace.value.groups[groupId]
    if (!group) return
    const currentIndex = group.tabs.findIndex(t => t.id === currentTabId)
    if (currentIndex === -1 || currentIndex >= group.tabs.length - 1) return
    const ids = group.tabs
      .slice(currentIndex + 1)
      .filter(t => getTabDefinition(t.type).isClosable(t as never))
      .map(t => t.id)
    for (const id of ids) {
      await closeTab(groupId, id)
    }
  }

  const closeOtherTabs = async (groupId: GroupId, currentTabId: TabId) => {
    const group = workspace.value.groups[groupId]
    if (!group) return
    const ids = group.tabs
      .filter(t => t.id !== currentTabId && getTabDefinition(t.type).isClosable(t as never))
      .map(t => t.id)
    for (const id of ids) {
      await closeTab(groupId, id)
    }
  }

  const closeAllTabs = async () => {
    const groupEntries = Object.values(workspace.value.groups)
    for (const group of groupEntries) {
      const ids = group.tabs.map(t => t.id)
      for (const id of ids) {
        await closeTab(group.id, id)
      }
    }
  }

  const findTabLocation = (tabId: TabId): { groupId: GroupId; tab: TabState } | null => {
    for (const group of Object.values(workspace.value.groups)) {
      const tab = group.tabs.find(t => t.id === tabId)
      if (tab) return { groupId: group.id, tab }
    }
    return null
  }

  const moveTab = async (args: { tabId: TabId; targetGroupId: GroupId; activate?: boolean }) => {
    const source = findTabLocation(args.tabId)
    if (!source) return
    if (source.groupId === args.targetGroupId) return

    removeTabFromGroup(source.groupId, args.tabId)
    cleanupEmptyGroups()

    await addTabToGroup(args.targetGroupId, { ...source.tab, isActive: false }, { activate: args.activate })
    cleanupEmptyGroups()
  }

  const splitGroupWithTab = async (args: {
    tabId: TabId
    targetGroupId: GroupId
    zone: Exclude<DropZone, 'center'>
  }) => {
    const source = findTabLocation(args.tabId)
    if (!source) return

    const targetGroup = workspace.value.groups[args.targetGroupId]
    if (!targetGroup) return

    const newGroupId = createGroupId('group')
    const direction: 'row' | 'column' = args.zone === 'left' || args.zone === 'right' ? 'row' : 'column'
    const placeFirst = args.zone === 'left' || args.zone === 'top'

    const newLeaf = createGroupLeafNode(newGroupId)
    const nextRoot = replaceLeafByGroupId(workspace.value.root, args.targetGroupId, targetLeaf =>
      createGroupSplitNode({
        direction,
        first: placeFirst ? newLeaf : targetLeaf,
        second: placeFirst ? targetLeaf : newLeaf,
      })
    )

    const movedTab = source.tab
    removeTabFromGroup(source.groupId, movedTab.id)

    const nextGroups = {
      ...workspace.value.groups,
      [args.targetGroupId]: workspace.value.groups[args.targetGroupId],
      [newGroupId]: {
        id: newGroupId,
        tabs: [],
        activeTabId: null,
      },
    }

    updateWorkspace({
      ...workspace.value,
      root: nextRoot,
      groups: nextGroups,
      activeGroupId: newGroupId,
    })

    await addTabToGroup(newGroupId, { ...movedTab, isActive: false }, { activate: true })
    cleanupEmptyGroups()
  }

  const updateSplitRatioById = (splitId: string, ratio: number) => {
    updateWorkspace({
      ...workspace.value,
      root: updateSplitRatio(workspace.value.root, splitId, ratio),
    })
  }

  const openDiffTab = async (args: { repoPath: string; data: DiffTabState['data'] }): Promise<TabId> => {
    const activeId = activeGroupId.value
    const existing = Object.values(workspace.value.groups)
      .flatMap(g => g.tabs)
      .find(
        (t): t is DiffTabState =>
          t.type === 'diff' &&
          t.context.repoPath === args.repoPath &&
          t.data.filePath === args.data.filePath &&
          (t.data.staged ?? false) === (args.data.staged ?? false) &&
          t.data.commitHash === args.data.commitHash
      )

    if (existing) {
      const loc = findTabLocation(existing.id)
      if (loc) await setActiveTab(loc.groupId, existing.id)
      return existing.id
    }

    const key = JSON.stringify({
      repoPath: args.repoPath,
      filePath: args.data.filePath,
      staged: args.data.staged ?? false,
      commitHash: args.data.commitHash ?? null,
    })
    const id = `diff:${fnv1aHash(key)}`

    const newTab: DiffTabState = {
      type: 'diff',
      id,
      isActive: false,
      context: { kind: 'git', repoPath: args.repoPath },
      data: args.data,
    }

    await addTabToGroup(activeId, newTab, { activate: true })
    return id
  }

  const createSettingsTab = async (): Promise<TabId> => {
    const existing = Object.values(workspace.value.groups)
      .flatMap(g => g.tabs)
      .find(t => t.type === 'settings')

    if (existing) {
      const loc = findTabLocation(existing.id)
      if (loc) await setActiveTab(loc.groupId, existing.id)
      return existing.id
    }

    const id: TabId = 'settings'
    const tab: TabState = {
      type: 'settings',
      id,
      isActive: false,
      context: { kind: 'none' },
      data: { lastSection: 'general' },
    }

    await addTabToGroup(activeGroupId.value, tab, { activate: true })
    return id
  }

  const updateSettingsTabSection = (tabId: TabId, section: string) => {
    const loc = findTabLocation(tabId)
    if (!loc) return
    const group = workspace.value.groups[loc.groupId]
    if (!group) return

    const nextTabs = group.tabs.map(tab => {
      if (tab.type === 'settings' && tab.id === tabId) {
        return {
          ...tab,
          data: {
            ...tab.data,
            lastSection: section,
          },
        }
      }
      return tab
    })

    updateWorkspace({
      ...workspace.value,
      groups: {
        ...workspace.value.groups,
        [loc.groupId]: {
          ...group,
          tabs: nextTabs,
        },
      },
    })
  }

  const getSettingsTabSection = (tabId: TabId): string | undefined => {
    const loc = findTabLocation(tabId)
    if (!loc || loc.tab.type !== 'settings') return undefined
    return loc.tab.data.lastSection
  }

  const updateDockMenu = () => {
    const tabs = activeGroup.value?.tabs ?? []
    const entries = tabs
      .filter(t => t.type === 'terminal')
      .map(t => {
        const terminal = terminalStore.terminals.find(x => x.id === t.context.paneId)
        return {
          id: t.id,
          title: getPathBasename(terminal?.cwd ?? ''),
        }
      })
    dockApi.updateTabs(entries, activeGroup.value?.activeTabId ?? null)
  }

  const initialize = async () => {
    if (!sessionStore.initialized) {
      await sessionStore.initialize()
    }

    if (!subscribedToTerminalRuntime) {
      terminalStore.subscribeToCwdChanged((paneId, cwd) => {
        updateTerminalTabsRuntime(paneId, cwd)
      })
      terminalStore.subscribeToTerminalExit(paneId => {
        closeTerminalTabsByPaneId(paneId)
      })
      subscribedToTerminalRuntime = true
    }

    await reconcileTerminalTabs()
    cleanupEmptyGroups()
    updateDockMenu()

    const group = workspace.value.groups[workspace.value.activeGroupId]
    if (group?.activeTabId) {
      await setActiveTab(group.id, group.activeTabId)
    }
  }

  const reconcileTerminalTabs = async () => {
    await terminalStore.refreshRuntimeTerminals()

    const runtimeMap = new Map(terminalStore.terminals.filter(t => t.shell !== 'agent').map(r => [r.id, r]))
    const current = workspace.value
    const nextGroups: typeof current.groups = {}
    let changed = false

    for (const group of Object.values(current.groups)) {
      const nextTabs: TabState[] = []
      let groupChanged = false

      for (const tab of group.tabs) {
        if (tab.type !== 'terminal') {
          nextTabs.push(tab)
          continue
        }

        const runtime = runtimeMap.get(tab.context.paneId)
        if (runtime) {
          runtimeMap.delete(runtime.id)

          const nextTab =
            tab.data.cwd === runtime.cwd && tab.data.shellName === runtime.shell
              ? tab
              : {
                  ...tab,
                  data: { ...tab.data, cwd: runtime.cwd, shellName: runtime.shell },
                }

          if (nextTab !== tab) groupChanged = true
          nextTabs.push(nextTab)
          continue
        }

        const createdPaneId = await terminalStore.createTerminalPane(tab.data.cwd)
        const created = terminalStore.terminals.find(t => t.id === createdPaneId)
        groupChanged = true
        nextTabs.push({
          ...tab,
          context: { kind: 'terminal', paneId: createdPaneId },
          data: {
            ...tab.data,
            cwd: created?.cwd,
            shellName: created?.shell,
          },
        })
      }

      if (groupChanged) {
        changed = true
        const nextActive = pickActiveTabId(group.activeTabId, nextTabs)
        nextGroups[group.id] = normalizeGroupActive({ ...group, tabs: nextTabs, activeTabId: nextActive })
      } else {
        nextGroups[group.id] = group
      }
    }

    if (runtimeMap.size > 0) {
      const targetGroupId = current.activeGroupId
      const target = nextGroups[targetGroupId]
      if (target) {
        const orphanTabs: TerminalTabState[] = Array.from(runtimeMap.values()).map(runtime => ({
          type: 'terminal',
          id: createTabId('terminal'),
          isActive: false,
          context: { kind: 'terminal', paneId: runtime.id },
          data: { cwd: runtime.cwd, shellName: runtime.shell },
        }))

        const tabs = [...target.tabs, ...orphanTabs]
        const activeTabId = pickActiveTabId(target.activeTabId, tabs)
        nextGroups[targetGroupId] = normalizeGroupActive({ ...target, tabs, activeTabId })
        changed = true
      }
    }

    if (!changed) return
    updateWorkspace({ ...current, groups: nextGroups })
  }

  return {
    workspace,
    groups,
    activeGroupId,
    activeGroup,
    activeTabId,
    activeTab,
    setActiveGroup,
    setActiveTab,
    addTabToGroup,
    moveTab,
    splitGroupWithTab,
    updateSplitRatio: updateSplitRatioById,
    closeTab,
    closeLeftTabs,
    closeRightTabs,
    closeOtherTabs,
    closeAllTabs,
    createTerminalTab,
    createTerminalTabWithShell,
    openAgentTerminalTab,
    openDiffTab,
    createSettingsTab,
    updateSettingsTabSection,
    getSettingsTabSection,
    initialize,
  }
})
