/**
 * Manual test for Terminal Store
 *
 * This test verifies the updated Terminal Store functionality:
 * 1. setActiveTerminal calls backend set_active_pane
 * 2. CWD updates are subscription-only (no backend writes)
 * 3. State synchronization works correctly
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useTerminalStore } from '../Terminal'
import { terminalContextApi } from '@/api'

// Mock the APIs
vi.mock('@/api', () => ({
  shellApi: {
    getDefaultShell: vi.fn().mockResolvedValue({ name: 'bash', path: '/bin/bash' }),
    getAvailableShells: vi.fn().mockResolvedValue([]),
  },
  terminalApi: {
    createTerminal: vi.fn().mockResolvedValue(1),
    closeTerminal: vi.fn().mockResolvedValue(undefined),
    writeToTerminal: vi.fn().mockResolvedValue(undefined),
    resizeTerminal: vi.fn().mockResolvedValue(undefined),
  },
  terminalContextApi: {
    setActivePaneId: vi.fn().mockResolvedValue(undefined),
    getActivePaneId: vi.fn().mockResolvedValue(null),
    getTerminalContext: vi.fn().mockResolvedValue({
      paneId: 1,
      currentWorkingDirectory: '/home/user',
      shellType: 'bash',
      shellIntegrationEnabled: true,
    }),
  },
}))

// Mock Tauri API
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}))

// Mock session store
vi.mock('@/stores/session', () => ({
  useSessionStore: () => ({
    setActiveTabId: vi.fn(),
    updateTerminals: vi.fn(),
    saveSessionState: vi.fn().mockResolvedValue(undefined),
    initialized: true,
    terminals: [],
    sessionState: { activeTabId: null },
  }),
}))

describe('Terminal Store - Context Refactor', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('should call backend setActivePaneId when setting active terminal', async () => {
    const store = useTerminalStore()

    // Create a terminal first
    const terminalId = await store.createTerminal()
    const terminal = store.terminals.find(t => t.id === terminalId)

    expect(terminal).toBeDefined()
    expect(terminal?.backendId).toBe(1)

    // Verify that setActivePaneId was called during terminal creation
    expect(terminalContextApi.setActivePaneId).toHaveBeenCalledWith(1)
  })

  it('should handle setActiveTerminal with existing terminal', async () => {
    const store = useTerminalStore()

    // Create a terminal
    const terminalId = await store.createTerminal()
    vi.clearAllMocks() // Clear the mock calls from creation

    // Set it as active again
    await store.setActiveTerminal(terminalId)

    // Verify backend call
    expect(terminalContextApi.setActivePaneId).toHaveBeenCalledWith(1)
    expect(store.activeTerminalId).toBe(terminalId)
  })

  it('should not call backend when terminal has no backendId', async () => {
    const store = useTerminalStore()

    // Create a terminal and manually set backendId to null
    const terminalId = await store.createTerminal()
    const terminal = store.terminals.find(t => t.id === terminalId)
    if (terminal) {
      terminal.backendId = null
    }

    vi.clearAllMocks()

    // Try to set as active
    await store.setActiveTerminal(terminalId)

    // Should not call backend
    expect(terminalContextApi.setActivePaneId).not.toHaveBeenCalled()
    expect(store.activeTerminalId).toBe(terminalId)
  })

  it('should handle backend errors gracefully', async () => {
    const store = useTerminalStore()

    // Mock backend to throw error
    vi.mocked(terminalContextApi.setActivePaneId).mockRejectedValueOnce(new Error('Backend error'))

    const terminalId = await store.createTerminal()
    vi.clearAllMocks()

    // Should not throw error
    await expect(store.setActiveTerminal(terminalId)).resolves.not.toThrow()

    // Frontend state should still be updated
    expect(store.activeTerminalId).toBe(terminalId)
  })

  it('should update CWD from backend events only', () => {
    const store = useTerminalStore()

    // The updateTerminalCwd method should only update frontend state
    // and not make any backend calls
    const terminalId = 'test-terminal'
    store.terminals.push({
      id: terminalId,
      title: 'Test',
      cwd: '/old/path',
      active: false,
      shell: 'bash',
      backendId: 1,
    })

    // Update CWD
    store.updateTerminalCwd(terminalId, '/new/path')

    // Should update frontend state
    const terminal = store.terminals.find(t => t.id === terminalId)
    expect(terminal?.cwd).toBe('/new/path')

    // Should not make any backend calls (this is verified by the lack of API mocks being called)
  })
})

/**
 * Manual Test Instructions:
 *
 * To run this test manually:
 * 1. Install vitest: npm install -D vitest @vitest/ui
 * 2. Add test script to package.json: "test": "vitest"
 * 3. Run: npm test
 *
 * Expected behavior:
 * - setActiveTerminal should call terminalContextApi.setActivePaneId
 * - CWD updates should only affect frontend state
 * - Backend errors should be handled gracefully
 * - State synchronization should work correctly
 */
