/**
 * 状态持久化测试工具
 *
 * 用于测试完整的状态保存和恢复流程
 */

import { useSessionStore } from '@/stores/session'
import { useTerminalStore } from '@/stores/Terminal'
import { useAIChatStore } from '@/components/AIChatSidebar'

/**
 * 测试状态持久化功能
 */
export async function testStatePersistence() {
  console.log('🧪 开始测试状态持久化功能')

  const sessionStore = useSessionStore()
  const terminalStore = useTerminalStore()
  const aiChatStore = useAIChatStore()

  try {
    // 1. 初始化所有Store
    console.log('📋 初始化Store...')
    await sessionStore.initialize()
    await terminalStore.initializeTerminalStore()
    await aiChatStore.initialize()

    // 2. 创建一些测试数据
    console.log('📝 创建测试数据...')

    // 创建终端
    const terminalId = await terminalStore.createTerminal('/tmp')
    console.log(`✅ 创建终端: ${terminalId}`)

    // 更新窗口状态
    sessionStore.updateWindowState({
      x: 200,
      y: 150,
      width: 1400,
      height: 900,
      maximized: false,
    })
    console.log('✅ 更新窗口状态')

    // 更新UI状态
    sessionStore.updateUiState({
      theme: 'light',
      fontSize: 16,
      sidebarWidth: 400,
    })
    console.log('✅ 更新UI状态')

    // 更新AI状态
    sessionStore.updateAiState({
      visible: true,
      width: 400,
      mode: 'agent',
      conversationId: 123,
    })
    console.log('✅ 更新AI状态')

    // 3. 立即保存状态
    console.log('💾 保存状态...')
    await sessionStore.saveImmediately()
    console.log('✅ 状态保存完成')

    // 4. 验证保存的状态
    console.log('🔍 验证保存的状态...')
    const currentState = sessionStore.sessionState

    console.log('📊 当前状态:')
    console.log(
      `  - 窗口: ${currentState.window.width}x${currentState.window.height} at (${currentState.window.x}, ${currentState.window.y})`
    )
    console.log(`  - 终端数量: ${currentState.terminals.length}`)
    console.log(`  - UI主题: ${currentState.ui.theme}, 字体: ${currentState.ui.fontSize}px`)
    console.log(`  - AI可见: ${currentState.ai.visible}, 模式: ${currentState.ai.mode}`)

    // 5. 模拟重新加载
    console.log('🔄 模拟重新加载...')
    await sessionStore.loadSessionState()

    const reloadedState = sessionStore.sessionState
    console.log('📊 重新加载后的状态:')
    console.log(
      `  - 窗口: ${reloadedState.window.width}x${reloadedState.window.height} at (${reloadedState.window.x}, ${reloadedState.window.y})`
    )
    console.log(`  - 终端数量: ${reloadedState.terminals.length}`)
    console.log(`  - UI主题: ${reloadedState.ui.theme}, 字体: ${reloadedState.ui.fontSize}px`)
    console.log(`  - AI可见: ${reloadedState.ai.visible}, 模式: ${reloadedState.ai.mode}`)

    // 6. 验证数据一致性
    const isConsistent =
      currentState.window.width === reloadedState.window.width &&
      currentState.terminals.length === reloadedState.terminals.length &&
      currentState.ui.theme === reloadedState.ui.theme &&
      currentState.ai.visible === reloadedState.ai.visible

    if (isConsistent) {
      console.log('✅ 状态持久化测试通过！数据一致性验证成功')
      return true
    } else {
      console.error('❌ 状态持久化测试失败！数据不一致')
      return false
    }
  } catch (error) {
    console.error('❌ 状态持久化测试失败:', error)
    return false
  }
}

/**
 * 在开发环境中暴露测试函数到全局
 */
if (import.meta.env.DEV) {
  ;(window as any).testStatePersistence = testStatePersistence
  console.log('🧪 状态持久化测试函数已暴露到 window.testStatePersistence()')
}
