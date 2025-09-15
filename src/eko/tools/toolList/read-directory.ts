/**
 * 目录读取工具
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@/eko-core/types'
import { FileNotFoundError, ToolError } from '../tool-error'
import { filesystemApi } from '@/api'

export interface ReadDirectoryParams {
  path: string
}

export interface FileEntry {
  name: string
  path: string
  isDirectory: boolean
}

/**
 * 目录读取工具
 */
export class ReadDirectoryTool extends ModifiableTool {
  constructor() {
    super(
      'read_directory',
      `List directory contents in tree format. Use this for basic directory structure overview when orbit_search results need additional context. Recursively lists files and subdirectories up to 5 levels deep. For code understanding and finding specific functionality, prefer orbit_search instead. Must use absolute paths.`,
      {
        type: 'object',
        properties: {
          path: {
            type: 'string',
            description:
              'Absolute path to the directory. Must be a complete path, for example: "/Users/user/project/src", "/home/user/workspace/components"',
          },
        },
        required: ['path'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { path } = context.parameters as unknown as ReadDirectoryParams

    console.warn(`[ReadDirectoryTool] 输入参数: ${path}`)

    try {
      // 检查目录是否存在
      const exists = await this.checkPathExists(path)
      if (!exists) {
        throw new FileNotFoundError(path)
      }

      // 递归读取目录内容（最多5层）
      const entries = await this.readDirectoryRecursive(path, 0, 5)

      // 格式化为树形输出
      const output = await this.formatTreeOutput(path, entries)

      console.warn(`[ReadDirectoryTool] 输出结果:\n${output}`)

      return {
        content: [
          {
            type: 'text',
            text: output,
          },
        ],
      }
    } catch (error) {
      console.error(`[ReadDirectoryTool] 执行失败:`, error)
      if (error instanceof FileNotFoundError) {
        throw error
      }
      throw new ToolError(`Failed to read directory: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  private async checkPathExists(path: string): Promise<boolean> {
    try {
      const exists = await filesystemApi.exists(path)
      return exists
    } catch (error) {
      return false
    }
  }

  private async readDirectoryRecursive(dirPath: string, currentDepth: number, maxDepth: number): Promise<FileEntry[]> {
    const entries: FileEntry[] = []

    if (currentDepth >= maxDepth) {
      return entries
    }

    try {
      // 使用Tauri API读取目录
      const dirEntries = await filesystemApi.readDir(dirPath)

      for (const entry of dirEntries) {
        // 过滤隐藏文件、依赖、缓存等噪音文件
        if (this.shouldSkipEntry(entry.name, entry.isDirectory)) {
          continue
        }

        // 构建完整路径
        const fullPath = `${dirPath}/${entry.name}`.replace(/\/+/g, '/')

        const fileEntry: FileEntry = {
          name: entry.name,
          path: fullPath,
          isDirectory: entry.isDirectory,
        }

        entries.push(fileEntry)
      }

      // 对目录排序，目录在前，文件在后
      entries.sort((a, b) => {
        if (a.isDirectory && !b.isDirectory) return -1
        if (!a.isDirectory && b.isDirectory) return 1
        return a.name.localeCompare(b.name)
      })
    } catch (error) {
      // 如果读取失败，返回已有的entries
      console.warn(`Failed to read directory ${dirPath}: ${error instanceof Error ? error.message : String(error)}`)
    }

    return entries
  }

  private shouldSkipEntry(name: string, isDirectory: boolean): boolean {
    // 隐藏文件和文件夹
    if (name.startsWith('.')) {
      return true
    }

    // 常见的依赖和缓存文件夹
    const skipDirectories = new Set([
      'node_modules',
      '__pycache__',
      '.pytest_cache',
      '.coverage',
      'coverage',
      'dist',
      'build',
      'out',
      'target', // Rust
      'bin',
      'obj', // C#/.NET
      '.gradle', // Gradle
      '.maven', // Maven
      'vendor', // PHP/Go vendor
      '.bundle', // Ruby
      '.cache',
      '.tmp',
      '.temp',
      'tmp',
      'temp',
      '.nuxt', // Nuxt.js
      '.next', // Next.js
      '.vscode',
      '.idea', // IntelliJ
      '.vs', // Visual Studio
      'bower_components',
      'jspm_packages',
      'web_modules',
      'logs',
      '*.egg-info', // Python eggs
      '.tox', // Python tox
      '.venv', // Python virtual env
      'venv',
      'env',
      '.env',
    ])

    if (isDirectory && skipDirectories.has(name)) {
      return true
    }

    // 常见的缓存和临时文件
    const skipFilePatterns = [
      /\.log$/,
      /\.cache$/,
      /\.tmp$/,
      /\.temp$/,
      /\.bak$/,
      /\.backup$/,
      /\.swp$/,
      /\.swo$/,
      /~$/,
      /\.pyc$/,
      /\.pyo$/,
      /\.pyd$/,
      /\.so$/,
      /\.dll$/,
      /\.dylib$/,
      /\.o$/,
      /\.obj$/,
      /\.class$/,
      /\.jar$/,
      /\.war$/,
      /\.ear$/,
      /\.dSYM$/,
      /Thumbs\.db$/,
      /\.DS_Store$/,
      /desktop\.ini$/,
      /\.lock$/,
      /package-lock\.json$/,
      /yarn\.lock$/,
      /pnpm-lock\.yaml$/,
      /Pipfile\.lock$/,
      /poetry\.lock$/,
      /Cargo\.lock$/,
      /composer\.lock$/,
      /Gemfile\.lock$/,
    ]

    if (!isDirectory) {
      for (const pattern of skipFilePatterns) {
        if (pattern.test(name)) {
          return true
        }
      }
    }

    return false
  }

  private async formatTreeOutput(rootPath: string, entries: FileEntry[]): Promise<string> {
    if (entries.length === 0) {
      return `Directory is empty`
    }

    const lines: string[] = []
    const allFiles: string[] = []

    // 收集所有文件路径
    await this.collectAllFiles(rootPath, entries, allFiles, 0, 5)

    // 计算相对路径的基础路径
    const basePath = rootPath.endsWith('/') ? rootPath.slice(0, -1) : rootPath

    // 输出所有文件的相对路径
    for (const filePath of allFiles.sort()) {
      const relativePath = filePath.replace(basePath + '/', '')
      lines.push(relativePath)
    }

    // 添加LLM友好的提示
    const MAX_DISPLAY_ITEMS = 1000
    if (allFiles.length > MAX_DISPLAY_ITEMS) {
      lines.push('')
      lines.push(`Important note: Directory structure has been truncated (maximum 5 levels deep).`)
      lines.push(`Status: Partial content shown, actual project may contain more files.`)
      lines.push(`Suggestion: To view specific files, please use the read_file tool.`)
    }

    return lines.join('\n')
  }

  private async collectAllFiles(
    rootPath: string,
    entries: FileEntry[],
    allFiles: string[],
    currentDepth: number,
    maxDepth: number
  ): Promise<void> {
    if (currentDepth >= maxDepth) {
      return
    }

    for (const entry of entries) {
      if (entry.isDirectory) {
        // 递归读取子目录
        try {
          const subEntries = await this.readDirectoryRecursive(entry.path, currentDepth + 1, maxDepth)
          if (subEntries.length > 0) {
            await this.collectAllFiles(rootPath, subEntries, allFiles, currentDepth + 1, maxDepth)
          }
        } catch (error) {
          // 忽略无法读取的目录
        }
      } else {
        // 收集文件路径
        allFiles.push(entry.path)
      }
    }
  }
}

export const readDirectoryTool = new ReadDirectoryTool()
