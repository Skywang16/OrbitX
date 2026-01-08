/**
 * FNV-1a 32位哈希算法
 * 用于生成稳定的字符串哈希值，适合作为缓存 key 或去重标识
 */
export const fnv1aHash = (str: string): string => {
  let hash = 2166136261 // FNV offset basis
  for (let i = 0; i < str.length; i++) {
    hash ^= str.charCodeAt(i)
    hash = Math.imul(hash, 16777619) // FNV prime
  }
  return (hash >>> 0).toString(16)
}
