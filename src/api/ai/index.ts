/**
 * AI功能相关的API接口
 */

import type {
  AIHealthStatus,
  AIModelConfig,
  AIResponse,
  AISettings,
  AIStats,
  ChatMessage,
  CommandExplanation,
  ErrorAnalysis,
  StreamCallback,
  StreamChunk,
} from '@/types'
import { invoke } from '@/utils/request'
import { Channel } from '@tauri-apps/api/core'

import { handleError } from '../../utils/errorHandler'

/**
 * AI管理API类
 * 提供AI相关的功能和管理操作，包括模型管理、聊天、补全等功能
 */
export class AIAPI {
  // ===== AI配置管理 =====

  /**
   * 获取所有AI模型配置
   * @returns 返回所有已配置的AI模型列表
   */
  async getModels(): Promise<AIModelConfig[]> {
    try {
      return await invoke<AIModelConfig[]>('get_ai_models')
    } catch (error) {
      throw new Error(handleError(error, '获取AI模型失败'))
    }
  }

  /**
   * 添加AI模型配置
   * @param config AI模型配置对象，包含模型ID、名称、API密钥等信息
   */
  async addModel(config: AIModelConfig): Promise<void> {
    try {
      await invoke('add_ai_model', { config })
    } catch (error) {
      throw new Error(handleError(error, '添加AI模型失败'))
    }
  }

  /**
   * 更新AI模型配置
   * @param modelId 要更新的模型ID
   * @param updates 要更新的配置项（部分更新）
   */
  async updateModel(modelId: string, updates: Partial<AIModelConfig>): Promise<void> {
    try {
      await invoke('update_ai_model', { modelId, updates })
    } catch (error) {
      throw new Error(handleError(error, '更新AI模型失败'))
    }
  }

  /**
   * 删除AI模型配置
   * @param modelId 要删除的模型ID
   */
  async removeModel(modelId: string): Promise<void> {
    try {
      await invoke('remove_ai_model', { modelId })
    } catch (error) {
      throw new Error(handleError(error, '删除AI模型失败'))
    }
  }

  /**
   * 设置默认AI模型
   * @param modelId 要设置为默认的模型ID
   */
  async setDefaultModel(modelId: string): Promise<void> {
    try {
      await invoke('set_default_ai_model', { modelId })
    } catch (error) {
      throw new Error(handleError(error, '设置默认模型失败'))
    }
  }

  /**
   * 测试AI模型连接
   * @param modelId 要测试的模型ID
   * @returns 连接是否成功
   */
  async testConnection(modelId: string): Promise<boolean> {
    try {
      return await invoke('test_ai_connection', { modelId })
    } catch (error) {
      const errorMessage = handleError(error)
      // 处理未实现的适配器
      if (errorMessage.includes('not implemented yet')) {
        return false
      }
      throw new Error(errorMessage)
    }
  }

  // ===== AI功能接口 =====

  /**
   * 发送聊天消息（一次性返回完整响应）
   * @param message 用户输入的聊天消息
   * @param modelId 可选的模型ID，不指定则使用默认模型
   * @returns 返回AI的完整响应
   */
  async sendChatMessage(message: string, modelId?: string): Promise<AIResponse> {
    try {
      return await invoke('send_chat_message', { message, modelId })
    } catch (error) {
      throw new Error(handleError(error, '发送聊天消息失败'))
    }
  }

  /**
   * 流式聊天消息（使用Channel实时接收响应片段）
   * @param message 用户输入的聊天消息
   * @param callback 处理流式数据的回调函数
   * @param modelId 可选的模型ID，不指定则使用默认模型
   */
  async streamChatMessageWithChannel(message: string, callback: StreamCallback, modelId?: string): Promise<void> {
    try {
      // 创建Channel用于接收流式数据
      const channel = new Channel<StreamChunk>()

      // 设置Channel消息处理器
      channel.onmessage = (chunk: StreamChunk) => {
        callback({
          content: chunk.content,
          isComplete: chunk.isComplete,
          metadata: chunk.metadata,
        })
      }

      // 调用后端命令，传递Channel，设置无超时限制
      await invoke(
        'stream_chat_message_with_channel',
        {
          message,
          modelId,
          channel,
        },
        { timeout: 0 }
      ) // 0 表示无超时限制
    } catch (error) {
      throw new Error(handleError(error, 'Channel流式聊天失败'))
    }
  }

  /**
   * 可取消的流式聊天消息（支持中途取消请求）
   * @param message 用户输入的聊天消息
   * @param callback 处理流式数据的回调函数
   * @param modelId 可选的模型ID，不指定则使用默认模型
   * @returns 返回包含取消函数的对象
   */
  async streamChatMessageCancellable(
    message: string,
    callback: StreamCallback,
    modelId?: string
  ): Promise<{ cancel: () => void }> {
    let cancelled = false // 取消标志
    const cancel = () => {
      cancelled = true
    }

    try {
      // 创建Channel用于接收流式数据
      const channel = new Channel<StreamChunk>()

      // 设置Channel消息处理器
      channel.onmessage = (chunk: StreamChunk) => {
        // 检查是否已取消
        if (cancelled) {
          return
        }

        callback({
          content: chunk.content,
          isComplete: chunk.isComplete,
          metadata: chunk.metadata,
        })
      }

      // 调用后端命令，传递Channel，设置无超时限制
      await invoke(
        'stream_chat_message_with_channel',
        {
          message,
          modelId,
          channel,
        },
        { timeout: 0 }
      ) // 0 表示无超时限制

      return { cancel }
    } catch (error) {
      throw new Error(handleError(error, '可取消流式聊天失败'))
    }
  }

  /**
   * 解释命令功能
   * @param command 要解释的命令字符串
   * @param context 可选的上下文信息
   * @returns 返回命令的详细解释
   */
  async explainCommand(command: string, context?: Record<string, unknown>): Promise<CommandExplanation> {
    try {
      return await invoke('explain_command', { command, context })
    } catch (error) {
      throw new Error(handleError(error, '解释命令失败'))
    }
  }

  /**
   * 分析错误并提供解决方案
   * @param error 错误信息
   * @param command 导致错误的命令
   * @param context 可选的上下文信息
   * @returns 返回错误分析和解决建议
   */
  async analyzeError(error: string, command: string, context?: Record<string, unknown>): Promise<ErrorAnalysis> {
    try {
      return await invoke('analyze_error', { error, command, context })
    } catch (err) {
      throw new Error(handleError(err, '分析错误失败'))
    }
  }

  // ===== AI设置管理 =====

  /**
   * 获取AI设置配置
   * @returns 返回当前的AI设置配置
   */
  async getSettings(): Promise<AISettings> {
    try {
      return await invoke('get_ai_settings')
    } catch (error) {
      throw new Error(handleError(error, '获取AI设置失败'))
    }
  }

  /**
   * 更新AI设置配置
   * @param settings 要更新的设置项（部分更新）
   */
  async updateSettings(settings: Partial<AISettings>): Promise<void> {
    try {
      return await invoke('update_ai_settings', { settings })
    } catch (error) {
      throw new Error(handleError(error, '更新AI设置失败'))
    }
  }

  // ===== 统计和监控 =====

  /**
   * 获取AI使用统计数据
   * @returns 返回AI功能的使用统计信息
   */
  async getStats(): Promise<AIStats> {
    try {
      return await invoke('get_ai_stats')
    } catch (error) {
      throw new Error(handleError(error, '获取AI统计失败'))
    }
  }

  /**
   * 获取AI服务健康状态
   * @returns 返回所有AI模型的健康状态列表
   */
  async getHealthStatus(): Promise<AIHealthStatus[]> {
    try {
      return await invoke('get_ai_health_status')
    } catch (error) {
      throw new Error(handleError(error, '获取AI健康状态失败'))
    }
  }

  // ===== 聊天历史管理 =====

  /**
   * 获取聊天历史记录
   * @param sessionId 可选的会话ID，不指定则获取默认会话历史
   * @returns 返回聊天消息列表
   */
  async getChatHistory(sessionId?: string): Promise<ChatMessage[]> {
    try {
      return await invoke('get_chat_history', { sessionId })
    } catch (error) {
      throw new Error(handleError(error, '获取聊天历史失败'))
    }
  }

  async getChatSessions(): Promise<string[]> {
    try {
      return await invoke('get_chat_sessions')
    } catch (error) {
      throw new Error(handleError(error, '获取会话列表失败'))
    }
  }

  /**
   * 保存聊天历史记录
   * @param messages 要保存的聊天消息列表
   * @param sessionId 可选的会话ID，不指定则保存到默认会话
   * @returns 返回会话ID
   */
  async saveChatHistory(messages: ChatMessage[], sessionId?: string): Promise<string> {
    try {
      return await invoke('save_chat_history', { messages, sessionId })
    } catch (error) {
      throw new Error(handleError(error, '保存聊天历史失败'))
    }
  }

  /**
   * 清除聊天历史记录
   * @param sessionId 可选的会话ID，不指定则清除默认会话历史
   */
  async clearChatHistory(sessionId?: string): Promise<void> {
    try {
      return await invoke('clear_chat_history', { sessionId })
    } catch (error) {
      throw new Error(handleError(error, '清除聊天历史失败'))
    }
  }

  // ===== 上下文管理 =====

  /**
   * 获取当前终端上下文信息
   * @returns 返回终端的上下文数据，包含当前目录、环境变量等
   */
  async getTerminalContext(): Promise<Record<string, unknown>> {
    try {
      return await invoke('get_terminal_context')
    } catch (error) {
      throw new Error(handleError(error, '获取终端上下文失败'))
    }
  }

  /**
   * 更新终端上下文信息
   * @param context 要更新的上下文数据
   */
  async updateTerminalContext(context: Record<string, unknown>): Promise<void> {
    try {
      return await invoke('update_terminal_context', { context })
    } catch (error) {
      throw new Error(handleError(error, '更新终端上下文失败'))
    }
  }

  // ===== 用户前置提示词管理 =====

  /**
   * 获取用户前置提示词
   * @returns 返回用户设置的前置提示词，如果没有设置则返回null
   */
  async getUserPrefixPrompt(): Promise<string | null> {
    try {
      return await invoke('get_user_prefix_prompt')
    } catch (error) {
      throw new Error(handleError(error, '获取用户前置提示词失败'))
    }
  }

  /**
   * 设置用户前置提示词
   * @param prompt 要设置的前置提示词，传入null表示清除
   */
  async setUserPrefixPrompt(prompt: string | null): Promise<void> {
    try {
      return await invoke('set_user_prefix_prompt', { prompt })
    } catch (error) {
      throw new Error(handleError(error, '设置用户前置提示词失败'))
    }
  }
}

/**
 * AI API单例实例
 * 全局唯一的AI API实例，用于所有AI相关操作
 */
export const aiAPI = new AIAPI()

/**
 * 便捷的AI操作函数集合
 * 提供简化的AI功能调用接口，避免直接使用aiAPI实例
 */
export const ai = {
  // 模型管理
  getModels: () => aiAPI.getModels(),
  addModel: (config: AIModelConfig) => aiAPI.addModel(config),
  updateModel: (modelId: string, updates: Partial<AIModelConfig>) => aiAPI.updateModel(modelId, updates),
  removeModel: (modelId: string) => aiAPI.removeModel(modelId),
  deleteModel: (modelId: string) => aiAPI.removeModel(modelId), // 别名
  setDefaultModel: (modelId: string) => aiAPI.setDefaultModel(modelId),
  testConnection: (modelId: string) => aiAPI.testConnection(modelId),

  // AI功能
  sendMessage: (message: string, modelId?: string) => aiAPI.sendChatMessage(message, modelId),
  streamMessage: (message: string, callback: StreamCallback, modelId?: string) =>
    aiAPI.streamChatMessageWithChannel(message, callback, modelId),
  streamMessageCancellable: (message: string, callback: StreamCallback, modelId?: string) =>
    aiAPI.streamChatMessageCancellable(message, callback, modelId),
  explainCommand: (command: string, context?: Record<string, unknown>) => aiAPI.explainCommand(command, context),
  analyzeError: (error: string, command: string, context?: Record<string, unknown>) =>
    aiAPI.analyzeError(error, command, context),

  // 设置管理
  getSettings: () => aiAPI.getSettings(),
  updateSettings: (settings: Partial<AISettings>) => aiAPI.updateSettings(settings),

  // 统计监控
  getStats: () => aiAPI.getStats(),
  getHealthStatus: () => aiAPI.getHealthStatus(),

  // 聊天历史
  getChatHistory: (sessionId?: string) => aiAPI.getChatHistory(sessionId),
  getChatSessions: () => aiAPI.getChatSessions(),
  saveChatHistory: (messages: ChatMessage[], sessionId?: string) => aiAPI.saveChatHistory(messages, sessionId),
  clearChatHistory: (sessionId?: string) => aiAPI.clearChatHistory(sessionId),

  // 上下文管理
  getTerminalContext: () => aiAPI.getTerminalContext(),
  updateTerminalContext: (context: Record<string, unknown>) => aiAPI.updateTerminalContext(context),

  // 用户前置提示词管理
  getUserPrefixPrompt: () => aiAPI.getUserPrefixPrompt(),
  setUserPrefixPrompt: (prompt: string | null) => aiAPI.setUserPrefixPrompt(prompt),
}

// 类型定义现在统一从 @/types 导入，不在此处重复导出
