/**
 * Markdown 渲染器单例
 */

import { marked } from 'marked'
import { markedHighlight } from 'marked-highlight'
import hljs from 'highlight.js'

// 配置 marked（只执行一次）
marked.use(
  markedHighlight({
    langPrefix: 'hljs language-',
    highlight(code, lang) {
      const language = hljs.getLanguage(lang) ? lang : 'plaintext'
      return hljs.highlight(code, { language }).value
    },
  })
)

/**
 * 渲染 Markdown 内容
 * @param content Markdown 文本
 * @returns 渲染后的 HTML 字符串
 */
export const renderMarkdown = (content?: string): string => {
  return marked.parse(content || '') as string
}
