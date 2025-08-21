/**
 * 快捷键动作实现服务
 *
 * 将快捷键动作连接到实际的UI功能
 */

import { useTabManagerStore } from '@/stores/TabManager'
import { useTerminalStore } from '@/stores/Terminal'

export class ShortcutActionsService {
  private get tabManagerStore() {
    return useTabManagerStore()
  }

  private get terminalStore() {
    return useTerminalStore()
  }

  /**
   * 切换到指定索引的标签页
   */
  switchToTab(index: number): boolean {
    try {
      const tabs = this.tabManagerStore.tabs
      if (index >= 0 && index < tabs.length) {
        const tab = tabs[index]
        this.tabManagerStore.setActiveTab(tab.id)
        return true
      } else {
        return false
      }
    } catch (error) {
      return false
    }
  }

  /**
   * 切换到最后一个标签页
   */
  switchToLastTab(): boolean {
    try {
      const tabs = this.tabManagerStore.tabs
      if (tabs.length > 0) {
        const lastTab = tabs[tabs.length - 1]
        this.tabManagerStore.setActiveTab(lastTab.id)
        return true
      } else {
        return false
      }
    } catch (error) {
      return false
    }
  }

  /**
   * 新建标签页
   */
  async newTab(): Promise<boolean> {
    try {
      await this.terminalStore.createTerminal()
      return true
    } catch (error) {
      return false
    }
  }

  /**
   * 关闭当前标签页
   */
  closeCurrentTab(): boolean {
    try {
      const tabs = this.tabManagerStore.tabs
      const activeTab = this.tabManagerStore.activeTab

      // 如果只有一个标签页，不允许关闭，防止触发整个应用关闭
      if (tabs.length <= 1) {
        console.warn('⚠️ 无法关闭最后一个标签页，避免应用退出')
        return false
      }

      if (activeTab) {
        this.tabManagerStore.closeTab(activeTab.id)
        console.log(`✅ 已关闭标签页: ${activeTab.title || activeTab.shell}`)
        return true
      } else {
        console.warn('⚠️ 没有活动的标签页可关闭')
        return false
      }
    } catch (error) {
      console.error('❌ 关闭标签页失败:', error)
      return false
    }
  }

  /**
   * 新建窗口
   */
  async newWindow(): Promise<boolean> {
    try {
      // TODO: 实现新建窗口功能
      return false
    } catch (error) {
      return false
    }
  }

  /**
   * 复制选中内容
   */
  async copyToClipboard(): Promise<boolean> {
    try {
      // 浏览器的原生复制功能会自动处理
      return true
    } catch (error) {
      return false
    }
  }

  /**
   * 粘贴剪贴板内容
   */
  async pasteFromClipboard(): Promise<boolean> {
    try {
      // 浏览器的原生粘贴功能会自动处理
      return true
    } catch (error) {
      return false
    }
  }

  /**
   * 搜索
   */
  searchForward(): boolean {
    try {
      // TODO: 实现搜索功能
      return true
    } catch (error) {
      return false
    }
  }

  /**
   * 分割终端（垂直）
   */
  splitVertical(): boolean {
    try {
      // TODO: 实现垂直分割功能
      return false
    } catch (error) {
      return false
    }
  }

  /**
   * 分割终端（水平）
   */
  splitHorizontal(): boolean {
    try {
      // TODO: 实现水平分割功能
      return false
    } catch (error) {
      return false
    }
  }

  /**
   * 焦点切换到下一个面板
   */
  focusNextPane(): boolean {
    try {
      // TODO: 实现面板焦点切换功能
      return false
    } catch (error) {
      return false
    }
  }

  /**
   * 焦点切换到上一个面板
   */
  focusPrevPane(): boolean {
    try {
      // TODO: 实现面板焦点切换功能
      return false
    } catch (error) {
      return false
    }
  }

  /**
   * 接受当前补全建议
   */
  acceptCompletion(): boolean {
    try {
      // 查找当前激活的终端组件
      const activeTerminal = document.querySelector('.terminal-active')
      if (!activeTerminal) {
        return false
      }

      // 查找补全组件
      const completionComponent = activeTerminal.querySelector('.completion-suggestion')
      if (!completionComponent) {
        return false
      }

      // 触发补全接受
      // 这里需要通过事件或者组件引用来触发
      const event = new CustomEvent('accept-completion', { bubbles: true })
      activeTerminal.dispatchEvent(event)

      return true
    } catch (error) {
      return false
    }
  }

  /**
   * 切换到指定索引的标签页（支持更多标签页）
   */
  switchToTabByIndex(index: number): boolean {
    return this.switchToTab(index)
  }
}

// 导出单例实例
export const shortcutActionsService = new ShortcutActionsService()
