// OAuth 认证类型
export enum AuthType {
  ApiKey = 'api_key',
  OAuth = 'oauth',
}

// OAuth Provider 类型
export enum OAuthProvider {
  OpenAiCodex = 'openai_codex',
  ClaudePro = 'claude_pro',
  GeminiAdvanced = 'gemini_advanced',
}

// OAuth 配置
export interface OAuthConfig {
  provider: OAuthProvider
  refreshToken: string
  accessToken?: string
  expiresAt?: number
  metadata?: Record<string, any>
}

// OAuth 流程信息
export interface OAuthFlowInfo {
  flowId: string
  authorizeUrl: string
  provider: string
}

// OAuth 状态
export enum OAuthStatus {
  NotAuthorized = 'not_authorized',
  Authorized = 'authorized',
  TokenExpired = 'token_expired',
  Authorizing = 'authorizing',
}

// Provider 信息
export interface ProviderInfo {
  id: OAuthProvider
  name: string
  description: string
  icon: string
  available: boolean
}
