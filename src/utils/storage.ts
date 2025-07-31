/**
 * 本地存储抽象层
 * 提供类型安全的本地存储操作
 */

export class StorageManager<T = any> {
  constructor(private key: string) {}

  /**
   * 保存数据到本地存储
   */
  save(data: T): boolean {
    try {
      localStorage.setItem(this.key, JSON.stringify(data))
      return true
    } catch (error) {
      console.error(`保存数据失败 [${this.key}]:`, error)
      return false
    }
  }

  /**
   * 从本地存储加载数据
   */
  load(): T | null {
    try {
      const data = localStorage.getItem(this.key)
      return data ? JSON.parse(data) : null
    } catch (error) {
      console.error(`加载数据失败 [${this.key}]:`, error)
      return null
    }
  }

  /**
   * 删除本地存储数据
   */
  remove(): boolean {
    try {
      localStorage.removeItem(this.key)
      return true
    } catch (error) {
      console.error(`删除数据失败 [${this.key}]:`, error)
      return false
    }
  }

  /**
   * 检查数据是否存在
   */
  exists(): boolean {
    return localStorage.getItem(this.key) !== null
  }
}

/**
 * 创建存储管理器实例
 */
export function createStorage<T>(key: string) {
  return new StorageManager<T>(key)
}
