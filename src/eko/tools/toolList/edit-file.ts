/**
 * 文件编辑工具
 * 支持：
 * - 字面量或正则匹配
 * - 大小写可选
 * - 可选的行范围（1-based，闭区间）
 * - 预览模式（不落盘，仅返回替换统计与命中行号）
 * - 结构化 extInfo 返回详细信息
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@/eko-core/types'
import { writeTextFile, readTextFile } from '@tauri-apps/plugin-fs'
import { filesystemApi } from '@/api'
import { ValidationError, FileNotFoundError, ToolError } from '../tool-error'

export interface SearchAndReplaceParams {
  path: string
  // 新参数名（优先）
  search?: string
  replace?: string
  // 兼容旧参数名
  oldString?: string
  newString?: string
  // 行为控制
  useRegex?: boolean
  ignoreCase?: boolean
  startLine?: number // 1-based
  endLine?: number // 1-based，闭区间
  previewOnly?: boolean
}

/**
 * 文件编辑工具 - 支持多种编辑模式
 */
export class EditFileTool extends ModifiableTool {
  constructor() {
    super(
      'edit_file',
      `Find and replace file contents with optional regex and line range.
Behavior:
- Supports literal or regex matching, optional ignore-case
- Can restrict replacement to a specific 1-based line range [startLine, endLine]
- If previewOnly = true, will NOT write changes, only report match lines and replacement count
- If no match is found, returns a success message "No changes needed" and does not write
Notes:
- Absolute paths are required.
- Prefer using read_file to inspect context before replacing.
- Compatible with legacy parameters oldString/newString (alias of search/replace).`,
      {
        type: 'object',
        properties: {
          path: {
            type: 'string',
            description:
              'Absolute path to the file. Must be a complete path, for example: "/Users/user/project/src/main.ts", "/home/user/workspace/config.json"',
          },
          // 新参数命名（推荐）
          search: {
            type: 'string',
            description: 'Search string (literal or regex when useRegex=true).',
          },
          replace: {
            type: 'string',
            description: 'Replacement string (can be empty).',
          },
          // 兼容旧参数命名
          oldString: {
            type: 'string',
            description: 'Deprecated: same as search.',
          },
          newString: {
            type: 'string',
            description: 'Deprecated: same as replace.',
          },
          useRegex: {
            type: 'boolean',
            description: 'Enable regex matching. Default: false (literal match).',
          },
          ignoreCase: {
            type: 'boolean',
            description: 'Case-insensitive matching. Default: false.',
          },
          startLine: {
            type: 'number',
            description: 'Optional: 1-based start line (inclusive). If set, restricts replacement to this range.',
            minimum: 1,
          },
          endLine: {
            type: 'number',
            description: 'Optional: 1-based end line (inclusive). Must be >= startLine when both provided.',
            minimum: 1,
          },
          previewOnly: {
            type: 'boolean',
            description: 'If true, only preview replacement stats without writing changes.',
          },
        },
        // 仅强制要求 path，其余在运行时做更灵活的校验（兼容新旧参数名）
        required: ['path'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const params = context.parameters as unknown as SearchAndReplaceParams

    // 基本校验与参数解析
    const path = (params.path || '').toString().trim()
    if (!path) {
      throw new ValidationError('File path cannot be empty')
    }
    if (!isAbsolutePath(path)) {
      throw new ValidationError('Path must be an absolute path')
    }

    // 兼容参数名
    const search = params.search ?? params.oldString
    const replace = params.replace ?? params.newString

    if (search === undefined) {
      throw new ValidationError('Missing required parameter: search (or oldString)')
    }
    if (replace === undefined) {
      throw new ValidationError('Missing required parameter: replace (or newString)')
    }

    const useRegex = params.useRegex === true
    const ignoreCase = params.ignoreCase === true
    const previewOnly = params.previewOnly === true

    // 行范围校验（1-based）
    const hasRange = params.startLine !== undefined || params.endLine !== undefined
    const startLine = params.startLine !== undefined ? Math.max(1, Math.floor(params.startLine)) : undefined
    const endLine = params.endLine !== undefined ? Math.max(1, Math.floor(params.endLine)) : undefined
    if (startLine !== undefined && endLine !== undefined && endLine < startLine) {
      throw new ValidationError('endLine must be greater than or equal to startLine')
    }

    // 文件校验
    const exists = await filesystemApi.exists(path)
    if (!exists) {
      throw new FileNotFoundError(path)
    }
    const isDir = await filesystemApi.isDirectory(path)
    if (isDir) {
      throw new ValidationError(`Path ${path} is a directory, cannot perform content replacement`)
    }
    if (isBinaryFile(path)) {
      throw new ValidationError(`File ${path} appears to be binary, text replacement not supported`)
    }

    // 读取原始内容
    let originalContent: string
    try {
      originalContent = await readTextFile(path)
    } catch (e) {
      throw new ToolError(`Failed to read file: ${e instanceof Error ? e.message : String(e)}`)
    }

    // 目标文本（可能是范围内的子串）
    const lines = originalContent.split('\n')
    const totalLines = lines.length

    let targetStart = 0
    let targetEnd = totalLines - 1
    if (hasRange) {
      targetStart = Math.max(0, (startLine ?? 1) - 1)
      targetEnd = Math.min(totalLines - 1, (endLine ?? totalLines) - 1)
    }

    const before = lines.slice(0, targetStart)
    const target = lines.slice(targetStart, targetEnd + 1).join('\n')
    const after = lines.slice(targetEnd + 1)

    // 构造匹配模式
    const flags = ignoreCase ? 'gi' : 'g'
    const pattern = useRegex ? new RegExp(search as string, flags) : new RegExp(escapeRegExp(search as string), flags)

    // 统计匹配（避免直接替换造成统计困难）
    const matchCount = countMatches(target, pattern)

    if (matchCount === 0) {
      const message = `No changes needed for '${path}' (0 matches).`
      return {
        content: [
          {
            type: 'text',
            text: message,
          },
        ],
        extInfo: {
          file: path,
          replacedCount: 0,
          useRegex,
          ignoreCase,
          startLine: startLine ?? null,
          endLine: endLine ?? null,
          previewOnly,
        },
      }
    }

    // 进行替换
    const modifiedTarget = (target as string).replace(pattern, replace as string)
    const modifiedContent = [...before, modifiedTarget, ...after].join('\n')

    // 预览模式：仅返回统计与命中行信息
    const matchedLines = computeMatchLineNumbers(target, pattern, hasRange ? targetStart + 1 : 1)

    // 为前端diff显示准备old和new内容
    const firstMatch = pattern.exec(target)
    const oldContent = firstMatch ? firstMatch[0] : (search as string)
    const newContent = replace as string

    if (previewOnly) {
      const message = `Preview: ${matchCount} replacement(s) will be made in '${path}'.
Lines affected (first 50): ${matchedLines.slice(0, 50).join(', ')}`
      return {
        content: [
          {
            type: 'text',
            text: message,
          },
        ],
        extInfo: {
          file: path,
          replacedCount: matchCount,
          affectedLines: matchedLines,
          useRegex,
          ignoreCase,
          startLine: startLine ?? null,
          endLine: endLine ?? null,
          previewOnly: true,
          old: oldContent,
          new: newContent,
        },
      }
    }

    // 写入文件
    try {
      await writeTextFile(path, modifiedContent)
    } catch (e) {
      throw new ToolError(`Failed to write file: ${e instanceof Error ? e.message : String(e)}`)
    }

    const message = `File edited successfully: ${path}
Status: ${matchCount} replacement(s) applied.`

    return {
      content: [
        {
          type: 'text',
          text: message,
        },
      ],
      extInfo: {
        file: path,
        replacedCount: matchCount,
        affectedLines: matchedLines,
        useRegex,
        ignoreCase,
        startLine: startLine ?? null,
        endLine: endLine ?? null,
        previewOnly: false,
        old: oldContent,
        new: newContent,
      },
    }
  }
}

export const editFileTool = new EditFileTool()

// ===== 工具函数 =====

function isAbsolutePath(p: string): boolean {
  // macOS/Linux: 以 / 开头；简单判断即可
  return p.startsWith('/')
}

function escapeRegExp(input: string): string {
  return input.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function countMatches(text: string, regex: RegExp): number {
  if (!regex.global) {
    // 确保是全局匹配
    const flags = (regex.ignoreCase ? 'i' : '') + (regex.multiline ? 'm' : '') + (regex.dotAll ? 's' : '')
    regex = new RegExp(regex.source, 'g' + flags)
  }
  let count = 0
  let m: RegExpExecArray | null
  while ((m = regex.exec(text)) !== null) {
    count++
    if (m[0].length === 0) {
      // 防止零宽匹配死循环
      regex.lastIndex++
    }
  }
  return count
}

function computeMatchLineNumbers(text: string, regex: RegExp, baseLine: number): number[] {
  const lines: number[] = []
  if (!regex.global) {
    const flags = (regex.ignoreCase ? 'i' : '') + (regex.multiline ? 'm' : '') + (regex.dotAll ? 's' : '')
    regex = new RegExp(regex.source, 'g' + flags)
  }
  let m: RegExpExecArray | null
  while ((m = regex.exec(text)) !== null) {
    const before = text.slice(0, m.index)
    const line = baseLine + (before.split('\n').length - 1)
    lines.push(line)
    if (m[0].length === 0) regex.lastIndex++
  }
  return lines
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
