import type { AIModelConfig } from '@/types'
import type { AuthType, OAuthConfig } from '@/types/oauth'

export type { AIModelConfig, AISettings } from '@/types'

export interface AIModelCreateInput {
  provider: AIModelConfig['provider']
  authType: AuthType
  apiUrl?: string
  apiKey?: string
  model: string
  modelType: AIModelConfig['modelType']
  options?: AIModelConfig['options']
  oauthConfig?: OAuthConfig
  useCustomBaseUrl?: boolean
}

export type AIModelUpdateChanges = Partial<
  Pick<
    AIModelConfig,
    | 'provider'
    | 'authType'
    | 'apiUrl'
    | 'apiKey'
    | 'model'
    | 'modelType'
    | 'options'
    | 'oauthConfig'
    | 'useCustomBaseUrl'
  >
>

export interface AIModelUpdateInput {
  id: string
  changes: AIModelUpdateChanges
}

export interface AIModelTestConnectionInput extends Omit<AIModelCreateInput, 'oauthConfig'> {}
