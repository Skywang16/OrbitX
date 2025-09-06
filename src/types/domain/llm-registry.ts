// LLM供应商注册表相关类型定义
// 对应后端 src-tauri/src/llm/registry.rs

/// 模型特殊能力标志
export interface ModelCapabilities {
  /// 是否支持工具调用
  supportsTools: boolean
  /// 是否支持视觉输入
  supportsVision: boolean
  /// 是否支持流式输出
  supportsStreaming: boolean
  /// 是否为推理模型（如o1系列）
  isReasoningModel: boolean
  /// 最大上下文长度
  maxContextTokens: number
  /// 推荐的温度范围
  temperatureRange?: [number, number]
}

/// 模型信息
export interface ModelInfo {
  /// 模型ID
  id: string
  /// 显示名称
  displayName: string
  /// 模型能力
  capabilities: ModelCapabilities
  /// 是否已弃用
  deprecated: boolean
}

/// LLM供应商类型
export type LLMProviderType = 'OpenAI' | 'Anthropic' | 'Gemini' | 'Qwen' | 'Custom'

/// 供应商配置
export interface ProviderInfo {
  /// 供应商类型
  providerType: LLMProviderType
  /// 显示名称
  displayName: string
  /// 默认API URL
  defaultApiUrl: string
  /// 文档链接
  documentationUrl?: string
  /// 支持的模型列表
  models: ModelInfo[]
  /// 是否需要API密钥
  requiresApiKey: boolean
}

/// 前端使用的Provider选项，用于表单
export interface ProviderOption {
  value: string
  label: string
  apiUrl: string
  models: string[]
  requiresApiKey?: boolean
  documentationUrl?: string
}

/// 前端使用的Model选项，用于表单
export interface ModelOption {
  value: string
  label: string
  capabilities?: ModelCapabilities
  deprecated?: boolean
}
