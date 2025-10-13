// LLM供应商注册表相关类型定义

/// 预设模型信息（与后端 PresetModel 结构一致）

export interface PresetModel {
  id: string
  name: string
  maxTokens: number | null
  contextWindow: number
  supportsImages: boolean
  supportsPromptCache: boolean
  inputPrice?: number // 每百万 tokens，单位：美元
  outputPrice?: number // 每百万 tokens，单位：美元
  cacheReadsPrice?: number // 每百万 tokens，单位：美元
  cacheWritesPrice?: number // 每百万 tokens，单位：美元
  description?: string
}

/// 供应商元数据
export interface ProviderInfo {
  providerType: string
  displayName: string
  defaultApiUrl: string
  presetModels: PresetModel[]
}

/// 前端使用的Provider选项
export interface ProviderOption {
  value: string
  label: string
  apiUrl: string
  models: string[]
}

/// 前端使用的Model选项
export interface ModelOption {
  value: string
  label: string
}
