import type { Component } from 'vue'
import Terminal from '@/components/terminal/Terminal.vue'
import DiffView from '@/views/DiffView/DiffView.vue'
import SettingsView from '@/views/Settings/SettingsView.vue'
import type {
  AgentTerminalTabState,
  DiffTabState,
  SettingsTabState,
  TabState,
  TerminalTabState,
} from '@/types/domain/storage'
import { getPathBasename } from '@/utils/path'

export type TabKind = TabState['type']

export interface TabBarBadge {
  text: string
  variant: 'shell' | 'diff'
}

export interface TabBarPresentation {
  tooltip: string
  badge?: TabBarBadge
  title: string
}

export interface TabUiContext {
  t: (key: string) => string
  getTerminalCwd: (paneId: number) => string
}

export interface TabActionContext {
  setActiveTabId: (tabId: string | null) => void
  setActiveTerminalPane: (paneId: number) => Promise<void>
  closeTerminalPane: (paneId: number) => Promise<void>
}

export interface TabDefinition<TTab extends TabState = TabState> {
  kind: TabKind
  component: Component
  getComponentProps: (tab: TTab) => Record<string, unknown>
  getPresentation: (tab: TTab, ctx: TabUiContext) => TabBarPresentation
  isClosable: (tab: TTab) => boolean
  activate: (tab: TTab, ctx: TabActionContext) => Promise<void>
  dispose?: (tab: TTab, ctx: TabActionContext) => Promise<void>
  /**
   * Whether this tab type should affect the workspace context.
   * - true: switching to this tab will change currentWorkspacePath (e.g. terminal tabs)
   * - false: switching to this tab preserves the current workspace (e.g. settings, agent terminals)
   */
  affectsWorkspace: boolean
}

const terminalTab: TabDefinition<TerminalTabState> = {
  kind: 'terminal',
  component: Terminal,
  affectsWorkspace: true,
  getComponentProps: tab => ({
    terminalId: tab.context.paneId,
    isActive: tab.isActive,
  }),
  getPresentation: (tab, ctx) => {
    const cwd = ctx.getTerminalCwd(tab.context.paneId)
    return {
      tooltip: cwd,
      badge: { text: tab.data.shellName ?? 'shell', variant: 'shell' },
      title: getPathBasename(cwd),
    }
  },
  isClosable: () => true,
  activate: async (tab, ctx) => {
    await ctx.setActiveTerminalPane(tab.context.paneId)
    ctx.setActiveTabId(tab.id)
  },
  dispose: async (tab, ctx) => {
    await ctx.closeTerminalPane(tab.context.paneId)
  },
}

const agentTerminalTab: TabDefinition<AgentTerminalTabState> = {
  kind: 'agent_terminal',
  component: Terminal,
  affectsWorkspace: false, // Agent terminals are auxiliary tools, don't change workspace
  getComponentProps: tab => ({
    terminalId: tab.context.paneId,
    isActive: tab.isActive,
  }),
  getPresentation: tab => {
    const title = tab.data.label?.trim() || tab.data.command || 'Agent Terminal'
    return {
      tooltip: tab.data.command,
      badge: { text: 'AGENT', variant: 'shell' },
      title,
    }
  },
  isClosable: () => true,
  activate: async (tab, ctx) => {
    await ctx.setActiveTerminalPane(tab.context.paneId)
    ctx.setActiveTabId(tab.id)
  },
  dispose: async () => {
    // Closing the tab only hides the agent terminal; do not close the pane.
  },
}

const settingsTab: TabDefinition<SettingsTabState> = {
  kind: 'settings',
  component: SettingsView,
  affectsWorkspace: false, // Settings are global, don't change workspace
  getComponentProps: () => ({}),
  getPresentation: (tab, ctx) => {
    void tab
    return {
      tooltip: ctx.t('settings.title'),
      title: ctx.t('settings.title'),
    }
  },
  isClosable: () => true,
  activate: async (tab, ctx) => {
    ctx.setActiveTabId(tab.id)
  },
}

const diffTab: TabDefinition<DiffTabState> = {
  kind: 'diff',
  component: DiffView,
  affectsWorkspace: false, // Diff tabs are for viewing file changes, don't change workspace context
  getComponentProps: tab => ({
    repoPath: tab.context.repoPath,
    filePath: tab.data.filePath,
    staged: tab.data.staged,
    commitHash: tab.data.commitHash,
  }),
  getPresentation: tab => {
    const parts = tab.data.filePath.split('/')
    const fileName = parts[parts.length - 1] || tab.data.filePath
    return {
      tooltip: `Diff: ${tab.data.filePath}`,
      badge: { text: 'DIFF', variant: 'diff' },
      title: fileName,
    }
  },
  isClosable: () => true,
  activate: async (tab, ctx) => {
    ctx.setActiveTabId(tab.id)
  },
}

type TabDefinitionMap = {
  [K in TabKind]: TabDefinition<Extract<TabState, { type: K }>>
}

const registry: TabDefinitionMap = {
  terminal: terminalTab,
  agent_terminal: agentTerminalTab,
  settings: settingsTab,
  diff: diffTab,
}

export const getTabDefinition = <TKind extends TabKind>(
  kind: TKind
): TabDefinition<Extract<TabState, { type: TKind }>> => {
  const def = registry[kind] as TabDefinition<Extract<TabState, { type: TKind }>> | undefined
  if (!def) throw new Error(`Missing tab definition for kind: ${kind}`)
  return def
}

export const tabRegistry = registry
