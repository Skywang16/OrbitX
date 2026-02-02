/**
 * Markdown 渲染器单例
 */

import hljs from 'highlight.js'
import { marked } from 'marked'
import { markedHighlight } from 'marked-highlight'

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

// 配置 marked 渲染选项
const renderer = new marked.Renderer()

// 自定义代码块渲染，增加 Header 结构
renderer.code = ({ text, lang }: { text: string; lang?: string }) => {
  const language = lang || 'text'
  // 此时 text 已经是经过 marked-highlight 高亮过的 HTML（如果配置了的话）
  // 但我们需要注意的是，如果我们覆盖了 renderer.code，我们需要确保 highlight 依然生效
  // 实际上 marked-highlight 是通过 hook 修改 token 的，所以传入这里的 text 已经是高亮过的 HTML

  return `
      <div class="code-block-wrapper">
        <div class="code-block-header">
        <span class="code-lang">${language}</span>
        <button class="code-copy-btn" aria-label="Copy code">
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
        </button>
      </div>
      <pre><code class="hljs language-${language}">${text}</code></pre>
    </div>
  `
}

// 自定义表格渲染，增加包裹容器以支持滚动和样式
// eslint-disable-next-line @typescript-eslint/no-explicit-any
renderer.table = (token: any) => {
  const header = token.header || ''
  const body = token.body || ''
  return `
    <div class="table-wrapper">
      <table>
        <thead>${header}</thead>
        <tbody>${body}</tbody>
      </table>
    </div>
  `
}

marked.use({
  renderer,
  breaks: true,
  gfm: true,
})

/**
 * 渲染 Markdown 内容
 * @param content Markdown 文本
 * @returns 渲染后的 HTML 字符串
 */
export const renderMarkdown = (content?: string): string => {
  return marked.parse(content || '') as string
}
