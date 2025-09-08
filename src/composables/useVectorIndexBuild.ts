import { ref, computed } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { vectorIndexApi } from '@/api/vector-index'
import { createMessage } from '@/ui'

// Singleton listener to avoid multiple registrations
let listenerInstalled = false

// Shared state across consumers
const _isBuilding = ref(false)
const _progress = ref(0)
const _errorMessage = ref('')

export function useVectorIndexBuild() {
  const isBuilding = computed(() => _isBuilding.value)
  const progress = computed(() => _progress.value)
  const errorMessage = computed(() => _errorMessage.value)

  const formatBuildError = (raw: string): string => {
    const msg = raw.toLowerCase()
    if (msg.includes('model is not embedding') || msg.includes('模型未找到') || msg.includes('model not found')) {
      return '构建失败：所选 Embedding 模型不可用或不支持向量化。请到「AI 设置」切换为有效的 Embedding 模型（如 text-embedding-3-small），或正确配置当前模型。'
    }
    if (msg.includes('aead::error') || msg.includes('解密')) {
      return '构建失败：API 密钥解密失败。请在「AI 设置」重新输入并保存正确的密钥后重试。'
    }
    if (msg.includes('错误过多') || msg.includes('error rate')) {
      return raw // 已包含错误统计与根因示例
    }
    return raw
  }

  const ensureListener = async () => {
    if (listenerInstalled) return
    try {
      await listen('vector-index-event', event => {
        // VectorIndexEvent is tagged enum: { type, data }
        const payload = event.payload as
          | { type: 'progress'; data: { progress: number } }
          | { type: 'completed'; data: { totalFiles: number; totalChunks: number; elapsedTime: number } }
          | { type: 'error'; data: { operation?: string; message: string; timestamp?: string } }
          | { type: 'servicestatus'; data: { initialized: boolean; message: string } }
          | { type: 'monitorstarted'; data: unknown }
          | { type: 'monitorstopped'; data: unknown }
          | { type: 'incrementalupdatecomplete'; data: unknown }
          | { type: 'searchcomplete'; data: unknown }

        switch (payload.type) {
          case 'progress': {
            _isBuilding.value = true
            _errorMessage.value = ''
            const p = (payload.data?.progress ?? 0) * 100
            _progress.value = Math.max(1, Math.round(p))
            break
          }
          case 'completed': {
            _progress.value = 100
            _isBuilding.value = false
            break
          }
          case 'error': {
            // Any backend error during build should stop
            _isBuilding.value = false
            _progress.value = 0
            const raw = payload.data?.message || '构建失败'
            const pretty = formatBuildError(raw)
            _errorMessage.value = pretty
            createMessage.error('构建索引失败')
            break
          }
          default:
            break
        }
      })
      listenerInstalled = true
    } catch {
      // ignore listen error
    }
  }

  const startBuild = async () => {
    // 先切换前端状态，保证按钮立即变更
    _isBuilding.value = true
    _errorMessage.value = ''
    if (_progress.value === 0) _progress.value = 1
    await ensureListener()
    try {
      await vectorIndexApi.build()
      // completion handled by event
    } catch (e) {
      _isBuilding.value = false
      _progress.value = 0
      // 错误消息将通过后端 'vector-index-event' -> error 分支弹出
      _errorMessage.value = e instanceof Error ? e.message : String(e)
      throw e
    }
  }

  const cancelBuild = async () => {
    await ensureListener()
    try {
      await vectorIndexApi.cancelBuild()
    } finally {
      _isBuilding.value = false
      _progress.value = 0
    }
  }

  return {
    isBuilding,
    progress,
    errorMessage,
    startBuild,
    cancelBuild,
  }
}
