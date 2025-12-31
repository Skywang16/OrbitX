/**
 * 从完整路径提取最后一部分作为显示名称
 * @param path 完整路径
 * @returns 路径最后一部分，如果是根目录或空返回 '~'
 */
export function getPathBasename(path: string): string {
  if (!path || path === '~') return '~'

  const parts = path.replace(/[/\\]+$/, '').split(/[/\\]/)
  const basename = parts[parts.length - 1]

  return basename || '~'
}
