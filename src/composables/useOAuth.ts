import oauthApi from '@/api/llm/oauth'
import type { OAuthConfig, OAuthProvider, OAuthStatus } from '@/types/oauth'
import createMessage from '@/ui/composables/message-api'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, ref } from 'vue'

export interface OAuthState {
  isAuthenticating: boolean
  flowId: string | null
  cancelled: boolean
  config: OAuthConfig | null
}

export function useOAuth() {
  const state = ref<OAuthState>({
    isAuthenticating: false,
    flowId: null,
    cancelled: false,
    config: null,
  })

  const isAuthenticating = computed(() => state.value.isAuthenticating)
  const config = computed(() => state.value.config)

  /**
   * 启动 OAuth 授权流程
   */
  async function startAuthorization(provider: OAuthProvider): Promise<OAuthConfig | null> {
    state.value.isAuthenticating = true
    state.value.cancelled = false
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
    } catch (err: unknown) {
      // 主动取消不显示错误
      if (!state.value.cancelled) {
        const errorMsg = err instanceof Error ? err.message : String(err)
        createMessage.error(errorMsg || '授权失败')
        console.error('OAuth 授权失败:', errorMsg)
      }
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

    // 标记为主动取消
    state.value.cancelled = true

    try {
      await oauthApi.cancelFlow(state.value.flowId)
    } catch (err: unknown) {
      console.error('取消 OAuth 流程失败:', err)
    } finally {
      state.value.isAuthenticating = false
      state.value.flowId = null
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
    } catch (err: unknown) {
      const errorMsg = err instanceof Error ? err.message : String(err)
      createMessage.error(errorMsg || '刷新 Token 失败')
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
    } catch (err: unknown) {
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
      cancelled: false,
      config: null,
    }
  }

  return {
    isAuthenticating,
    config,
    startAuthorization,
    cancelAuthorization,
    refreshToken,
    checkStatus,
    reset,
  }
}
