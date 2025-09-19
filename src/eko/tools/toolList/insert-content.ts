/**
 * insert_content 工具
 * 支持：
 * - 在指定行插入文本（不修改已有行）
 * - 新文件创建：当文件不存在时，允许在第 0 行（追加）或第 1 行（文件开头）插入
 * - 预览模式与审批
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@/eko-core/types'
import { filesystemApi } from '@/api'
import { ValidationError, ToolError } from '../tool-error'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'

export interface InsertContentParams {
  path: string
  line: number // 1-based；若为 0，则表示在文件末尾追加（兼容 Roo 的语义）
  content: string
  previewOnly?: boolean
  requireApproval?: boolean
}

export class InsertContentTool extends ModifiableTool {
  constructor() {
    super(
      'insert_content',
      `Insert lines into a file at a specific line without modifying existing lines.
Notes:
- line is 1-based. When line=0, append to the end. When creating a new file, only line=0 (append) or line=1 (insert at beginning) are valid.
- If previewOnly=true, only preview the change (no write).
- If requireApproval=true and a callback is available, the tool will request user confirmation before writing.`,
      {
        type: 'object',
        properties: {
          path: { type: 'string', description: 'Absolute file path' },
          line: { type: 'number', description: '1-based line number, or 0 to append', minimum: 0 },
          content: { type: 'string', description: 'Content to insert (may include newlines)' },
          previewOnly: { type: 'boolean' },
          requireApproval: { type: 'boolean' },
        },
        required: ['path', 'line', 'content'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const params = context.parameters as unknown as InsertContentParams

    const path = (params.path || '').toString().trim()
    if (!path) throw new ValidationError('File path cannot be empty')
    if (!isAbsolutePath(path)) throw new ValidationError('Path must be an absolute path')

    const line = Number(params.line)
    if (!Number.isFinite(line) || line < 0) throw new ValidationError('Invalid line number. Must be >= 0')

    const content = params.content ?? ''
    const previewOnly = params.previewOnly === true
    const requireApproval = params.requireApproval === true

    const exists = await filesystemApi.exists(path)
    let fileContent = ''
    let fileCreated = false

    if (!exists) {
      // 新建文件时，仅允许 line=0（末尾追加）或 line=1（文件开头）
      if (line > 1) {
        throw new ValidationError(
          `Cannot insert content at line ${line} into a non-existent file. For new files, line must be 0 (append) or 1 (beginning).`
        )
      }
      fileCreated = true
    } else {
      const isDir = await filesystemApi.isDirectory(path)
      if (isDir) throw new ValidationError(`Path ${path} is a directory, cannot insert content`)
      if (isBinaryFile(path))
        throw new ValidationError(`File ${path} appears to be binary, text insertion not supported`)

      try {
        fileContent = await readTextFile(path)
      } catch (e) {
        throw new ToolError(`Failed to read file: ${e instanceof Error ? e.message : String(e)}`)
      }
    }

    // 生成新内容（行级插入）
    const lines = fileContent ? fileContent.split('\n') : []
    const insertIndex = clamp(line === 0 ? lines.length : line - 1, 0, lines.length)
    const insertLines = content.split('\n')

    const before = lines.slice(0, insertIndex)
    const after = lines.slice(insertIndex)
    const updatedContent = [...before, ...insertLines, ...after].join('\n')

    // 预览与审批
    const previewText = `insert_content preview\nFile: ${path}\nCreated: ${fileCreated}\nInsert at line: ${line}\nInserted lines: ${insertLines.length}`

    if (previewOnly) {
      return {
        content: [{ type: 'text', text: previewText }],
        extInfo: {
          file: path,
          created: fileCreated,
          line,
          insertedLinesCount: insertLines.length,
          previewOnly: true,
        },
      }
    }

    if (requireApproval) {
      const approved = await this.askForApproval(context, previewText)
      if (!approved) {
        return {
          content: [{ type: 'text', text: 'Changes were rejected by the user.' }],
          extInfo: { approved: false, previewOnly: false },
        }
      }
    }

    // 写入
    try {
      await writeTextFile(path, updatedContent)
    } catch (e) {
      throw new ToolError(`Failed to write file: ${e instanceof Error ? e.message : String(e)}`)
    }

    return {
      content: [
        {
          type: 'text',
          text: `insert_content applied\nFile: ${path}\nInsert at line: ${line}\nInserted lines: ${insertLines.length}`,
        },
      ],
      extInfo: {
        file: path,
        created: fileCreated,
        line,
        insertedLinesCount: insertLines.length,
        previewOnly: false,
        approved: requireApproval ? true : undefined,
      },
    }
  }

  private async askForApproval(context: ToolExecutionContext, previewText: string): Promise<boolean> {
    const cb = context.agentContext.context.config.callback
    if (cb && cb.onHumanConfirm) {
      try {
        const prompt = `About to insert content. Proceed?\n\n${previewText}`
        const ok = await cb.onHumanConfirm(context.agentContext, prompt)
        return !!ok
      } catch {
        return false
      }
    }
    return false
  }
}

export const insertContentTool = new InsertContentTool()

// ===== 辅助函数 =====

function isAbsolutePath(p: string): boolean {
  return p.startsWith('/')
}

function clamp(n: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, n))
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
