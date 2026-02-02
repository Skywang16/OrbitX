import type { AIModelConfig } from '@/types'
import { invoke } from '@/utils/request'
import type { AIModelCreateInput, AIModelTestConnectionInput, AIModelUpdateInput } from './types'

export class AiApi {
  getModels = async (): Promise<AIModelConfig[]> => {
    return await invoke<AIModelConfig[]>('ai_models_get')
  }

  addModel = async (model: AIModelCreateInput): Promise<AIModelConfig> => {
    const timestamp = new Date()
    const config: AIModelConfig = {
      id: crypto.randomUUID(),
      provider: model.provider,
      authType: model.authType,
      apiUrl: model.apiUrl,
      apiKey: model.apiKey,
      model: model.model,
      modelType: model.modelType,
      options: model.options,
      oauthConfig: model.oauthConfig,
      useCustomBaseUrl: model.useCustomBaseUrl,
      createdAt: timestamp,
      updatedAt: timestamp,
    }

    return await invoke<AIModelConfig>('ai_models_add', { config })
  }

  updateModel = async ({ id, changes }: AIModelUpdateInput): Promise<void> => {
    await invoke<void>('ai_models_update', {
      modelId: id,
      updates: changes,
    })
  }

  deleteModel = async (modelId: string): Promise<void> => {
    await invoke<void>('ai_models_remove', { modelId })
  }

  testConnectionWithConfig = async (config: AIModelTestConnectionInput): Promise<void> => {
    const payload: AIModelConfig = {
      id: crypto.randomUUID(),
      provider: config.provider,
      authType: config.authType,
      apiUrl: config.apiUrl,
      apiKey: config.apiKey,
      model: config.model,
      modelType: config.modelType,
      options: config.options,
      useCustomBaseUrl: config.useCustomBaseUrl,
      createdAt: new Date(),
      updatedAt: new Date(),
    }

    await invoke<void>('ai_models_test_connection', { config: payload })
  }
}

export const aiApi = new AiApi()
export type * from './types'
export default aiApi
