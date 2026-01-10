/**
 * 文件系统 API
 *
 * 提供文件系统操作的统一接口，包括：
 * - 文件读取
 * - 目录操作
 * - 文件元数据
 */

import { invoke } from '@tauri-apps/api/core'
import { invoke as appInvoke } from '@/utils/request'

/**
 * 文件系统 API 接口类
 */
export class FilesystemApi {
  /**
   * 读取文本文件
   */
  readTextFile = async (path: string): Promise<ArrayBuffer> => {
    return await invoke<ArrayBuffer>('plugin:fs|read_text_file', { path })
  }

  /**
   * 检查文件或目录是否存在
   */
  exists = async (path: string): Promise<boolean> => {
    return await invoke<boolean>('plugin:fs|exists', { path })
  }

  /**
   * 获取文件或目录元数据（使用 Tauri fs 插件的 stat 接口）
   */
  getMetadata = async (
    path: string
  ): Promise<{ isDir?: boolean; size?: number; isFile?: boolean; isSymlink?: boolean }> => {
    // tauri-plugin-fs v2 使用 'stat'，权限对应 capabilities 中的 "fs:allow-stat"
    return await invoke<{ isDir?: boolean; size?: number; isFile?: boolean; isSymlink?: boolean }>('plugin:fs|stat', {
      path,
    })
  }

  /**
   * 检查是否为目录
   */
  isDirectory = async (path: string): Promise<boolean> => {
    const metadata = await this.getMetadata(path)
    return metadata.isDir || false
  }

  /**
   * 获取文件大小
   */
  getFileSize = async (path: string): Promise<number> => {
    const metadata = await this.getMetadata(path)
    return metadata.size || 0
  }

  /**
   * 读取目录内容（包含 gitignore 状态）
   */
  readDir = async (
    path: string
  ): Promise<
    Array<{
      name: string
      isDirectory: boolean
      isFile: boolean
      isSymlink: boolean
      isIgnored: boolean
    }>
  > => {
    return await appInvoke<
      Array<{
        name: string
        is_directory: boolean
        is_file: boolean
        is_symlink: boolean
        is_ignored: boolean
      }>
    >('fs_read_dir', { path }).then(entries =>
      entries.map(entry => ({
        name: entry.name,
        isDirectory: entry.is_directory,
        isFile: entry.is_file,
        isSymlink: entry.is_symlink,
        isIgnored: entry.is_ignored,
      }))
    )
  }

  /**
   * 列出目录（后端命令，完整 .gitignore 语义，递归可选）
   */
}

export const filesystemApi = new FilesystemApi()

// 默认导出
export default filesystemApi
