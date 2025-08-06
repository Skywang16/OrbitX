/**
 * @file MemoryManager.ts
 * @description Manages the agent's memory, including chat history and working memory, with advanced capacity management and compression .
 */

import { Memory, ChatMessage, ChatMessageRole } from '../types/memory'
import { TaskContext } from '../context/TaskContext'
import { promptEngine } from '../prompt/PromptEngine'
import { llmManager } from '../llm/LLMProvider'

export interface MemoryManagerConfig {
  maxMessages?: number
  maxTokens?: number
  enableCompression?: boolean
  compressionThreshold?: number
  compressionTargetCount?: number
  compressionTriggerRatio?: number // 触发压缩的Token使用率
  enableDynamicSystemPrompt?: boolean
  enableLargeContentOptimization?: boolean
  maxLargeContentLength?: number
}

const DEFAULT_CONFIG: Required<MemoryManagerConfig> = {
  maxMessages: 20,
  maxTokens: 16000,
  enableCompression: true,
  compressionThreshold: 15,
  compressionTargetCount: 5,
  compressionTriggerRatio: 0.8, // 80%使用率时触发压缩
  enableDynamicSystemPrompt: true,
  enableLargeContentOptimization: true,
  maxLargeContentLength: 5000,
}

export class MemoryManager {
  private memory: Memory
  private config: Required<MemoryManagerConfig>
  private taskContext?: TaskContext
  private tokenCache: Map<string, number> = new Map() // Token计算缓存
  private lastSystemPromptUpdate: number = 0 // 上次系统提示更新时间

  constructor(initialMemory: Partial<Memory> = {}, config: MemoryManagerConfig = {}, taskContext?: TaskContext) {
    this.memory = {
      chatHistory: initialMemory?.chatHistory || [],
      workingMemory: initialMemory?.workingMemory || {},
    }
    this.config = { ...DEFAULT_CONFIG, ...config }
    this.taskContext = taskContext
  }

  public async addChatMessage(message: ChatMessage): Promise<void> {
    // 优化大内容
    if (this.config.enableLargeContentOptimization) {
      message = this._optimizeLargeContent(message)
    }

    this.memory.chatHistory.push(message)

    // 动态系统提示调整
    if (this.config.enableDynamicSystemPrompt && message.role === ChatMessageRole.USER) {
      await this._updateDynamicSystemPrompt()
    }

    await this.manageCapacity()
  }

  private async manageCapacity(): Promise<void> {
    const currentTokens = this._getEstimatedTokens()

    // 智能压缩触发 - 基于Token使用率
    if (
      this.config.enableCompression &&
      (currentTokens > this.config.maxTokens * this.config.compressionTriggerRatio ||
        this.memory.chatHistory.length > this.config.compressionThreshold)
    ) {
      await this.compressMessages()
    }

    // 批量删除消息而不是逐个删除
    if (this.memory.chatHistory.length > this.config.maxMessages) {
      const excess = this.memory.chatHistory.length - this.config.maxMessages
      this.memory.chatHistory.splice(1, excess) // Keep system message if present
      this._clearTokenCache() // 清除缓存
    }

    // 基于Token的批量清理
    const updatedTokens = this._getEstimatedTokens()
    if (updatedTokens > this.config.maxTokens && this.memory.chatHistory.length > 2) {
      const targetTokens = this.config.maxTokens * 0.7 // 目标70%使用率
      this._batchRemoveByTokens(targetTokens)
    }

    this._fixDiscontinuousMessages()
  }

  public async compressMessages(): Promise<void> {
    if (!this.taskContext) {
      // 静默跳过压缩，避免console警告
      return
    }

    const messagesToCompress = this.memory.chatHistory.slice(0, -this.config.compressionTargetCount)
    if (messagesToCompress.length < 2) return

    const formattedHistory = messagesToCompress.map(msg => `${msg.role}: ${msg.content}`).join('\n')

    const workflowState = this.taskContext.workflow
      ? JSON.stringify(
          {
            name: this.taskContext.workflow.name,
            agents: this.taskContext.workflow.agents.map(a => ({ id: a.id, name: a.name, status: a.status })),
            variables: Object.fromEntries(this.taskContext.variables.entries()),
          },
          null,
          2
        )
      : 'No active workflow.'

    const prompt = promptEngine.generate('memory-compression', {
      variables: {
        chatHistory: formattedHistory,
        workflowState,
      },
    })

    try {
      const llmResponse = await llmManager.call(prompt)
      const summary = llmResponse.content?.trim()

      if (summary) {
        const summaryMessage: ChatMessage = {
          role: ChatMessageRole.SYSTEM,
          content: `[Task Summary]\n${summary}`,
        }

        const remainingMessages = this.memory.chatHistory.slice(-this.config.compressionTargetCount)
        this.memory.chatHistory = [summaryMessage, ...remainingMessages]
      }
    } catch (error) {
      // 压缩失败时静默处理，避免影响主流程
      // 可以考虑添加到错误日志系统
    }
  }

  private _getEstimatedTokens(): number {
    return this.memory.chatHistory.reduce((total, message) => {
      return total + this._calculateTokens(message.content)
    }, 0)
  }

  /**
   * 区分中英文字符，提供更准确的Token估算
   */
  private _calculateTokens(content: string): number {
    if (!content) return 0

    // 使用缓存提高性能
    const cacheKey = content.length < 1000 ? content : `${content.substring(0, 100)}...${content.length}`
    if (this.tokenCache.has(cacheKey)) {
      return this.tokenCache.get(cacheKey)!
    }

    // 区分中英文字符进行Token计算
    const chineseCharCount = (content.match(/[\u4e00-\u9fff]/g) || []).length
    const otherCharCount = content.length - chineseCharCount

    // 中文字符1:1，其他字符4:1的比例
    const tokens = chineseCharCount + Math.ceil(otherCharCount / 4)

    // 缓存结果
    if (this.tokenCache.size > 1000) {
      this.tokenCache.clear() // 防止缓存过大
    }
    this.tokenCache.set(cacheKey, tokens)

    return tokens
  }

  private _fixDiscontinuousMessages(): void {
    if (
      this.memory.chatHistory.length > 1 &&
      this.memory.chatHistory[0].role !== ChatMessageRole.SYSTEM &&
      this.memory.chatHistory[0].role !== ChatMessageRole.USER
    ) {
      this.memory.chatHistory.shift()
    }
  }

  public getMemory(): Memory {
    return this.memory
  }

  public setWorkingMemory(key: string, value: unknown): void {
    this.memory.workingMemory[key] = value
  }

  /**
   * 清除Token缓存
   */
  private _clearTokenCache(): void {
    this.tokenCache.clear()
  }

  /**
   * 基于Token数量批量删除消息
   */
  private _batchRemoveByTokens(targetTokens: number): void {
    let currentTokens = this._getEstimatedTokens()
    let removeCount = 0

    // 计算需要删除的消息数量
    for (let i = 1; i < this.memory.chatHistory.length - 1 && currentTokens > targetTokens; i++) {
      const messageTokens = this._calculateTokens(this.memory.chatHistory[i].content)
      currentTokens -= messageTokens
      removeCount++
    }

    if (removeCount > 0) {
      this.memory.chatHistory.splice(1, removeCount)
      this._clearTokenCache()
    }
  }

  /**
   * 优化大内容消息
   */
  private _optimizeLargeContent(message: ChatMessage): ChatMessage {
    if (message.content.length <= this.config.maxLargeContentLength) {
      return message
    }

    // 对于过长的内容，保留开头和结尾，中间用省略号替代
    const halfLength = Math.floor(this.config.maxLargeContentLength / 2)
    const optimizedContent =
      message.content.substring(0, halfLength) +
      '\n\n[... 内容过长，已省略 ...]\n\n' +
      message.content.substring(message.content.length - halfLength)

    return {
      ...message,
      content: optimizedContent,
    }
  }

  /**
   * 动态更新系统提示
   */
  private async _updateDynamicSystemPrompt(): Promise<void> {
    const now = Date.now()

    // 限制更新频率，避免过于频繁的调用
    if (now - this.lastSystemPromptUpdate < 30000) {
      // 30秒内不重复更新
      return
    }

    this.lastSystemPromptUpdate = now

    // 如果有最新的用户消息，可以基于此调整系统提示
    const lastUserMessage = this.memory.chatHistory
      .slice()
      .reverse()
      .find(msg => msg.role === ChatMessageRole.USER)

    if (lastUserMessage && this.taskContext?.workflow) {
      // 这里可以根据最新的用户输入动态调整系统提示
      // 暂时保留接口，具体实现可以根据需求扩展
      // 动态系统提示更新逻辑
    }
  }
}
