/**
 * CK API 类型定义
 */

export interface CkSearchResult {
  path: string
  span: {
    byte_start: number
    byte_end: number
    line_start: number
    line_end: number
  }
  language: string
  snippet: string
  score?: number
}

export interface CkBuildProgress {
  /** 当前处理的文件名 */
  currentFile?: string
  /** 已完成的文件数 */
  filesCompleted: number
  /** 总文件数 */
  totalFiles: number
  /** 当前文件的块数 */
  currentFileChunks?: number
  /** 总块数 */
  totalChunks: number
  /** 是否完成 */
  isComplete: boolean
  /** 错误信息 */
  error?: string
}
