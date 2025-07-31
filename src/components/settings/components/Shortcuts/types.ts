/**
 * 快捷键设置组件类型定义
 */

import type { ShortcutBinding, ShortcutCategory } from '@/api/shortcuts/types';

/**
 * 快捷键列表项
 */
export interface ShortcutListItem {
  /** 快捷键绑定 */
  binding: ShortcutBinding;
  /** 类别 */
  category: ShortcutCategory;
  /** 索引 */
  index: number;
  /** 是否有冲突 */
  hasConflict?: boolean;
  /** 是否有验证错误 */
  hasError?: boolean;
  /** 格式化的快捷键字符串 */
  formatted?: string;
}

/**
 * 快捷键编辑器模式
 */
export enum ShortcutEditorMode {
  Add = 'add',
  Edit = 'edit',
  View = 'view'
}

/**
 * 快捷键编辑器选项
 */
export interface ShortcutEditorOptions {
  /** 编辑模式 */
  mode: ShortcutEditorMode;
  /** 初始快捷键 */
  initialShortcut?: ShortcutBinding;
  /** 初始类别 */
  initialCategory?: ShortcutCategory;
  /** 初始索引 */
  initialIndex?: number;
  /** 是否显示高级选项 */
  showAdvanced?: boolean;
}

/**
 * 快捷键搜索过滤器
 */
export interface ShortcutSearchFilter {
  /** 搜索关键词 */
  query: string;
  /** 选中的类别 */
  categories: ShortcutCategory[];
  /** 是否只显示有冲突的 */
  conflictsOnly: boolean;
  /** 是否只显示有错误的 */
  errorsOnly: boolean;
}

/**
 * 快捷键统计数据
 */
export interface ShortcutStatsData {
  /** 总数 */
  total: number;
  /** 各类别数量 */
  byCategory: Record<ShortcutCategory, number>;
  /** 冲突数量 */
  conflicts: number;
  /** 错误数量 */
  errors: number;
}

/**
 * 快捷键操作类型
 */
export enum ShortcutActionType {
  Add = 'add',
  Edit = 'edit',
  Delete = 'delete',
  Duplicate = 'duplicate',
  Export = 'export',
  Import = 'import',
  Reset = 'reset'
}

/**
 * 快捷键操作事件
 */
export interface ShortcutActionEvent {
  /** 操作类型 */
  type: ShortcutActionType;
  /** 相关的快捷键项 */
  item?: ShortcutListItem;
  /** 额外数据 */
  data?: any;
}
