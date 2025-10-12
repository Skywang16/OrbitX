import type { AIHealthStatus, AIModelConfig, AISettings, AIStats } from '@/types'
import { invoke } from '@/utils/request'
import type { AIModelCreateInput, AIModelUpdateInput, AIModelTestConnectionInput } from './types'

export class AiApi {
  async getModels(): Promise<AIModelConfig[]> {
    return await invoke<AIModelConfig[]>('ai_models_get')
  }

  async addModel(model: AIModelCreateInput): Promise<AIModelConfig> {
    const timestamp = new Date()
    const config: AIModelConfig = {
      id: crypto.randomUUID(),
      name: model.name,
      provider: model.provider,
      apiUrl: model.apiUrl,
      apiKey: model.apiKey,
      model: model.model,
      modelType: model.modelType,
      enabled: model.enabled ?? true,
      options: model.options,
      createdAt: timestamp,
      updatedAt: timestamp,
    }

    return await invoke<AIModelConfig>('ai_models_add', { config })
  }

  async updateModel({ id, changes }: AIModelUpdateInput): Promise<void> {
    await invoke<void>('ai_models_update', {
      modelId: id,
      updates: changes,
    })
  }

  async deleteModel(modelId: string): Promise<void> {
    await invoke<void>('ai_models_remove', { modelId })
  }

  async testConnectionWithConfig(config: AIModelTestConnectionInput): Promise<void> {
    const payload: AIModelConfig = {
      id: crypto.randomUUID(),
      name: config.name,
      provider: config.provider,
      apiUrl: config.apiUrl,
      apiKey: config.apiKey,
      model: config.model,
      modelType: config.modelType,
      enabled: config.enabled ?? true,
      options: config.options,
      createdAt: new Date(),
      updatedAt: new Date(),
    }

    await invoke<void>('ai_models_test_connection', { config: payload })
  }

  async getUserRules(): Promise<string | null> {
    return await invoke<string | null>('agent_get_user_rules')
  }

  async setUserRules(rules: string | null): Promise<void> {
    await invoke<void>('agent_set_user_rules', { rules })
  }


  async getSettings(): Promise<AISettings> {
    return await invoke<AISettings>('get_ai_settings')
  }

  async updateSettings(settings: Partial<AISettings>): Promise<void> {
    await invoke<void>('update_ai_settings', { settings })
  }

  async getStats(): Promise<AIStats> {
    return await invoke<AIStats>('get_ai_stats')
  }

  async getHealthStatus(): Promise<AIHealthStatus> {
    return await invoke<AIHealthStatus>('get_ai_health_status')
  }
}

export const aiApi = new AiApi()
export type * from './types'
export default aiApi
