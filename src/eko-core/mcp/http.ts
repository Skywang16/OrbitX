import Log from '../common/log'
import { uuidv4 } from '../common/utils'
import { ToolResult, IMcpClient, McpCallToolParam, McpListToolParam, McpListToolResult } from '../types'

type SseEventData = {
  id?: string
  event?: string
  data?: string
  [key: string]: unknown
}

type McpResponse<T = unknown> = {
  id: string
  result?: T & { isError?: boolean; content?: unknown }
  error?: string | { message: string }
}

export class SimpleHttpMcpClient implements IMcpClient {
  private httpUrl: string
  private clientName: string
  private headers: Record<string, string>
  private protocolVersion: string = '2025-06-18'
  private connected: boolean = false
  private mcpSessionId?: string | null // Mcp-Session-Id

  constructor(httpUrl: string, clientName: string = 'EkoMcpClient', headers: Record<string, string> = {}) {
    this.httpUrl = httpUrl
    this.clientName = clientName
    this.headers = headers
  }

  async connect(signal?: AbortSignal): Promise<void> {
    Log.info('MCP Client, connecting...', this.httpUrl)
    this.mcpSessionId = null
    await this.request(
      'initialize',
      {
        protocolVersion: this.protocolVersion,
        capabilities: {
          tools: {
            listChanged: true,
          },
          sampling: {},
        },
        clientInfo: {
          name: this.clientName,
          version: '1.0.0',
        },
      },
      signal
    )
    this.connected = true
  }

  async listTools(param: McpListToolParam, signal?: AbortSignal): Promise<McpListToolResult> {
    const message = await this.request<{ tools?: McpListToolResult }>(
      'tools/list',
      {
        ...param,
      },
      signal
    )
    return message.result?.tools || []
  }

  async callTool(param: McpCallToolParam, signal?: AbortSignal): Promise<ToolResult> {
    const message = await this.request<ToolResult>(
      'tools/call',
      {
        ...param,
      },
      signal
    )
    return message.result as ToolResult
  }

  isConnected(): boolean {
    return this.connected
  }

  async close(): Promise<void> {
    this.connected = false
    if (this.mcpSessionId) {
      this.request('notifications/cancelled', {
        requestId: uuidv4(),
        reason: 'User requested cancellation',
      })
      this.mcpSessionId = null
    }
  }

  async request<T = unknown>(
    method: string,
    params: Record<string, unknown>,
    signal?: AbortSignal
  ): Promise<McpResponse<T>> {
    try {
      const id = uuidv4()
      const extHeaders: Record<string, string> = {}
      if (this.mcpSessionId && method !== 'initialize') {
        extHeaders['Mcp-Session-Id'] = this.mcpSessionId
      }
      const response = await fetch(this.httpUrl, {
        method: 'POST',
        headers: {
          'Cache-Control': 'no-cache',
          'Content-Type': 'application/json',
          Accept: 'application/json, text/event-stream',
          'MCP-Protocol-Version': this.protocolVersion,
          ...extHeaders,
          ...this.headers,
        },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: id,
          method: method,
          params: {
            ...params,
          },
        }),
        keepalive: true,
        signal: signal,
      })
      if (method == 'initialize') {
        this.mcpSessionId = response.headers.get('Mcp-Session-Id') || response.headers.get('mcp-session-id')
      }
      const contentType =
        response.headers.get('Content-Type') || response.headers.get('content-type') || 'application/json'
      if (typeof contentType === 'string' && contentType.includes('text/event-stream')) {
        // SSE
        if (!response.body) {
          throw new Error('Readable stream is not supported by the environment')
        }
        const reader = response.body.getReader() as ReadableStreamDefaultReader
        let str = ''
        let message: McpResponse<T> | undefined
        const decoder = new TextDecoder()
        while (true) {
          const { value, done } = await reader.read()
          if (done) {
            break
          }
          const text = decoder.decode(value)
          str += text
          if (str.indexOf('\n\n') > -1) {
            const chunks = str.split('\n\n')
            for (let i = 0; i < chunks.length - 1; i++) {
              const chunk = chunks[i]
              const chunkData = this.parseChunk(chunk)
              if (chunkData.event == 'message') {
                const parsed = JSON.parse(chunkData.data as string) as McpResponse<T>
                if (parsed.id == id) {
                  return parsed
                }
              }
            }
            str = chunks[chunks.length - 1]
          }
        }
        if (!message) {
          throw new Error(`MCP ${method} error: no response`)
        }
        this.handleError(method, message)
        return message
      } else {
        // JSON
        const message = (await response.json()) as McpResponse<T>
        this.handleError(method, message)
        return message
      }
    } catch (e) {
      const err = e as { name?: string }
      if (err?.name !== 'AbortError') {
        Log.error('MCP Client, connectSse error:', e instanceof Error ? e : String(e))
      }
      throw e
    }
  }

  private handleError(method: string, message: McpResponse<unknown>) {
    if (!message) {
      throw new Error(`MCP ${method} error: no response`)
    }
    if (message?.error) {
      Log.error(`MCP ${method} error: ` + message.error)
      throw new Error(
        `MCP ${method} error: ` + (typeof message.error === 'string' ? message.error : message.error.message)
      )
    }
    if (message.result?.isError == true) {
      const content = message.result.content
      if (typeof content === 'string') {
        throw new Error(`MCP ${method} error: ` + content)
      }
      throw new Error(`MCP ${method} error: ` + JSON.stringify(message.result))
    }
  }

  parseChunk(chunk: string): SseEventData {
    const lines = chunk.split('\n')
    const chunk_obj: SseEventData = {}
    for (let j = 0; j < lines.length; j++) {
      const line = lines[j]
      if (line.startsWith('id:')) {
        chunk_obj['id'] = line.substring(3).trim()
      } else if (line.startsWith('event:')) {
        chunk_obj['event'] = line.substring(6).trim()
      } else if (line.startsWith('data:')) {
        chunk_obj['data'] = line.substring(5).trim()
      } else {
        const idx = line.indexOf(':')
        if (idx > -1) {
          chunk_obj[line.substring(0, idx)] = line.substring(idx + 1).trim()
        }
      }
    }
    return chunk_obj
  }
}
