import type { TabContext, TabState } from '@/types/domain/storage'
import type { RuntimeTerminalState } from '@/types/domain/storage'

export interface TabContextResolverDeps {
  terminals: RuntimeTerminalState[]
}

export const getWorkspacePathFromContext = (context: TabContext, deps: TabContextResolverDeps): string | null => {
  if (context.kind === 'workspace') return context.path
  if (context.kind === 'git') return context.repoPath
  if (context.kind === 'terminal') {
    return deps.terminals.find(t => t.id === context.paneId)?.cwd ?? null
  }
  return null
}

export const getWorkspacePathForTab = (tab: TabState, deps: TabContextResolverDeps): string | null => {
  return getWorkspacePathFromContext(tab.context, deps)
}
