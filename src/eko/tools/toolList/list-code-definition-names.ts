/**
 * list_code_definition_names 工具
 *
 * 目前支持 TypeScript/JavaScript（.ts/.tsx/.js/.jsx），递归遍历，
 * 提取函数、类、接口、类型别名、枚举、导出默认等定义名称与行号。
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@/eko-core/types'
import { terminalContextApi } from '@/api'
import { codeApi, type CodeDefinition } from '@/api'
import { ValidationError } from '../tool-error'

export interface ListCodeDefNamesParams {
  path: string // 绝对或相对路径：文件或目录
}

type DefinitionItem = CodeDefinition

export class ListCodeDefinitionNamesTool extends ModifiableTool {
  constructor() {
    super(
      'list_code_definition_names',
      `List definition names (classes, functions, methods, etc.) from source code. Analyzes either a single file or all files in a directory. Provides insights into codebase structure and important constructs for understanding the overall architecture.`,
      {
        type: 'object',
        properties: {
          path: { type: 'string', description: 'File or directory path (relative or absolute)' },
        },
        required: ['path'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const p = context.parameters as unknown as ListCodeDefNamesParams

    const inputPath = (p.path || '').toString().trim()
    if (!inputPath) throw new ValidationError('Path cannot be empty')
    const root = await resolveToAbsolute(inputPath)

    // 强制走后端解析：tree-sitter/oxc 由后端实现
    const defs: DefinitionItem[] = await codeApi.listDefinitionNames({ path: root })

    // 构建文本输出
    const header = `Found ${defs.length} definition(s)`
    const body = defs
      .slice(0, defs.length)
      .map(
        d => `${d.kind}${d.exported ? ' export' : ''}${d.isDefault ? ' default' : ''} ${d.name} @ ${d.file}:L${d.line}`
      )
      .join('\n')

    return {
      content: [
        {
          type: 'text',
          text: `${header}${defs.length ? '\n' + body : ''}`,
        },
      ],
      extInfo: {
        count: defs.length,
        definitions: defs,
      },
    }
  }
  // 本地正则解析实现已移除：统一由后端提供语法解析
}

export const listCodeDefinitionNamesTool = new ListCodeDefinitionNamesTool()

// ===== 工具函数 =====
function isAbsolutePath(p: string): boolean {
  return p.startsWith('/')
}

async function resolveToAbsolute(input: string): Promise<string> {
  if (isAbsolutePath(input)) return input
  try {
    const cwd = await terminalContextApi.getCurrentWorkingDirectory()
    if (!cwd) throw new Error('No active terminal CWD')
    return (cwd.endsWith('/') ? cwd : cwd + '/') + input.replace(/^\.\//, '')
  } catch (e) {
    throw new ValidationError(
      `Cannot resolve relative path '${input}'. Please provide an absolute path or set an active terminal with a working directory.`
    )
  }
}

// 本地 .gitignore / glob 过滤逻辑已移除：交由后端处理
