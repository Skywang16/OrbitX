/**
 * 系统级右键菜单 Composable API
 *
 * 使用 Tauri 原生菜单API实现系统级右键菜单功能
 */

import { ref, readonly } from 'vue'
import { Menu, MenuItem } from '@tauri-apps/api/menu'
import { LogicalPosition } from '@tauri-apps/api/dpi'

export interface PopoverMenuItem {
  label: string
  value?: unknown
  disabled?: boolean
  onClick?: () => void
}

export interface PopoverOptions {
  x: number
  y: number
  items: PopoverMenuItem[]
}

/**
 * 显示系统级右键菜单
 */
export const showContextMenu = async (options: PopoverOptions): Promise<void> => {
  if (options.items.length === 0) return

  const menuItems = []

  for (const item of options.items) {
    menuItems.push(
      await MenuItem.new({
        id: String(item.value || item.label),
        text: item.label,
        enabled: !item.disabled,
        action: () => {
          if (!item.disabled && item.onClick) {
            item.onClick()
          }
        },
      })
    )
  }

  if (menuItems.length > 0) {
    const menu = await Menu.new({ items: menuItems })
    const position = new LogicalPosition(options.x, options.y)
    await menu.popup(position)
  }
}

/**
 * 创建 Popover 实例
 *
 * 用于替代原来的 Popover 组件
 */
export const createPopover = () => {
  const visible = ref(false)
  const currentItems = ref<PopoverMenuItem[]>([])

  const show = async (x: number, y: number, items: PopoverMenuItem[]) => {
    currentItems.value = items
    visible.value = true

    await showContextMenu({ x, y, items })

    // 菜单自动关闭后更新状态
    visible.value = false
  }

  const hide = () => {
    visible.value = false
    currentItems.value = []
  }

  return {
    visible: readonly(visible),
    show,
    hide,
  }
}

/**
 * 便捷函数：直接在指定位置显示菜单
 */
export const showPopoverAt = async (x: number, y: number, items: PopoverMenuItem[]): Promise<void> => {
  await showContextMenu({ x, y, items })
}
