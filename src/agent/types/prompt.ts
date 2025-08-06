/**
 * 提示词相关类型定义
 */

/**
 * 提示词模板变量
 */
export interface PromptVariable {
  name: string
  type: 'string' | 'number' | 'boolean' | 'object' | 'array'
  description: string
  required?: boolean
  default?: unknown
}

/**
 * 提示词模板
 */
export interface PromptTemplate {
  id: string
  name: string
  description?: string
  template: string
  variables: PromptVariable[]
  outputFormat?: 'text' | 'json' | 'markdown'
  examples?: Array<{
    input: Record<string, unknown>
    output: string
  }>
}

/**
 * 提示词生成选项
 */
export interface PromptGenerationOptions {
  variables: Record<string, unknown>
  outputFormat?: 'text' | 'json' | 'markdown'
  includeSystemPrompt?: boolean
  includeExamples?: boolean
}
