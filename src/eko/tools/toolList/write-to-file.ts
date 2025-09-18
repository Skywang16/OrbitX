/**
 * write_to_file 工具
 * 支持：
 * - 覆盖写入或新建文件
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@/eko-core/types'
import { filesystemApi } from '@/api'
import { ValidationError, ToolError } from '../tool-error'
import { writeTextFile } from '@tauri-apps/plugin-fs'

export interface WriteToFileParams {
  path: string
  content: string
}

export class WriteToFileTool extends ModifiableTool {
  constructor() {
    super(
      'write_to_file',
      `Create or overwrite a file with provided content.
Notes:
- Absolute paths are required.
- If the file exists, it will be overwritten entirely.`,
      {
        type: 'object',
        properties: {
          path: { type: 'string', description: 'Absolute file path' },
          content: { type: 'string', description: 'New file content' },
        },
        required: ['path', 'content'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const params = context.parameters as unknown as WriteToFileParams

    const path = (params.path || '').toString().trim()
    if (!path) throw new ValidationError('File path cannot be empty')
    if (!isAbsolutePath(path)) throw new ValidationError('Path must be an absolute path')

    const content = params.content ?? ''

    const exists = await filesystemApi.exists(path)
    const isDir = exists ? await filesystemApi.isDirectory(path) : false
    if (isDir) throw new ValidationError(`Path ${path} is a directory, cannot write file`)
    if (isBinaryFile(path)) throw new ValidationError(`File ${path} appears to be binary, text writing not supported`)

    const created = !exists

    try {
      await writeTextFile(path, content)
    } catch (e) {
      throw new ToolError(`Failed to write file: ${e instanceof Error ? e.message : String(e)}`)
    }

    return {
      content: [
        {
          type: 'text',
          text: `write_to_file applied\nFile: ${path}\nCreated: ${created}`,
        },
      ],
      extInfo: {
        file: path,
        created,
      },
    }
  }
}

export const writeToFileTool = new WriteToFileTool()

// ===== 辅助函数 =====
function isAbsolutePath(p: string): boolean {
  return p.startsWith('/')
}

function isBinaryFile(path: string): boolean {
  const ext = path.toLowerCase().split('.').pop() || ''
  const binaryExtensions = new Set([
    'jpg',
    'jpeg',
    'png',
    'gif',
    'bmp',
    'tiff',
    'tif',
    'webp',
    'ico',
    'svg',
    'mp3',
    'wav',
    'flac',
    'aac',
    'ogg',
    'm4a',
    'wma',
    'mp4',
    'avi',
    'mkv',
    'mov',
    'wmv',
    'flv',
    'webm',
    '3gp',
    'zip',
    'rar',
    '7z',
    'tar',
    'gz',
    'bz2',
    'xz',
    'exe',
    'dll',
    'so',
    'dylib',
    'app',
    'deb',
    'rpm',
    'dmg',
    'doc',
    'docx',
    'xls',
    'xlsx',
    'ppt',
    'pptx',
    'pdf',
    'ttf',
    'otf',
    'woff',
    'woff2',
    'eot',
    'db',
    'sqlite',
    'sqlite3',
    'class',
    'jar',
    'war',
    'ear',
    'pyc',
    'pyo',
    'o',
    'obj',
    'bin',
    'dat',
    'iso',
    'img',
  ])
  return binaryExtensions.has(ext)
}
