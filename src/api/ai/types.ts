import type { AIModelConfig } from '@/types'

export type { AIHealthStatus, AIModelConfig, AISettings, AIStats } from '@/types'

export interface AIModelCreateInput {
  name: string
  provider: AIModelConfig['provider']
  apiUrl: string
  apiKey: string
  model: string
  modelType: AIModelConfig['modelType']
  enabled?: boolean
  options?: AIModelConfig['options']
  useCustomBaseUrl?: boolean
}

export type AIModelUpdateChanges = Partial<
  Pick<AIModelConfig, 'name' | 'provider' | 'apiUrl' | 'apiKey' | 'model' | 'modelType' | 'enabled' | 'options'>
>

export interface AIModelUpdateInput {
  id: string
  changes: AIModelUpdateChanges
}

export interface AIModelTestConnectionInput extends AIModelCreateInput {}
