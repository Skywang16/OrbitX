import type { AIModelConfig } from '@/types'

export type { AIModelConfig, AISettings } from '@/types'

export interface AIModelCreateInput {
  provider: AIModelConfig['provider']
  apiUrl: string
  apiKey: string
  model: string
  modelType: AIModelConfig['modelType']
  options?: AIModelConfig['options']
  useCustomBaseUrl?: boolean
}

export type AIModelUpdateChanges = Partial<
  Pick<AIModelConfig, 'provider' | 'apiUrl' | 'apiKey' | 'model' | 'modelType' | 'options' | 'useCustomBaseUrl'>
>

export interface AIModelUpdateInput {
  id: string
  changes: AIModelUpdateChanges
}

export interface AIModelTestConnectionInput extends AIModelCreateInput {}
