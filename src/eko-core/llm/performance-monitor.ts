/**
 * Performance monitoring for stream processing
 * Tracks metrics and provides insights for optimization
 */

export interface PerformanceMetrics {
  // Throughput metrics
  chunksPerSecond: number
  bytesPerSecond: number
  averageChunkSize: number

  // Latency metrics
  averageProcessingTime: number
  p95ProcessingTime: number
  p99ProcessingTime: number

  // Buffer metrics
  averageBufferUtilization: number
  maxBufferUtilization: number
  bufferOverflows: number
  backpressureEvents: number

  // Error metrics
  errorRate: number
  totalErrors: number

  // Session metrics
  totalChunks: number
  totalBytes: number
  sessionDuration: number
  startTime: number
}

/**
 * Sliding window for calculating percentiles
 */
class SlidingWindow {
  private values: number[] = []
  private maxSize: number

  constructor(maxSize: number = 1000) {
    this.maxSize = maxSize
  }

  add(value: number): void {
    this.values.push(value)
    if (this.values.length > this.maxSize) {
      this.values.shift()
    }
  }

  getPercentile(percentile: number): number {
    if (this.values.length === 0) return 0

    const sorted = [...this.values].sort((a, b) => a - b)
    const index = Math.ceil((percentile / 100) * sorted.length) - 1
    return sorted[Math.max(0, index)]
  }

  getAverage(): number {
    if (this.values.length === 0) return 0
    return this.values.reduce((sum, val) => sum + val, 0) / this.values.length
  }

  clear(): void {
    this.values.length = 0
  }
}

/**
 * High-performance stream processing monitor
 */
export class PerformanceMonitor {
  private startTime: number
  private totalChunks: number = 0
  private totalBytes: number = 0
  private totalErrors: number = 0
  private bufferOverflows: number = 0
  private backpressureEvents: number = 0

  // Sliding windows for metrics
  private processingTimes = new SlidingWindow(1000)
  private bufferUtilizations = new SlidingWindow(1000)
  private chunkSizes = new SlidingWindow(1000)

  // Time-based metrics
  private lastMetricsTime: number
  private lastChunkCount: number = 0
  private lastByteCount: number = 0

  constructor() {
    this.startTime = performance.now()
    this.lastMetricsTime = this.startTime
  }

  /**
   * Record a processed chunk
   */
  recordChunk(chunkSize: number, processingTime: number): void {
    this.totalChunks++
    this.totalBytes += chunkSize

    this.processingTimes.add(processingTime)
    this.chunkSizes.add(chunkSize)
  }

  /**
   * Record buffer utilization
   */
  recordBufferUtilization(utilization: number): void {
    this.bufferUtilizations.add(utilization)
  }

  /**
   * Record a buffer overflow event
   */
  recordBufferOverflow(): void {
    this.bufferOverflows++
  }

  /**
   * Record a backpressure event
   */
  recordBackpressure(): void {
    this.backpressureEvents++
  }

  /**
   * Record an error
   */
  recordError(): void {
    this.totalErrors++
  }

  /**
   * Get current performance metrics
   */
  getMetrics(): PerformanceMetrics {
    const now = performance.now()
    const sessionDuration = now - this.startTime
    const timeSinceLastMetrics = now - this.lastMetricsTime

    // Calculate throughput
    const chunksSinceLastMetrics = this.totalChunks - this.lastChunkCount
    const bytesSinceLastMetrics = this.totalBytes - this.lastByteCount

    const chunksPerSecond = timeSinceLastMetrics > 0 ? (chunksSinceLastMetrics / timeSinceLastMetrics) * 1000 : 0

    const bytesPerSecond = timeSinceLastMetrics > 0 ? (bytesSinceLastMetrics / timeSinceLastMetrics) * 1000 : 0

    // Update for next calculation
    this.lastMetricsTime = now
    this.lastChunkCount = this.totalChunks
    this.lastByteCount = this.totalBytes

    return {
      // Throughput metrics
      chunksPerSecond,
      bytesPerSecond,
      averageChunkSize: this.chunkSizes.getAverage(),

      // Latency metrics
      averageProcessingTime: this.processingTimes.getAverage(),
      p95ProcessingTime: this.processingTimes.getPercentile(95),
      p99ProcessingTime: this.processingTimes.getPercentile(99),

      // Buffer metrics
      averageBufferUtilization: this.bufferUtilizations.getAverage(),
      maxBufferUtilization: this.bufferUtilizations.getPercentile(100),
      bufferOverflows: this.bufferOverflows,
      backpressureEvents: this.backpressureEvents,

      // Error metrics
      errorRate: this.totalChunks > 0 ? (this.totalErrors / this.totalChunks) * 100 : 0,
      totalErrors: this.totalErrors,

      // Session metrics
      totalChunks: this.totalChunks,
      totalBytes: this.totalBytes,
      sessionDuration,
      startTime: this.startTime,
    }
  }

  /**
   * Reset all metrics
   */
  reset(): void {
    this.startTime = performance.now()
    this.lastMetricsTime = this.startTime
    this.totalChunks = 0
    this.totalBytes = 0
    this.totalErrors = 0
    this.bufferOverflows = 0
    this.backpressureEvents = 0
    this.lastChunkCount = 0
    this.lastByteCount = 0

    this.processingTimes.clear()
    this.bufferUtilizations.clear()
    this.chunkSizes.clear()
  }

  /**
   * Get performance insights and recommendations
   */
  getInsights(): string[] {
    const metrics = this.getMetrics()
    const insights: string[] = []

    // Throughput insights
    if (metrics.chunksPerSecond < 10) {
      insights.push('Low throughput detected. Consider increasing batch size or reducing processing overhead.')
    }

    // Latency insights
    if (metrics.averageProcessingTime > 50) {
      insights.push('High processing latency detected. Consider optimizing chunk processing logic.')
    }

    if (metrics.p99ProcessingTime > metrics.averageProcessingTime * 3) {
      insights.push('High latency variance detected. Some chunks are taking much longer to process.')
    }

    // Buffer insights
    if (metrics.averageBufferUtilization > 80) {
      insights.push('High buffer utilization. Consider increasing buffer size or improving processing speed.')
    }

    if (metrics.bufferOverflows > 0) {
      insights.push(`${metrics.bufferOverflows} buffer overflows detected. Increase buffer size or enable compression.`)
    }

    if (metrics.backpressureEvents > metrics.totalChunks * 0.1) {
      insights.push('Frequent backpressure events. Consider optimizing downstream processing.')
    }

    // Error insights
    if (metrics.errorRate > 1) {
      insights.push(`High error rate: ${metrics.errorRate.toFixed(2)}%. Check error handling and input validation.`)
    }

    // Performance insights
    if (metrics.averageChunkSize < 10) {
      insights.push('Small average chunk size. Consider batching smaller chunks for better efficiency.')
    }

    if (insights.length === 0) {
      insights.push('Stream processing performance looks good!')
    }

    return insights
  }

  /**
   * Log performance summary
   */
  logSummary(): void {
    const metrics = this.getMetrics()
    const insights = this.getInsights()

    console.group('🚀 Stream Performance Summary')
    console.log(
      `📊 Throughput: ${metrics.chunksPerSecond.toFixed(1)} chunks/sec, ${(metrics.bytesPerSecond / 1024).toFixed(1)} KB/sec`
    )
    console.log(
      `⏱️  Latency: ${metrics.averageProcessingTime.toFixed(2)}ms avg, ${metrics.p95ProcessingTime.toFixed(2)}ms p95`
    )
    console.log(
      `📦 Buffer: ${metrics.averageBufferUtilization.toFixed(1)}% avg utilization, ${metrics.bufferOverflows} overflows`
    )
    console.log(`❌ Errors: ${metrics.totalErrors} total (${metrics.errorRate.toFixed(2)}% rate)`)
    console.log(
      `📈 Session: ${metrics.totalChunks} chunks, ${(metrics.totalBytes / 1024).toFixed(1)} KB, ${(metrics.sessionDuration / 1000).toFixed(1)}s`
    )

    if (insights.length > 0) {
      console.group('💡 Insights & Recommendations')
      insights.forEach(insight => console.log(`• ${insight}`))
      console.groupEnd()
    }

    console.groupEnd()
  }
}

/**
 * Global performance monitor instance
 */
export const globalPerformanceMonitor = new PerformanceMonitor()

/**
 * Decorator for monitoring function performance
 */
export function monitorPerformance<T extends (...args: any[]) => any>(
  fn: T,
  monitor: PerformanceMonitor = globalPerformanceMonitor
): T {
  return ((...args: any[]) => {
    const startTime = performance.now()
    try {
      const result = fn(...args)

      if (result instanceof Promise) {
        return result.finally(() => {
          const endTime = performance.now()
          monitor.recordChunk(1, endTime - startTime)
        })
      } else {
        const endTime = performance.now()
        monitor.recordChunk(1, endTime - startTime)
        return result
      }
    } catch (error) {
      monitor.recordError()
      throw error
    }
  }) as T
}
