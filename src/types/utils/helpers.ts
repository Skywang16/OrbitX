/**
 * 类型辅助工具
 */

import { createGroupId } from '@/types/domain/storage'

export const createDefaultSessionState = () => ({
  version: 1,
  workspace: (() => {
    const groupId = createGroupId('group')
    return {
      root: { type: 'leaf' as const, id: 'leaf:0', groupId },
      groups: {
        [groupId]: {
          id: groupId,
          tabs: [],
          activeTabId: null,
        },
      },
      activeGroupId: groupId,
    }
  })(),
  ui: {
    theme: 'dark',
    fontSize: 14,
    sidebarWidth: 300,
    leftSidebarVisible: false,
    leftSidebarWidth: 280,
    leftSidebarActivePanel: 'workspace' as const,
    onboardingCompleted: false,
  },
  ai: {
    visible: false,
    width: 350,
    mode: 'chat' as const,
    workspacePath: undefined,
    sessionId: undefined,
    selectedModelId: undefined,
  },
  timestamp: new Date().toISOString(),
})
