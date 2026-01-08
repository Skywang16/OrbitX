import type { EditorSplitDirection, GroupId, GroupLeafNode, GroupNode, GroupSplitNode } from '@/types/domain/storage'

/**
 * 编辑器布局工具函数
 *
 * 布局使用二叉树结构：
 * - GroupLeafNode: 叶子节点，对应一个 tab group
 * - GroupSplitNode: 分割节点，包含 first/second 两个子节点和分割比例
 */

export const createEditorLayoutId = (prefix: string): string => {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return `${prefix}:${crypto.randomUUID()}`
  }
  return `${prefix}:${Date.now()}-${Math.random().toString(16).slice(2)}`
}

export const createGroupLeafNode = (groupId: GroupId): GroupLeafNode => {
  return {
    type: 'leaf',
    id: createEditorLayoutId('leaf'),
    groupId,
  }
}

export const createGroupSplitNode = (args: {
  direction: EditorSplitDirection
  ratio?: number
  first: GroupNode
  second: GroupNode
}): GroupSplitNode => {
  return {
    type: 'split',
    id: createEditorLayoutId('split'),
    direction: args.direction,
    ratio: typeof args.ratio === 'number' ? args.ratio : 0.5,
    first: args.first,
    second: args.second,
  }
}

export const getGroupIdsInLayout = (node: GroupNode): GroupId[] => {
  const ids: GroupId[] = []
  const walk = (n: GroupNode) => {
    if (n.type === 'leaf') {
      ids.push(n.groupId)
      return
    }
    walk(n.first)
    walk(n.second)
  }
  walk(node)
  return ids
}

/**
 * 获取“顶部边缘”的 groupIds：
 * - row: 左右并排，顶部同时可见 → 两边都算
 * - column: 上下堆叠，只有最上面的在顶部 → 只取 first
 */
export const getTopGroupIdsInLayout = (node: GroupNode): GroupId[] => {
  const ids: GroupId[] = []
  const walk = (n: GroupNode) => {
    if (n.type === 'leaf') {
      ids.push(n.groupId)
      return
    }
    if (n.direction === 'column') {
      walk(n.first)
      return
    }
    walk(n.first)
    walk(n.second)
  }
  walk(node)
  return ids
}

/** 在布局树中查找并替换指定 groupId 的叶子节点 */
export const replaceLeafByGroupId = (
  node: GroupNode,
  groupId: GroupId,
  replace: (leaf: GroupLeafNode) => GroupNode
): GroupNode => {
  if (node.type === 'leaf') {
    if (node.groupId !== groupId) return node
    return replace(node)
  }

  const nextFirst = replaceLeafByGroupId(node.first, groupId, replace)
  const nextSecond = replaceLeafByGroupId(node.second, groupId, replace)
  if (nextFirst === node.first && nextSecond === node.second) return node
  return { ...node, first: nextFirst, second: nextSecond }
}

export const updateSplitRatio = (node: GroupNode, splitId: string, ratio: number): GroupNode => {
  if (node.type === 'leaf') return node
  if (node.id === splitId) return { ...node, ratio }
  const nextFirst = updateSplitRatio(node.first, splitId, ratio)
  const nextSecond = updateSplitRatio(node.second, splitId, ratio)
  if (nextFirst === node.first && nextSecond === node.second) return node
  return { ...node, first: nextFirst, second: nextSecond }
}

/** 从布局树中移除指定 groupId 的叶子节点，返回被移除的节点 */
export const removeLeafByGroupId = (
  node: GroupNode,
  groupId: GroupId
): { node: GroupNode | null; removed: GroupLeafNode | null } => {
  if (node.type === 'leaf') {
    if (node.groupId !== groupId) return { node, removed: null }
    return { node: null, removed: node }
  }

  const firstResult = removeLeafByGroupId(node.first, groupId)
  if (firstResult.removed) {
    if (!firstResult.node) return { node: node.second, removed: firstResult.removed }
    return { node: { ...node, first: firstResult.node }, removed: firstResult.removed }
  }

  const secondResult = removeLeafByGroupId(node.second, groupId)
  if (secondResult.removed) {
    if (!secondResult.node) return { node: node.first, removed: secondResult.removed }
    return { node: { ...node, second: secondResult.node }, removed: secondResult.removed }
  }

  return { node, removed: null }
}
