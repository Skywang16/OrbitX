/**
 * 文件系统 API
 *
 * 提供文件系统操作的统一接口，包括：
 * - 文件读取
 * - 目录操作
 * - 文件元数据
 */

import { invoke } from '@tauri-apps/api/core'

/**
 * 文件系统 API 接口类
 */
export class FilesystemApi {
  /**
   * 读取文本文件
   */
  async readTextFile(path: string): Promise<ArrayBuffer> {
    return await invoke<ArrayBuffer>('plugin:fs|read_text_file', { path })
  }

  /**
   * 检查文件或目录是否存在
   */
  async exists(path: string): Promise<boolean> {
    return await invoke<boolean>('plugin:fs|exists', { path })
  }

  /**
   * 获取文件或目录元数据
   */
  async getMetadata(path: string): Promise<{ isDir?: boolean; size?: number }> {
    return await invoke<{ isDir?: boolean; size?: number }>('plugin:fs|metadata', { path })
  }

  /**
   * 检查是否为目录
   */
  async isDirectory(path: string): Promise<boolean> {
    const metadata = await this.getMetadata(path)
    return metadata.isDir || false
  }

  /**
   * 获取文件大小
   */
  async getFileSize(path: string): Promise<number> {
    const metadata = await this.getMetadata(path)
    return metadata.size || 0
  }

  /**
   * 读取目录内容
   */
  async readDir(path: string): Promise<
    Array<{
      name: string
      isDirectory: boolean
      isFile: boolean
      isSymlink: boolean
    }>
  > {
    return await invoke<
      Array<{
        name: string
        isDirectory: boolean
        isFile: boolean
        isSymlink: boolean
      }>
    >('plugin:fs|read_dir', { path })
  }
}

// 导出单例实例
export const filesystemApi = new FilesystemApi()

// 默认导出
export default filesystemApi
