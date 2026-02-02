import oauthApi from '@/api/llm/oauth'
import type { OAuthConfig, OAuthProvider, OAuthStatus } from '@/types/oauth'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, ref } from 'vue'

export interface OAuthState {
  isAuthenticating: boolean
  flowId: string | null
  error: string | null
  config: OAuthConfig | null
}

export function useOAuth() {
  const state = ref<OAuthState>({
    isAuthenticating: false,
    flowId: null,
    error: null,
    config: null,
  })

  const isAuthenticating = computed(() => state.value.isAuthenticating)
  const error = computed(() => state.value.error)
  const config = computed(() => state.value.config)

  /**
   * 启动 OAuth 授权流程
   */
  async function startAuthorization(provider: OAuthProvider): Promise<OAuthConfig | null> {
    state.value.isAuthenticating = true
    state.value.error = null
    state.value.config = null

    try {
      // 1. 启动 OAuth 流程,获取授权 URL
      const flowInfo = await oauthApi.startFlow(provider)
      state.value.flowId = flowInfo.flowId

      // 2. 在浏览器中打开授权 URL
      await openUrl(flowInfo.authorizeUrl)

      // 3. 等待回调
      const oauthConfig = await oauthApi.waitForCallback(flowInfo.flowId, provider)
      state.value.config = oauthConfig

      return oauthConfig
    } catch (err: any) {
      const errorMsg = err?.message || String(err)
      state.value.error = errorMsg
      console.error('OAuth 授权失败:', errorMsg)
      return null
    } finally {
      state.value.isAuthenticating = false
      state.value.flowId = null
    }
  }

  /**
   * 取消当前 OAuth 流程
   */
  async function cancelAuthorization(): Promise<void> {
    if (!state.value.flowId) return

    try {
      await oauthApi.cancelFlow(state.value.flowId)
    } catch (err: any) {
      console.error('取消 OAuth 流程失败:', err)
    } finally {
      state.value.isAuthenticating = false
      state.value.flowId = null
      state.value.error = null
    }
  }

  /**
   * 刷新 OAuth token
   */
  async function refreshToken(oauthConfig: OAuthConfig): Promise<OAuthConfig | null> {
    try {
      const newConfig = await oauthApi.refreshToken(oauthConfig)
      state.value.config = newConfig
      return newConfig
    } catch (err: any) {
      const errorMsg = err?.message || String(err)
      state.value.error = errorMsg
      console.error('刷新 OAuth token 失败:', errorMsg)
      return null
    }
  }

  /**
   * 检查 OAuth 状态
   */
  async function checkStatus(oauthConfig: OAuthConfig): Promise<OAuthStatus | null> {
    try {
      return await oauthApi.checkStatus(oauthConfig)
    } catch (err: any) {
      console.error('检查 OAuth 状态失败:', err)
      return null
    }
  }

  /**
   * 重置状态
   */
  function reset(): void {
    state.value = {
      isAuthenticating: false,
      flowId: null,
      error: null,
      config: null,
    }
  }

  return {
    isAuthenticating,
    error,
    config,
    startAuthorization,
    cancelAuthorization,
    refreshToken,
    checkStatus,
    reset,
  }
}
