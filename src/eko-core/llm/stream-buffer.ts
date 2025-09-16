import { NativeLLMStreamChunk } from '../types/llm.types'

/**
 * Configuration for the high-performance stream buffer
 */
export interface StreamBufferConfig {
  maxSize: number
  batchSize: number
  flushInterval: number
  enableCompression: boolean
  compressionThreshold: number
}

/**
 * Default configuration for stream buffer
 */
export const DEFAULT_BUFFER_CONFIG: StreamBufferConfig = {
  maxSize: 10000,
  batchSize: 50,
  flushInterval: 8, // ~120fps for very smooth streaming
  enableCompression: true,
  compressionThreshold: 1000, // Compress when buffer exceeds this size
}

/**
 * High-performance circular buffer for stream chunks
 * Optimized for memory efficiency and fast access patterns
 */
export class StreamBuffer {
  private buffer: Array<NativeLLMStreamChunk | undefined>
  private head: number = 0
  private tail: number = 0
  private size: number = 0
  private readonly maxSize: number
  private readonly config: StreamBufferConfig

  constructor(config: StreamBufferConfig = DEFAULT_BUFFER_CONFIG) {
    this.config = config
    this.maxSize = config.maxSize
    this.buffer = new Array(this.maxSize)
  }

  /**
   * Add a chunk to the buffer
   * Returns true if successful, false if buffer is full
   */
  push(chunk: NativeLLMStreamChunk): boolean {
    if (this.size >= this.maxSize) {
      // Buffer is full, apply overflow strategy
      return this.handleOverflow(chunk)
    }

    this.buffer[this.tail] = chunk
    this.tail = (this.tail + 1) % this.maxSize
    this.size++
    return true
  }

  /**
   * Remove and return the oldest chunk from the buffer
   */
  shift(): NativeLLMStreamChunk | undefined {
    if (this.size === 0) {
      return undefined
    }

    const chunk = this.buffer[this.head]
    this.buffer[this.head] = undefined // Help GC
    this.head = (this.head + 1) % this.maxSize
    this.size--
    return chunk
  }

  /**
   * Get multiple chunks at once for batch processing
   */
  shiftBatch(count: number = this.config.batchSize): NativeLLMStreamChunk[] {
    const result: NativeLLMStreamChunk[] = []
    const actualCount = Math.min(count, this.size)

    for (let i = 0; i < actualCount; i++) {
      const chunk = this.shift()
      if (chunk) {
        result.push(chunk)
      }
    }

    return result
  }

  /**
   * Peek at the next chunk without removing it
   */
  peek(): NativeLLMStreamChunk | undefined {
    if (this.size === 0) {
      return undefined
    }
    return this.buffer[this.head]
  }

  /**
   * Get current buffer size
   */
  getSize(): number {
    return this.size
  }

  /**
   * Check if buffer is empty
   */
  isEmpty(): boolean {
    return this.size === 0
  }

  /**
   * Check if buffer is full
   */
  isFull(): boolean {
    return this.size >= this.maxSize
  }

  /**
   * Get buffer utilization as percentage
   */
  getUtilization(): number {
    return (this.size / this.maxSize) * 100
  }

  /**
   * Clear the buffer
   */
  clear(): void {
    // Help GC by clearing references
    for (let i = 0; i < this.maxSize; i++) {
      this.buffer[i] = undefined
    }
    this.head = 0
    this.tail = 0
    this.size = 0
  }

  /**
   * Handle buffer overflow with different strategies
   */
  private handleOverflow(chunk: NativeLLMStreamChunk): boolean {
    // Strategy 1: Drop oldest chunks to make room
    if (chunk.type === 'finish' || chunk.type === 'error') {
      // Always make room for finish/error chunks
      this.shift()
      return this.push(chunk)
    }

    // Strategy 2: Compress buffer if enabled
    if (this.config.enableCompression && this.size > this.config.compressionThreshold) {
      this.compressBuffer()
      if (this.size < this.maxSize) {
        return this.push(chunk)
      }
    }

    // Strategy 3: Drop the new chunk (backpressure)
    console.warn('Stream buffer overflow, dropping chunk:', chunk.type)
    return false
  }

  /**
   * Compress buffer by merging consecutive text deltas
   */
  private compressBuffer(): void {
    const compressed: NativeLLMStreamChunk[] = []
    let currentTextContent = ''
    let hasTextContent = false

    // Extract all chunks for processing
    const allChunks: NativeLLMStreamChunk[] = []
    while (!this.isEmpty()) {
      const chunk = this.shift()
      if (chunk) {
        allChunks.push(chunk)
      }
    }

    // Merge consecutive text deltas
    for (const chunk of allChunks) {
      if (chunk.type === 'delta' && chunk.content && !chunk.toolCalls) {
        currentTextContent += chunk.content
        hasTextContent = true
      } else {
        // Flush accumulated text if any
        if (hasTextContent) {
          compressed.push({
            type: 'delta',
            content: currentTextContent,
          })
          currentTextContent = ''
          hasTextContent = false
        }
        compressed.push(chunk)
      }
    }

    // Flush any remaining text
    if (hasTextContent) {
      compressed.push({
        type: 'delta',
        content: currentTextContent,
      })
    }

    // Put compressed chunks back
    for (const chunk of compressed) {
      if (!this.push(chunk)) {
        break // Stop if buffer becomes full again
      }
    }

    console.warn(`Buffer compressed from ${allChunks.length} to ${compressed.length} chunks`)
  }

  /**
   * Get buffer statistics for monitoring
   */
  getStats(): {
    size: number
    maxSize: number
    utilization: number
    isEmpty: boolean
    isFull: boolean
  } {
    return {
      size: this.size,
      maxSize: this.maxSize,
      utilization: this.getUtilization(),
      isEmpty: this.isEmpty(),
      isFull: this.isFull(),
    }
  }
}

/**
 * High-performance stream processor using the optimized buffer
 */
export class BufferedStreamProcessor {
  private buffer: StreamBuffer
  private flushTimer: NodeJS.Timeout | null = null
  private isProcessing = false

  constructor(
    private onBatch: (chunks: NativeLLMStreamChunk[]) => void,
    config: StreamBufferConfig = DEFAULT_BUFFER_CONFIG
  ) {
    this.buffer = new StreamBuffer(config)
    this.startFlushTimer(config.flushInterval)
  }

  /**
   * Add a chunk to the processor
   */
  addChunk(chunk: NativeLLMStreamChunk): void {
    const success = this.buffer.push(chunk)

    if (!success) {
      // Handle backpressure by forcing immediate flush
      this.flush()
      this.buffer.push(chunk) // Try again after flush
    }

    // Immediate processing for critical chunks
    if (chunk.type === 'finish' || chunk.type === 'error') {
      this.flush()
    }
  }

  /**
   * Flush buffered chunks
   */
  private flush(): void {
    if (this.isProcessing || this.buffer.isEmpty()) {
      return
    }

    this.isProcessing = true
    try {
      const batch = this.buffer.shiftBatch()
      if (batch.length > 0) {
        this.onBatch(batch)
      }
    } finally {
      this.isProcessing = false
    }
  }

  /**
   * Start the flush timer
   */
  private startFlushTimer(interval: number): void {
    this.flushTimer = setInterval(() => {
      this.flush()
    }, interval)
  }

  /**
   * Stop the processor and clean up
   */
  stop(): void {
    if (this.flushTimer) {
      clearInterval(this.flushTimer)
      this.flushTimer = null
    }

    // Final flush
    this.flush()
    this.buffer.clear()
  }

  /**
   * Get buffer statistics
   */
  getStats() {
    return this.buffer.getStats()
  }
}
