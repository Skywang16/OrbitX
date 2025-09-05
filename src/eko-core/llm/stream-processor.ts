import { NativeLLMStreamChunk, NativeLLMToolCall, NativeLLMUsage } from '../types/llm.types'
import { BufferedStreamProcessor, DEFAULT_BUFFER_CONFIG, StreamBufferConfig } from './stream-buffer'
import { PerformanceMonitor, globalPerformanceMonitor } from './performance-monitor'

/**
 * Configuration for stream processing optimization
 */
export interface StreamProcessorConfig {
  maxBufferSize: number
  backpressureThreshold: number
  batchSize: number
  flushInterval: number
  enableMetrics: boolean
}

/**
 * Default configuration for stream processing
 */
export const DEFAULT_STREAM_CONFIG: StreamProcessorConfig = {
  maxBufferSize: 1000,
  backpressureThreshold: 800,
  batchSize: 10,
  flushInterval: 0,
  enableMetrics: false,
}

/**
 * Stream processing metrics
 */
export interface StreamMetrics {
  chunksProcessed: number
  bytesProcessed: number
  averageLatency: number
  backpressureEvents: number
  bufferOverflows: number
  processingTime: number
}

/**
 * High-efficiency stream processor for handling native LLM stream chunks
 * Provides optimized utilities for processing different types of stream events
 * with backpressure handling and performance monitoring
 */
export class StreamProcessor {
  private static metrics: StreamMetrics = {
    chunksProcessed: 0,
    bytesProcessed: 0,
    averageLatency: 0,
    backpressureEvents: 0,
    bufferOverflows: 0,
    processingTime: 0,
  }

  /**
   * Process a stream chunk with optimized handling
   */
  static processStreamChunk(
    chunk: NativeLLMStreamChunk,
    handlers: {
      onTextDelta?: (content: string) => void
      onToolCalls?: (toolCalls: NativeLLMToolCall[]) => void
      onFinish?: (finishReason: string, usage?: NativeLLMUsage) => void
      onError?: (error: string) => void
    },
    config: StreamProcessorConfig = DEFAULT_STREAM_CONFIG
  ): void {
    const startTime = performance.now()

    try {
      switch (chunk.type) {
        case 'delta':
          if (chunk.content && handlers.onTextDelta) {
            handlers.onTextDelta(chunk.content)
            if (config.enableMetrics) {
              this.metrics.bytesProcessed += chunk.content.length
            }
          }
          if (chunk.toolCalls && handlers.onToolCalls) {
            handlers.onToolCalls(chunk.toolCalls)
          }
          break
        case 'finish':
          if (handlers.onFinish) {
            handlers.onFinish(chunk.finishReason, chunk.usage)
          }
          break
        case 'error':
          if (handlers.onError) {
            handlers.onError(chunk.error)
          }
          break
      }

      if (config.enableMetrics) {
        this.metrics.chunksProcessed++
        const processingTime = performance.now() - startTime
        this.metrics.processingTime += processingTime
        this.metrics.averageLatency = this.metrics.processingTime / this.metrics.chunksProcessed
      }
    } catch (error) {
      console.error('Error processing stream chunk:', error)
      if (handlers.onError) {
        handlers.onError(`Processing error: ${error}`)
      }
    }
  }

  /**
   * Create an optimized text accumulator from a stream with batching
   */
  static async accumulateText(
    stream: ReadableStream<NativeLLMStreamChunk>,
    config: StreamProcessorConfig = DEFAULT_STREAM_CONFIG
  ): Promise<{
    content: string
    toolCalls: NativeLLMToolCall[]
    finishReason: string
    usage?: NativeLLMUsage
  }> {
    let content = ''
    let toolCalls: NativeLLMToolCall[] = []
    let finishReason = 'stop'
    let usage: NativeLLMUsage | undefined

    // Use batching for better performance
    const contentBuffer: string[] = []
    let bufferSize = 0

    const flushBuffer = () => {
      if (contentBuffer.length > 0) {
        content += contentBuffer.join('')
        contentBuffer.length = 0
        bufferSize = 0
      }
    }

    const reader = stream.getReader()
    try {
      while (true) {
        const { done, value } = await reader.read()
        if (done) break

        this.processStreamChunk(
          value,
          {
            onTextDelta: text => {
              contentBuffer.push(text)
              bufferSize += text.length

              // Flush buffer when it gets too large or when batch size is reached
              if (bufferSize > config.maxBufferSize || contentBuffer.length >= config.batchSize) {
                flushBuffer()
              }
            },
            onToolCalls: calls => {
              toolCalls.push(...calls)
            },
            onFinish: (reason, streamUsage) => {
              finishReason = reason
              usage = streamUsage
              flushBuffer() // Ensure all content is flushed
            },
            onError: error => {
              flushBuffer() // Flush any remaining content before error
              throw new Error(`Stream error: ${error}`)
            },
          },
          config
        )
      }

      // Final flush in case there's remaining content
      flushBuffer()
    } finally {
      reader.releaseLock()
    }

    return { content, toolCalls, finishReason, usage }
  }

  /**
   * Transform a stream with custom processing and backpressure handling
   */
  static transformStream<T>(
    stream: ReadableStream<NativeLLMStreamChunk>,
    transformer: (chunk: NativeLLMStreamChunk) => T | null,
    config: StreamProcessorConfig = DEFAULT_STREAM_CONFIG
  ): ReadableStream<T> {
    let bufferSize = 0
    let backpressureActive = false

    return new ReadableStream<T>({
      start(controller) {
        const reader = stream.getReader()

        const pump = async () => {
          try {
            while (true) {
              // Handle backpressure
              if (bufferSize >= config.backpressureThreshold && !backpressureActive) {
                backpressureActive = true
                if (config.enableMetrics) {
                  StreamProcessor.metrics.backpressureEvents++
                }

                // Wait for buffer to drain
                while (bufferSize >= config.backpressureThreshold) {
                  await new Promise(resolve => setTimeout(resolve, config.flushInterval))
                }
                backpressureActive = false
              }

              const { done, value } = await reader.read()
              if (done) {
                controller.close()
                break
              }

              const transformed = transformer(value)
              if (transformed !== null) {
                if (bufferSize >= config.maxBufferSize) {
                  if (config.enableMetrics) {
                    StreamProcessor.metrics.bufferOverflows++
                  }
                  // Drop oldest items or handle overflow
                  console.warn('Stream buffer overflow, dropping data')
                  continue
                }

                controller.enqueue(transformed)
                bufferSize++

                // Simulate buffer consumption
                setTimeout(() => {
                  bufferSize = Math.max(0, bufferSize - 1)
                }, 1)
              }
            }
          } catch (error) {
            controller.error(error)
          } finally {
            reader.releaseLock()
          }
        }

        pump()
      },
    })
  }

  /**
   * Add advanced backpressure handling to prevent memory overflow
   */
  static addBackpressure(
    stream: ReadableStream<NativeLLMStreamChunk>,
    config: StreamProcessorConfig = DEFAULT_STREAM_CONFIG
  ): ReadableStream<NativeLLMStreamChunk> {
    let bufferSize = 0
    let backpressureActive = false
    const buffer: NativeLLMStreamChunk[] = []

    return new ReadableStream<NativeLLMStreamChunk>({
      start(controller) {
        const reader = stream.getReader()

        // Batch processing for better performance
        const processBatch = () => {
          const batchSize = Math.min(config.batchSize, buffer.length)
          for (let i = 0; i < batchSize; i++) {
            const chunk = buffer.shift()
            if (chunk) {
              controller.enqueue(chunk)
              bufferSize--
            }
          }
        }

        // Set up batch processing interval
        const batchInterval = setInterval(() => {
          if (buffer.length > 0) {
            processBatch()
          }
        }, config.flushInterval)

        const pump = async () => {
          try {
            while (true) {
              // Handle backpressure with adaptive throttling
              if (bufferSize >= config.backpressureThreshold && !backpressureActive) {
                backpressureActive = true
                if (config.enableMetrics) {
                  StreamProcessor.metrics.backpressureEvents++
                }

                // Adaptive backpressure - wait longer if buffer is very full
                const waitTime = Math.min(config.flushInterval * (bufferSize / config.backpressureThreshold), 100)

                await new Promise(resolve => setTimeout(resolve, waitTime))
                backpressureActive = false
              }

              const { done, value } = await reader.read()
              if (done) {
                // Process remaining buffer
                while (buffer.length > 0) {
                  processBatch()
                }
                clearInterval(batchInterval)
                controller.close()
                break
              }

              // Handle buffer overflow
              if (bufferSize >= config.maxBufferSize) {
                if (config.enableMetrics) {
                  StreamProcessor.metrics.bufferOverflows++
                }

                // Drop oldest chunks to make room (FIFO)
                const dropCount = Math.ceil(config.maxBufferSize * 0.1) // Drop 10%
                for (let i = 0; i < dropCount && buffer.length > 0; i++) {
                  buffer.shift()
                  bufferSize--
                }
                console.warn(`Stream buffer overflow, dropped ${dropCount} chunks`)
              }

              buffer.push(value)
              bufferSize++
            }
          } catch (error) {
            clearInterval(batchInterval)
            controller.error(error)
          } finally {
            reader.releaseLock()
          }
        }

        pump()
      },
      cancel() {
        // Clean up resources
        buffer.length = 0
        bufferSize = 0
      },
    })
  }

  /**
   * Create an optimized batched stream processor
   */
  static createBatchedStream(
    stream: ReadableStream<NativeLLMStreamChunk>,
    config: StreamProcessorConfig = DEFAULT_STREAM_CONFIG
  ): ReadableStream<NativeLLMStreamChunk[]> {
    const buffer: NativeLLMStreamChunk[] = []

    return new ReadableStream<NativeLLMStreamChunk[]>({
      start(controller) {
        const reader = stream.getReader()

        const flushBatch = () => {
          if (buffer.length > 0) {
            controller.enqueue([...buffer])
            buffer.length = 0
          }
        }

        // Set up periodic flushing
        const flushInterval = setInterval(flushBatch, config.flushInterval)

        const pump = async () => {
          try {
            while (true) {
              const { done, value } = await reader.read()
              if (done) {
                flushBatch() // Flush remaining items
                clearInterval(flushInterval)
                controller.close()
                break
              }

              buffer.push(value)

              // Flush when batch is full
              if (buffer.length >= config.batchSize) {
                flushBatch()
              }
            }
          } catch (error) {
            clearInterval(flushInterval)
            controller.error(error)
          } finally {
            reader.releaseLock()
          }
        }

        pump()
      },
      cancel() {
        buffer.length = 0
      },
    })
  }

  /**
   * Get current stream processing metrics
   */
  static getMetrics(): StreamMetrics {
    return { ...this.metrics }
  }

  /**
   * Reset stream processing metrics
   */
  static resetMetrics(): void {
    this.metrics = {
      chunksProcessed: 0,
      bytesProcessed: 0,
      averageLatency: 0,
      backpressureEvents: 0,
      bufferOverflows: 0,
      processingTime: 0,
    }
  }

  /**
   * Create a high-performance stream with all optimizations enabled
   */
  static createOptimizedStream(
    stream: ReadableStream<NativeLLMStreamChunk>,
    config: Partial<StreamProcessorConfig> = {},
    monitor: PerformanceMonitor = globalPerformanceMonitor
  ): ReadableStream<NativeLLMStreamChunk> {
    const fullConfig: StreamProcessorConfig = {
      ...DEFAULT_STREAM_CONFIG,
      ...config,
      enableMetrics: true, // Always enable metrics for optimized streams
    }

    // Apply backpressure handling first
    const backpressureStream = this.addBackpressure(stream, fullConfig)

    // Add performance monitoring
    return this.transformStream(
      backpressureStream,
      chunk => {
        const startTime = performance.now()

        // Calculate chunk size for metrics
        let chunkSize = 0
        if (chunk.type === 'delta' && chunk.content) {
          chunkSize = chunk.content.length
        }

        // Record metrics
        const processingTime = performance.now() - startTime
        monitor.recordChunk(chunkSize, processingTime)

        // Pass through all chunks while collecting metrics
        return chunk
      },
      fullConfig
    )
  }

  /**
   * Create a buffered high-performance stream processor
   */
  static createBufferedStream(
    stream: ReadableStream<NativeLLMStreamChunk>,
    onBatch: (chunks: NativeLLMStreamChunk[]) => void,
    bufferConfig: Partial<StreamBufferConfig> = {},
    monitor: PerformanceMonitor = globalPerformanceMonitor
  ): BufferedStreamProcessor {
    const fullConfig: StreamBufferConfig = {
      ...DEFAULT_BUFFER_CONFIG,
      ...bufferConfig,
    }

    const processor = new BufferedStreamProcessor(onBatch, fullConfig)

    // Process the stream
    const reader = stream.getReader()

    const pump = async () => {
      try {
        while (true) {
          const { done, value } = await reader.read()
          if (done) break

          const startTime = performance.now()
          processor.addChunk(value)

          // Record metrics
          let chunkSize = 0
          if (value.type === 'delta' && value.content) {
            chunkSize = value.content.length
          }

          const processingTime = performance.now() - startTime
          monitor.recordChunk(chunkSize, processingTime)

          // Record buffer utilization
          const stats = processor.getStats()
          monitor.recordBufferUtilization(stats.utilization)

          if (stats.isFull) {
            monitor.recordBufferOverflow()
          }
        }
      } catch (error) {
        monitor.recordError()
        console.error('Buffered stream processing error:', error)
      } finally {
        reader.releaseLock()
        processor.stop()
      }
    }

    pump()
    return processor
  }

  /**
   * Create a memory-efficient stream for large data processing
   */
  static createMemoryEfficientStream(
    stream: ReadableStream<NativeLLMStreamChunk>,
    config: Partial<StreamProcessorConfig> = {}
  ): ReadableStream<NativeLLMStreamChunk> {
    const fullConfig: StreamProcessorConfig = {
      ...DEFAULT_STREAM_CONFIG,
      maxBufferSize: 200, // Larger buffer to avoid dropping chunks
      backpressureThreshold: 150, // Higher threshold
      batchSize: 20, // Larger batches for efficiency
      flushInterval: 4, // Faster flushing
      enableMetrics: false, // Disable metrics for memory efficiency
      ...config,
    }

    let processedChunks = 0

    return this.transformStream(
      stream,
      chunk => {
        processedChunks++

        // Periodic garbage collection hint for large streams
        if (processedChunks % 1000 === 0) {
          // Suggest garbage collection for long-running streams
          if (typeof global !== 'undefined' && global.gc) {
            global.gc()
          }
        }

        return chunk
      },
      fullConfig
    )
  }

  /**
   * Create a debug stream that logs performance metrics
   */
  static createDebugStream(
    stream: ReadableStream<NativeLLMStreamChunk>,
    logInterval: number = 5000
  ): ReadableStream<NativeLLMStreamChunk> {
    const monitor = new PerformanceMonitor()
    let lastLogTime = performance.now()

    return this.transformStream(stream, chunk => {
      const now = performance.now()

      // Calculate chunk size
      let chunkSize = 0
      if (chunk.type === 'delta' && chunk.content) {
        chunkSize = chunk.content.length
      }

      monitor.recordChunk(chunkSize, 1) // Minimal processing time for debug

      // Log metrics periodically
      if (now - lastLogTime > logInterval) {
        monitor.logSummary()
        lastLogTime = now
      }

      return chunk
    })
  }
}
