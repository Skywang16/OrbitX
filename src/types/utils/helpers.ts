/**
 * 类型辅助工具
 */

export const createDefaultSessionState = () => ({
  version: 1,
  tabs: [],
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
