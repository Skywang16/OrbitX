import type { AiState, UiState } from '@/types/domain/storage'
import type { LeftSidebarPanel } from '@/stores/layout'

export type LeftSidebarPersistedState = Required<
  Pick<UiState, 'leftSidebarVisible' | 'leftSidebarWidth' | 'leftSidebarActivePanel'>
>

export const restoreLeftSidebarState = (ui: UiState | undefined | null): Partial<LeftSidebarPersistedState> => {
  if (!ui) return {}

  const restored: Partial<LeftSidebarPersistedState> = {}

  if (typeof ui.leftSidebarVisible === 'boolean') {
    restored.leftSidebarVisible = ui.leftSidebarVisible
  }

  if (typeof ui.leftSidebarWidth === 'number') {
    restored.leftSidebarWidth = ui.leftSidebarWidth
  }

  const panel = ui.leftSidebarActivePanel
  if (panel === 'workspace' || panel === 'git' || panel === null) {
    restored.leftSidebarActivePanel = panel
  }

  return restored
}

export const persistLeftSidebarState = (state: {
  leftSidebarVisible: boolean
  leftSidebarWidth: number
  leftSidebarActivePanel: LeftSidebarPanel
}): Partial<UiState> => {
  return {
    leftSidebarVisible: state.leftSidebarVisible,
    leftSidebarWidth: state.leftSidebarWidth,
    leftSidebarActivePanel: state.leftSidebarActivePanel,
  }
}

export type AiSidebarPersistedState = Pick<AiState, 'visible' | 'width' | 'mode'>

export const restoreAiSidebarState = (ai: AiState | undefined | null): Partial<AiSidebarPersistedState> => {
  if (!ai) return {}

  const restored: Partial<AiSidebarPersistedState> = {}

  if (typeof ai.visible === 'boolean') restored.visible = ai.visible
  if (typeof ai.width === 'number') restored.width = ai.width
  if (ai.mode === 'chat' || ai.mode === 'agent') restored.mode = ai.mode

  return restored
}
