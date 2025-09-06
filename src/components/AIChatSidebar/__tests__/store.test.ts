/**
 * @vitest-environment jsdom
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useAIChatStore } from '../store'
import { useTerminalStore } from '@/stores/Terminal'
import { aiApi } from '@/api'

// Mock the API modules
vi.mock('@/api', () => ({
  aiApi: {
    createConversation: vi.fn(),
    saveMessage: vi.fn(),
    buildPromptWithContext: vi.fn(),
    getConversations: vi.fn(),
    updateMessageContent: vi.fn(),
    updateMessageStatus: vi.fn(),
    updateMessageSteps: vi.fn(),
  },
}))

vi.mock('@/stores/Terminal', () => ({
  useTerminalStore: vi.fn(),
}))

vi.mock('@/stores/session', () => ({
  useSessionStore: vi.fn(() => ({
    aiState: {
      selectedModelId: 'test-model',
    },
  })),
}))

vi.mock('@/components/settings/components/AI', () => ({
  useAISettingsStore: vi.fn(() => ({
    hasModels: true,
    isLoading: false,
    loadSettings: vi.fn(),
  })),
}))

vi.mock('@/eko', () => ({
  createTerminalEko: vi.fn(() => ({
    setMode: vi.fn(),
    run: vi.fn(() => Promise.resolve({ success: true, result: 'Test response' })),
    abort: vi.fn(),
    setSelectedModelId: vi.fn(),
  })),
  createSidebarCallback: vi.fn(handler => handler),
}))

// Mock the error handler to avoid DOM issues
vi.mock('@/utils/errorHandler', () => ({
  handleErrorWithMessage: vi.fn((error, message) => {
    return `${message}: ${error.message}`
  }),
}))

describe('AI Chat Store - Terminal Context Integration', () => {
  let aiChatStore: ReturnType<typeof useAIChatStore>
  let mockTerminalStore: any

  beforeEach(() => {
    setActivePinia(createPinia())

    // Setup mock terminal store
    mockTerminalStore = {
      terminals: [
        {
          id: 'terminal-1',
          backendId: 123,
          title: 'Terminal 1',
          cwd: '/home/user',
        },
        {
          id: 'terminal-2',
          backendId: 456,
          title: 'Terminal 2',
          cwd: '/home/user/project',
        },
      ],
      activeTerminalId: 'terminal-1',
    }

    vi.mocked(useTerminalStore).mockReturnValue(mockTerminalStore)

    // Setup API mocks
    vi.mocked(aiApi.createConversation).mockResolvedValue(1)
    vi.mocked(aiApi.saveMessage).mockResolvedValue(1)
    vi.mocked(aiApi.buildPromptWithContext).mockResolvedValue('Test prompt')
    vi.mocked(aiApi.getConversations).mockResolvedValue([])
    vi.mocked(aiApi.updateMessageContent).mockResolvedValue(undefined)
    vi.mocked(aiApi.updateMessageStatus).mockResolvedValue(undefined)
    vi.mocked(aiApi.updateMessageSteps).mockResolvedValue(undefined)

    aiChatStore = useAIChatStore()
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  describe('sendMessage with new terminal context integration', () => {
    it('should pass activeTerminal.backendId as paneId parameter', async () => {
      // Arrange
      aiChatStore.currentConversationId = 1

      // Act
      await aiChatStore.sendMessage('Test message')

      // Assert
      expect(aiApi.buildPromptWithContext).toHaveBeenCalledWith(
        1, // conversationId
        'Test message', // content
        1, // userMessageId
        123 // paneId (activeTerminal.backendId)
      )
    })

    it('should pass undefined paneId when no active terminal', async () => {
      // Arrange
      aiChatStore.currentConversationId = 1
      mockTerminalStore.activeTerminalId = null

      // Act
      await aiChatStore.sendMessage('Test message')

      // Assert
      expect(aiApi.buildPromptWithContext).toHaveBeenCalledWith(
        1, // conversationId
        'Test message', // content
        1, // userMessageId
        undefined // paneId
      )
    })

    it('should pass undefined paneId when active terminal has no backendId', async () => {
      // Arrange
      aiChatStore.currentConversationId = 1
      mockTerminalStore.terminals[0].backendId = null

      // Act
      await aiChatStore.sendMessage('Test message')

      // Assert
      expect(aiApi.buildPromptWithContext).toHaveBeenCalledWith(
        1, // conversationId
        'Test message', // content
        1, // userMessageId
        undefined // paneId
      )
    })

    it('should use fallback when buildPromptWithContext fails with paneId', async () => {
      // Arrange
      aiChatStore.currentConversationId = 1
      vi.mocked(aiApi.buildPromptWithContext)
        .mockRejectedValueOnce(new Error('Terminal context error'))
        .mockResolvedValueOnce('Fallback prompt')

      // Act
      await aiChatStore.sendMessage('Test message')

      // Assert
      expect(aiApi.buildPromptWithContext).toHaveBeenCalledTimes(2)

      // First call with paneId
      expect(aiApi.buildPromptWithContext).toHaveBeenNthCalledWith(
        1,
        1, // conversationId
        'Test message', // content
        1, // userMessageId
        123 // paneId
      )

      // Second call without paneId (fallback)
      expect(aiApi.buildPromptWithContext).toHaveBeenNthCalledWith(
        2,
        1, // conversationId
        'Test message', // content
        1 // userMessageId
        // no paneId parameter
      )
    })

    it('should throw error when both primary and fallback calls fail', async () => {
      // Arrange
      aiChatStore.currentConversationId = 1
      vi.mocked(aiApi.buildPromptWithContext)
        .mockRejectedValueOnce(new Error('Terminal context error'))
        .mockRejectedValueOnce(new Error('Fallback error'))

      // Act & Assert
      await expect(aiChatStore.sendMessage('Test message')).rejects.toThrow('无法构建AI提示，请检查终端状态')
    })

    it('should work with different active terminals', async () => {
      // Arrange
      aiChatStore.currentConversationId = 1
      mockTerminalStore.activeTerminalId = 'terminal-2'

      // Act
      await aiChatStore.sendMessage('Test message')

      // Assert
      expect(aiApi.buildPromptWithContext).toHaveBeenCalledWith(
        1, // conversationId
        'Test message', // content
        1, // userMessageId
        456 // paneId (terminal-2's backendId)
      )
    })
  })

  describe('error handling and resilience', () => {
    it('should handle missing terminal gracefully', async () => {
      // Arrange
      aiChatStore.currentConversationId = 1
      mockTerminalStore.terminals = []
      mockTerminalStore.activeTerminalId = 'non-existent'

      // Act
      await aiChatStore.sendMessage('Test message')

      // Assert
      expect(aiApi.buildPromptWithContext).toHaveBeenCalledWith(
        1, // conversationId
        'Test message', // content
        1, // userMessageId
        undefined // paneId should be undefined
      )
    })

    it('should log warning when context retrieval fails', async () => {
      // Arrange
      const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
      aiChatStore.currentConversationId = 1
      vi.mocked(aiApi.buildPromptWithContext)
        .mockRejectedValueOnce(new Error('Context error'))
        .mockResolvedValueOnce('Fallback prompt')

      // Act
      await aiChatStore.sendMessage('Test message')

      // Assert
      expect(consoleSpy).toHaveBeenCalledWith('获取终端上下文失败，使用回退逻辑:', expect.any(Error))

      consoleSpy.mockRestore()
    })

    it('should log error when both attempts fail', async () => {
      // Arrange
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
      aiChatStore.currentConversationId = 1
      vi.mocked(aiApi.buildPromptWithContext).mockRejectedValue(new Error('Persistent error'))

      // Act & Assert
      await expect(aiChatStore.sendMessage('Test message')).rejects.toThrow()
      expect(consoleSpy).toHaveBeenCalledWith('构建AI提示失败:', expect.any(Error))

      consoleSpy.mockRestore()
    })
  })
})
