import { invoke } from '@tauri-apps/api/core'
import type { OAuthFlowInfo, OAuthProvider, OAuthConfig, OAuthStatus } from '@/types/oauth'

export class OAuthApi {
  /**
   * 启动 OAuth 流程
   */
  async startFlow(provider: OAuthProvider): Promise<OAuthFlowInfo> {
    return await invoke<OAuthFlowInfo>('start_oauth_flow', { provider })
  }

  /**
   * 等待 OAuth 回调
   */
  async waitForCallback(flowId: string, provider: OAuthProvider): Promise<OAuthConfig> {
    return await invoke<OAuthConfig>('wait_oauth_callback', { flowId, provider })
  }

  /**
   * 取消 OAuth 流程
   */
  async cancelFlow(flowId: string): Promise<void> {
    return await invoke<void>('cancel_oauth_flow', { flowId })
  }

  /**
   * 刷新 OAuth token
   */
  async refreshToken(oauthConfig: OAuthConfig): Promise<OAuthConfig> {
    return await invoke<OAuthConfig>('refresh_oauth_token', { oauthConfig })
  }

  /**
   * 检查 OAuth 状态
   */
  async checkStatus(oauthConfig: OAuthConfig): Promise<OAuthStatus> {
    const status = await invoke<string>('check_oauth_status', { oauthConfig })
    return status as OAuthStatus
  }
}

export const oauthApi = new OAuthApi()
export default oauthApi
