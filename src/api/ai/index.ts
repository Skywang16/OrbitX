import type { AIHealthStatus, AIModelConfig, AISettings, AIStats } from '@/types'
import { invoke } from '@/utils/request'
import type { AIModelCreateInput, AIModelUpdateInput, AIModelTestConnectionInput } from './types'

export class AiApi {
  getModels = async (): Promise<AIModelConfig[]> => {
    return await invoke<AIModelConfig[]>('ai_models_get')
  }

  addModel = async (model: AIModelCreateInput): Promise<AIModelConfig> => {
    const timestamp = new Date()
    const config: AIModelConfig = {
      id: crypto.randomUUID(),
      provider: model.provider,
      apiUrl: model.apiUrl,
      apiKey: model.apiKey,
      model: model.model,
      modelType: model.modelType,
      options: model.options,
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

  getUserRules = async (): Promise<string | null> => {
    return await invoke<string | null>('agent_get_user_rules')
  }

  setUserRules = async (rules: string | null): Promise<void> => {
    await invoke<void>('agent_set_user_rules', { rules })
  }

  getSettings = async (): Promise<AISettings> => {
    return await invoke<AISettings>('get_ai_settings')
  }

  updateSettings = async (settings: Partial<AISettings>): Promise<void> => {
    await invoke<void>('update_ai_settings', { settings })
  }

  getStats = async (): Promise<AIStats> => {
    return await invoke<AIStats>('get_ai_stats')
  }

  getHealthStatus = async (): Promise<AIHealthStatus> => {
    return await invoke<AIHealthStatus>('get_ai_health_status')
  }
}

export const aiApi = new AiApi()
export type * from './types'
export default aiApi
