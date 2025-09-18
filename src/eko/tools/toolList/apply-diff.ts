/**
 * apply_diff 工具
 * 支持：
 * - 多文件、多 hunk 的变更
 * - 预览模式（不落盘，仅返回预览与失败详情）
 * - 需要审批时的确认（若配置提供回调）
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@/eko-core/types'
import { filesystemApi } from '@/api'
import { ValidationError, ToolError } from '../tool-error'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'

interface DiffHunk {
  contextBefore?: string[]
  old?: string[]
  new?: string[]
  contextAfter?: string[]
  startLineHint?: number // 1-based，可选
}

// 汇总预览信息
type PerFileResult = {
  path: string
  exists: boolean
  isDirectory: boolean
  isBinary: boolean
  originalContent?: string
  newContent?: string
  hunks: Array<{
    index: number
    success: boolean
    reason?: string
    appliedAtLine?: number
    oldLineCount?: number
    newLineCount?: number
  }>
  applied: boolean
}

function buildPreviewSummary(results: PerFileResult[]): {
  previewText: string
  anyFailures: boolean
  totalApplied: number
  totalFilesChanged: number
} {
  const lines: string[] = []
  let anyFailures = false
  let totalApplied = 0
  let totalFilesChanged = 0

  for (const r of results) {
    lines.push(`File: ${r.path}`)
    lines.push(`  exists: ${r.exists}, dir: ${r.isDirectory}, binary: ${r.isBinary}`)

    if (!r.exists) {
      lines.push('  Skipped: file does not exist')
      anyFailures = true
      lines.push('')
      continue
    }
    if (r.isDirectory) {
      lines.push('  Skipped: path is a directory')
      anyFailures = true
      lines.push('')
      continue
    }
    if (r.isBinary) {
      lines.push('  Skipped: file looks binary')
      anyFailures = true
      lines.push('')
      continue
    }

    if (r.hunks.length === 0) {
      lines.push('  No hunks provided')
      lines.push('')
      continue
    }

    let appliedInThisFile = 0
    for (const h of r.hunks) {
      if (h.success) {
        lines.push(
          `  Hunk #${h.index}: applied at line ${h.appliedAtLine ?? '-'} (old ${h.oldLineCount ?? 0} -> new ${
            h.newLineCount ?? 0
          })`
        )
        totalApplied++
        appliedInThisFile++
      } else {
        lines.push(`  Hunk #${h.index}: FAILED - ${h.reason ?? 'Unknown reason'}`)
        anyFailures = true
      }
    }

    if (appliedInThisFile > 0) totalFilesChanged++
    lines.push('')
  }

  const header = `Summary: ${totalApplied} hunk(s) applied across ${totalFilesChanged} file(s).${
    anyFailures ? ' Some hunks failed or were skipped.' : ''
  }`

  const previewText = [header, '', ...lines].join('\n')
  return { previewText, anyFailures, totalApplied, totalFilesChanged }
}

interface FileDiff {
  path: string
  hunks: DiffHunk[]
}

export interface ApplyDiffParams {
  files: FileDiff[]
  previewOnly?: boolean
  requireApproval?: boolean
}

export class ApplyDiffTool extends ModifiableTool {
  constructor() {
    super(
      'apply_diff',
      `Apply multiple hunks across multiple files. Provides a preview summary and can ask for approval before writing.
Notes:
- Absolute paths are required.
- Each hunk supports optional contextBefore/contextAfter for safer matching.
- If previewOnly=true, only preview without writing.
- If requireApproval=true and a callback is available, tool will request user confirmation before writing.`,
      {
        type: 'object',
        properties: {
          files: {
            type: 'array',
            description: 'List of file diffs to apply',
            items: {
              type: 'object',
              properties: {
                path: { type: 'string', description: 'Absolute file path' },
                hunks: {
                  type: 'array',
                  items: {
                    type: 'object',
                    properties: {
                      contextBefore: { type: 'array', items: { type: 'string' } },
                      old: { type: 'array', items: { type: 'string' } },
                      new: { type: 'array', items: { type: 'string' } },
                      contextAfter: { type: 'array', items: { type: 'string' } },
                      startLineHint: { type: 'number', minimum: 1 },
                    },
                    required: [],
                  },
                },
              },
              required: ['path', 'hunks'],
            },
          },
          previewOnly: { type: 'boolean' },
          requireApproval: { type: 'boolean' },
        },
        required: ['files'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const params = context.parameters as unknown as ApplyDiffParams

    if (!Array.isArray(params.files) || params.files.length === 0) {
      throw new ValidationError('Parameter "files" must be a non-empty array')
    }

    const previewOnly = params.previewOnly === true
    const requireApproval = params.requireApproval === true

    const perFileResults: Array<{
      path: string
      exists: boolean
      isDirectory: boolean
      isBinary: boolean
      originalContent?: string
      newContent?: string
      hunks: Array<{
        index: number
        success: boolean
        reason?: string
        appliedAtLine?: number // 1-based
        oldLineCount?: number
        newLineCount?: number
      }>
      applied: boolean
    }> = []

    // 预处理并尝试应用补丁到内存
    for (const file of params.files) {
      const path = (file.path || '').toString().trim()
      if (!path) {
        throw new ValidationError('File path cannot be empty in files[]')
      }
      if (!isAbsolutePath(path)) {
        throw new ValidationError(`Path must be absolute: ${path}`)
      }

      const exists = await filesystemApi.exists(path)
      const isDirectory = exists ? await filesystemApi.isDirectory(path) : false
      const isBinary = exists && !isDirectory ? isBinaryFile(path) : false

      const fileResult = {
        path,
        exists,
        isDirectory,
        isBinary,
        originalContent: undefined as string | undefined,
        newContent: undefined as string | undefined,
        hunks: [] as Array<{
          index: number
          success: boolean
          reason?: string
          appliedAtLine?: number
          oldLineCount?: number
          newLineCount?: number
        }>,
        applied: false,
      }

      if (!exists) {
        perFileResults.push(fileResult)
        continue
      }
      if (isDirectory) {
        perFileResults.push(fileResult)
        continue
      }
      if (isBinary) {
        perFileResults.push(fileResult)
        continue
      }

      let originalContent: string
      try {
        originalContent = await readTextFile(path)
      } catch (e) {
        throw new ToolError(`Failed to read file: ${e instanceof Error ? e.message : String(e)}`)
      }

      const originalLines = originalContent.split('\n')
      let workingLines = originalLines.slice()

      for (let i = 0; i < file.hunks.length; i++) {
        const hunk = file.hunks[i]
        const oldLines = hunk.old ?? []
        const newLines = hunk.new ?? []
        const before = hunk.contextBefore ?? []
        const after = hunk.contextAfter ?? []

        const match = findHunkIndex(workingLines, oldLines, before, after, hunk.startLineHint)
        if (match === -1) {
          fileResult.hunks.push({ index: i, success: false, reason: 'No matching block found' })
          continue
        }

        // 应用替换：用 newLines 替换 oldLines
        const appliedAtLine = match + 1 // 1-based
        const oldCount = oldLines.length
        workingLines.splice(match, oldCount, ...newLines)
        fileResult.hunks.push({
          index: i,
          success: true,
          appliedAtLine,
          oldLineCount: oldLines.length,
          newLineCount: newLines.length,
        })
      }

      const newContent = workingLines.join('\n')
      fileResult.originalContent = originalContent
      fileResult.newContent = newContent
      fileResult.applied = fileResult.hunks.some(h => h.success)
      perFileResults.push(fileResult)
    }

    // 生成预览摘要
    const { previewText, anyFailures, totalApplied, totalFilesChanged } = buildPreviewSummary(perFileResults)

    // 仅预览则返回
    if (previewOnly) {
      return {
        content: [
          {
            type: 'text',
            text: `apply_diff preview\n${previewText}`,
          },
        ],
        extInfo: {
          files: perFileResults,
          totalApplied,
          totalFilesChanged,
          failures: anyFailures,
          previewOnly: true,
        },
      }
    }

    // 审批
    if (requireApproval) {
      const approved = await this.askForApproval(context, previewText)
      if (!approved) {
        return {
          content: [{ type: 'text', text: 'Changes were rejected by the user.' }],
          extInfo: { approved: false, previewOnly: false },
        }
      }
    }

    // 写入文件（仅对成功有新内容的文件）
    for (const r of perFileResults) {
      if (!r.exists || r.isDirectory || r.isBinary) continue
      if (!r.newContent || r.newContent === r.originalContent) continue
      try {
        await writeTextFile(r.path, r.newContent)
      } catch (e) {
        throw new ToolError(`Failed to write file: ${e instanceof Error ? e.message : String(e)}`)
      }
    }

    return {
      content: [
        {
          type: 'text',
          text: `apply_diff applied\n${previewText}`,
        },
      ],
      extInfo: {
        files: perFileResults,
        totalApplied,
        totalFilesChanged,
        failures: anyFailures,
        previewOnly: false,
        approved: requireApproval ? true : undefined,
      },
    }
  }

  private async askForApproval(context: ToolExecutionContext, previewText: string): Promise<boolean> {
    const cb = context.agentContext.context.config.callback
    if (cb && cb.onHumanConfirm) {
      try {
        const prompt = `About to apply the following changes. Proceed?\n\n${previewText}`
        const ok = await cb.onHumanConfirm(context.agentContext, prompt)
        return !!ok
      } catch {
        return false
      }
    }
    // 如果没有可用回调，则默认不批准，提示外层使用 human_interact 工具
    return false
  }
}

export const applyDiffTool = new ApplyDiffTool()

// ===== 辅助函数 =====

function isAbsolutePath(p: string): boolean {
  return p.startsWith('/')
}

function findHunkIndex(
  workingLines: string[],
  oldLines: string[],
  contextBefore: string[],
  contextAfter: string[],
  startLineHint?: number
): number {
  // 纯插入：oldLines 为空
  if (oldLines.length === 0) {
    if (startLineHint && startLineHint > 0) {
      const idx = Math.min(Math.max(startLineHint - 1, 0), workingLines.length)
      return idx
    }
    // 无 hint，则插入到末尾
    return workingLines.length
  }

  // 遍历查找 oldLines 块
  for (let i = 0; i <= workingLines.length - oldLines.length; i++) {
    // 如果有 startLineHint，且 i 远离 hint，可优化跳过
    if (startLineHint && i < startLineHint - 100 && i + oldLines.length < startLineHint - 100) {
      // 简单跳过远区域，提高粗略性能（可选）
    }

    let matchOld = true
    for (let j = 0; j < oldLines.length; j++) {
      if (workingLines[i + j] !== oldLines[j]) {
        matchOld = false
        break
      }
    }
    if (!matchOld) continue

    // 校验 contextBefore
    if (contextBefore.length > 0) {
      const start = i - contextBefore.length
      if (start < 0) continue
      let ok = true
      for (let k = 0; k < contextBefore.length; k++) {
        if (workingLines[start + k] !== contextBefore[k]) {
          ok = false
          break
        }
      }
      if (!ok) continue
    }

    // 校验 contextAfter
    if (contextAfter.length > 0) {
      const start = i + oldLines.length
      const end = start + contextAfter.length
      if (end > workingLines.length) continue
      let ok = true
      for (let k = 0; k < contextAfter.length; k++) {
        if (workingLines[start + k] !== contextAfter[k]) {
          ok = false
          break
        }
      }
      if (!ok) continue
    }

    return i
  }

  return -1
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
