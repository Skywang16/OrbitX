export class StorageManager<T = unknown> {
  constructor(private key: string) {}

  save = (data: T): boolean => {
    try {
      localStorage.setItem(this.key, JSON.stringify(data))
      return true
    } catch (error) {
      console.error(`Failed to save data [${this.key}]:`, error)
      return false
    }
  }

  load = (): T | null => {
    try {
      const data = localStorage.getItem(this.key)
      return data ? JSON.parse(data) : null
    } catch (error) {
      console.error(`Failed to load data [${this.key}]:`, error)
      return null
    }
  }

  remove = (): boolean => {
    try {
      localStorage.removeItem(this.key)
      return true
    } catch (error) {
      console.error(`Failed to remove data [${this.key}]:`, error)
      return false
    }
  }

  exists = (): boolean => {
    return localStorage.getItem(this.key) !== null
  }
}

export const createStorage = <T>(key: string) => {
  return new StorageManager<T>(key)
}
