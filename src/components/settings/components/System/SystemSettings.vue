<script setup lang="ts">
  import { onMounted, onUnmounted } from 'vue'
  import { useSystemStore } from '@/stores/system'

  const systemStore = useSystemStore()

  // 组件挂载时初始化系统监控
  onMounted(async () => {
    try {
      await systemStore.initialize()
    } catch (error) {
      console.error('系统监控初始化失败:', error)
    }
  })

  // 组件卸载时停止自动刷新
  onUnmounted(() => {
    systemStore.stopAutoRefresh()
  })

  // 手动刷新
  const handleRefresh = async () => {
    try {
      await systemStore.refreshAll()
    } catch (error) {
      console.error('刷新系统状态失败:', error)
    }
  }

  // 预加载缓存
  const handlePreloadCache = async () => {
    try {
      await systemStore.preloadCache()
    } catch (error) {
      console.error('预加载缓存失败:', error)
    }
  }

  // 清空缓存
  const handleClearCache = async () => {
    try {
      await systemStore.clearCache()
    } catch (error) {
      console.error('清空缓存失败:', error)
    }
  }

  // 获取效率等级的颜色
  const getEfficiencyColor = (rating: string) => {
    switch (rating) {
      case 'excellent':
        return 'var(--color-success)'
      case 'good':
        return 'var(--color-info)'
      case 'fair':
        return 'var(--color-warning)'
      case 'poor':
        return 'var(--color-error)'
      default:
        return 'var(--text-secondary)'
    }
  }

  // 获取效率等级的文本
  const getEfficiencyText = (rating: string) => {
    switch (rating) {
      case 'excellent':
        return '优秀'
      case 'good':
        return '良好'
      case 'fair':
        return '一般'
      case 'poor':
        return '较差'
      default:
        return '未知'
    }
  }
</script>

<template>
  <div class="system-settings">
    <div class="settings-header">
      <h2>系统监控</h2>
      <p>监控存储系统的健康状态、缓存性能和存储使用情况</p>
    </div>

    <!-- 系统健康状态 -->
    <div class="settings-section">
      <div class="section-header">
        <h3>系统健康</h3>
        <x-button size="small" variant="outline" :loading="systemStore.healthLoading" @click="handleRefresh">
          刷新
        </x-button>
      </div>

      <div class="health-status">
        <div class="status-indicator" :class="{ healthy: systemStore.isHealthy, unhealthy: !systemStore.isHealthy }">
          <div class="status-icon">
            <svg
              v-if="systemStore.isHealthy"
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
              <polyline points="22,4 12,14.01 9,11.01" />
            </svg>
            <svg v-else width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="10" />
              <line x1="15" y1="9" x2="9" y2="15" />
              <line x1="9" y1="9" x2="15" y2="15" />
            </svg>
          </div>
          <div class="status-text">
            <div class="status-title">
              {{ systemStore.isHealthy ? '系统健康' : '系统异常' }}
            </div>
            <div class="status-message">
              {{ systemStore.healthStatus?.message || '正在检查...' }}
            </div>
          </div>
        </div>

        <div v-if="systemStore.healthStatus" class="health-details">
          <div class="detail-item">
            <span class="label">检查时间:</span>
            <span class="value">{{ new Date(systemStore.healthStatus.checked_at).toLocaleString() }}</span>
          </div>
          <div class="detail-item">
            <span class="label">检查耗时:</span>
            <span class="value">{{ systemStore.healthStatus.duration }}ms</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 缓存统计 -->
    <div class="settings-section">
      <div class="section-header">
        <h3>缓存性能</h3>
        <div class="cache-actions">
          <x-button size="small" variant="outline" :loading="systemStore.cacheLoading" @click="handlePreloadCache">
            预加载
          </x-button>
          <x-button size="small" variant="outline" :loading="systemStore.cacheLoading" @click="handleClearCache">
            清空缓存
          </x-button>
        </div>
      </div>

      <!-- 加载状态 -->
      <div v-if="systemStore.cacheLoading" class="loading-skeleton">
        <div class="skeleton-stats">
          <div class="skeleton-card"></div>
          <div class="skeleton-card"></div>
          <div class="skeleton-card"></div>
        </div>
      </div>

      <div v-else-if="systemStore.cacheStats" class="cache-stats">
        <div class="stats-overview">
          <div class="stat-card">
            <div class="stat-value">{{ (systemStore.totalHitRate * 100).toFixed(1) }}%</div>
            <div class="stat-label">总命中率</div>
            <div class="stat-rating" :style="{ color: getEfficiencyColor(systemStore.cacheEfficiencyRating) }">
              {{ getEfficiencyText(systemStore.cacheEfficiencyRating) }}
            </div>
          </div>
          <div class="stat-card">
            <div class="stat-value">{{ systemStore.formattedMemoryUsage }}</div>
            <div class="stat-label">内存使用</div>
          </div>
          <div class="stat-card">
            <div class="stat-value">{{ systemStore.cacheStats.total_entries }}</div>
            <div class="stat-label">缓存条目</div>
          </div>
        </div>

        <div class="cache-layers">
          <h4>缓存层详情</h4>
          <div class="layer-list">
            <div
              v-for="[layer, stats] in Object.entries(systemStore.cacheStats.layers)"
              :key="layer"
              class="layer-item"
            >
              <div class="layer-name">{{ layer.toUpperCase() }}</div>
              <div class="layer-stats">
                <span>命中: {{ stats.hits }}</span>
                <span>未命中: {{ stats.misses }}</span>
                <span>命中率: {{ ((stats.hits / (stats.hits + stats.misses)) * 100).toFixed(1) }}%</span>
                <span>条目: {{ stats.entries }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 存储统计 -->
    <div class="settings-section">
      <div class="section-header">
        <h3>存储使用</h3>
      </div>

      <div v-if="systemStore.storageStats" class="storage-stats">
        <div class="storage-overview">
          <div class="total-size">
            <div class="size-value">{{ systemStore.formattedTotalSize }}</div>
            <div class="size-label">总存储大小</div>
          </div>
        </div>

        <div class="storage-breakdown">
          <h4>存储分布</h4>
          <div class="breakdown-list">
            <div v-for="item in systemStore.storageBreakdown" :key="item.name" class="breakdown-item">
              <div class="item-name">{{ item.name }}</div>
              <div class="item-size">{{ item.formatted }}</div>
              <div class="item-bar">
                <div
                  class="bar-fill"
                  :style="{
                    width: `${(item.size / systemStore.storageStats.total_size) * 100}%`,
                  }"
                ></div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 错误信息 -->
    <div v-if="systemStore.error" class="error-section">
      <div class="error-message">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10" />
          <line x1="15" y1="9" x2="9" y2="15" />
          <line x1="9" y1="9" x2="15" y2="15" />
        </svg>
        {{ systemStore.error }}
      </div>
      <x-button size="small" variant="outline" @click="systemStore.clearError()">清除错误</x-button>
    </div>

    <!-- 最后更新时间 -->
    <div v-if="systemStore.lastUpdated" class="update-info">
      最后更新: {{ new Date(systemStore.lastUpdated).toLocaleString() }}
    </div>
  </div>
</template>

<style scoped>
  .system-settings {
    padding: var(--spacing-lg);
    max-width: 800px;
  }

  .settings-header {
    margin-bottom: var(--spacing-xl);
  }

  .settings-header h2 {
    margin: 0 0 var(--spacing-sm) 0;
    color: var(--text-primary);
  }

  .settings-header p {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
  }

  .settings-section {
    margin-bottom: var(--spacing-xl);
    padding: var(--spacing-lg);
    background: var(--color-background-secondary);
    border-radius: var(--border-radius-md);
    border: 1px solid var(--border-color);
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--spacing-lg);
  }

  .section-header h3 {
    margin: 0;
    color: var(--text-primary);
  }

  .cache-actions {
    display: flex;
    gap: var(--spacing-sm);
  }

  .health-status {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    padding: var(--spacing-md);
    border-radius: var(--border-radius-sm);
    border: 1px solid var(--border-color);
  }

  .status-indicator.healthy {
    background: var(--color-success-alpha);
    border-color: var(--color-success);
  }

  .status-indicator.unhealthy {
    background: var(--color-error-alpha);
    border-color: var(--color-error);
  }

  .status-icon {
    color: inherit;
  }

  .status-text {
    flex: 1;
  }

  .status-title {
    font-weight: 600;
    margin-bottom: 4px;
  }

  .status-message {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  .health-details {
    display: flex;
    gap: var(--spacing-lg);
    padding: var(--spacing-sm) 0;
    font-size: var(--font-size-sm);
  }

  .detail-item {
    display: flex;
    gap: var(--spacing-xs);
  }

  .detail-item .label {
    color: var(--text-secondary);
  }

  .detail-item .value {
    color: var(--text-primary);
    font-weight: 500;
  }

  .stats-overview {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: var(--spacing-md);
    margin-bottom: var(--spacing-lg);
  }

  .stat-card {
    text-align: center;
    padding: var(--spacing-md);
    background: var(--color-background);
    border-radius: var(--border-radius-sm);
    border: 1px solid var(--border-color);
  }

  .stat-value {
    font-size: var(--font-size-xl);
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: var(--spacing-xs);
  }

  .stat-label {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    margin-bottom: var(--spacing-xs);
  }

  .stat-rating {
    font-size: var(--font-size-xs);
    font-weight: 600;
  }

  .cache-layers h4,
  .storage-breakdown h4 {
    margin: 0 0 var(--spacing-md) 0;
    color: var(--text-primary);
    font-size: var(--font-size-md);
  }

  .layer-list,
  .breakdown-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
  }

  .layer-item,
  .breakdown-item {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    padding: var(--spacing-sm);
    background: var(--color-background);
    border-radius: var(--border-radius-sm);
    border: 1px solid var(--border-color);
  }

  .layer-name,
  .item-name {
    font-weight: 600;
    color: var(--text-primary);
    min-width: 80px;
  }

  .layer-stats {
    display: flex;
    gap: var(--spacing-md);
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  .item-size {
    font-weight: 600;
    color: var(--text-primary);
    min-width: 80px;
    text-align: right;
  }

  .item-bar {
    flex: 1;
    height: 8px;
    background: var(--color-background-secondary);
    border-radius: 4px;
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    background: var(--color-primary);
    transition: width 0.3s ease;
  }

  .storage-overview {
    text-align: center;
    margin-bottom: var(--spacing-lg);
  }

  .total-size {
    display: inline-block;
    padding: var(--spacing-lg);
    background: var(--color-background);
    border-radius: var(--border-radius-md);
    border: 1px solid var(--border-color);
  }

  .size-value {
    font-size: 2rem;
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: var(--spacing-xs);
  }

  .size-label {
    color: var(--text-secondary);
  }

  .error-section {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--spacing-md);
    background: var(--color-error-alpha);
    border: 1px solid var(--color-error);
    border-radius: var(--border-radius-sm);
    margin-bottom: var(--spacing-lg);
  }

  .error-message {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }

  .update-info {
    text-align: center;
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    padding: var(--spacing-sm);
    border-top: 1px solid var(--border-color);
  }

  /* 加载状态样式 */
  .loading-skeleton {
    padding: var(--spacing-lg) 0;
  }

  .skeleton-stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: var(--spacing-md);
  }

  .skeleton-card {
    height: 80px;
    background: linear-gradient(
      90deg,
      var(--color-background) 25%,
      var(--color-background-secondary) 50%,
      var(--color-background) 75%
    );
    background-size: 200% 100%;
    animation: skeleton-loading 1.5s infinite;
    border-radius: var(--border-radius-sm);
  }

  @keyframes skeleton-loading {
    0% {
      background-position: 200% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }
</style>
