import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { StreamProcessor, DEFAULT_STREAM_CONFIG } from '../stream-processor'
import { StreamBuffer, BufferedStreamProcessor, DEFAULT_BUFFER_CONFIG } from '../stream-buffer'
import { PerformanceMonitor } from '../performance-monitor'
import { NativeLLMStreamChunk } from '../../types/llm.types'

describe('Stream Processing Optimization', () => {
  let mockChunks: NativeLLMStreamChunk[]

  beforeEach(() => {
    mockChunks = [
      { type: 'delta', content: 'Hello' },
      { type: 'delta', content: ' world' },
      { type: 'delta', content: '!' },
      { type: 'finish', finishReason: 'stop', usage: { promptTokens: 10, completionTokens: 3, totalTokens: 13 } },
    ]
  })

  describe('StreamProcessor', () => {
    it('should process chunks with optimized handling', async () => {
      const processedChunks: NativeLLMStreamChunk[] = []
      let textContent = ''
      let finishReason = ''

      const stream = new ReadableStream<NativeLLMStreamChunk>({
        start(controller) {
          mockChunks.forEach(chunk => controller.enqueue(chunk))
          controller.close()
        },
      })

      const optimizedStream = StreamProcessor.createOptimizedStream(stream, {
        maxBufferSize: 100,
        batchSize: 2,
        flushInterval: 10,
        enableMetrics: true,
      })

      const reader = optimizedStream.getReader()
      while (true) {
        const { done, value } = await reader.read()
        if (done) break

        processedChunks.push(value)
        StreamProcessor.processStreamChunk(value, {
          onTextDelta: content => {
            textContent += content
          },
          onFinish: reason => {
            finishReason = reason
          },
        })
      }

      expect(processedChunks).toHaveLength(4)
      expect(textContent).toBe('Hello world!')
      expect(finishReason).toBe('stop')
    })

    it('should handle backpressure correctly', async () => {
      const largeChunks = Array.from({ length: 1000 }, (_, i) => ({
        type: 'delta' as const,
        content: `chunk${i}`,
      }))

      const stream = new ReadableStream<NativeLLMStreamChunk>({
        start(controller) {
          largeChunks.forEach(chunk => controller.enqueue(chunk))
          controller.enqueue({ type: 'finish', finishReason: 'stop' })
          controller.close()
        },
      })

      const backpressureStream = StreamProcessor.addBackpressure(stream, {
        ...DEFAULT_STREAM_CONFIG,
        maxBufferSize: 50,
        backpressureThreshold: 40,
      })

      const processedChunks: NativeLLMStreamChunk[] = []
      const reader = backpressureStream.getReader()

      while (true) {
        const { done, value } = await reader.read()
        if (done) break
        processedChunks.push(value)
      }

      // Should have processed all chunks despite backpressure
      expect(processedChunks.length).toBeGreaterThan(0)
      expect(processedChunks[processedChunks.length - 1].type).toBe('finish')
    })

    it('should accumulate text efficiently with batching', async () => {
      const stream = new ReadableStream<NativeLLMStreamChunk>({
        start(controller) {
          mockChunks.forEach(chunk => controller.enqueue(chunk))
          controller.close()
        },
      })

      const result = await StreamProcessor.accumulateText(stream, {
        ...DEFAULT_STREAM_CONFIG,
        batchSize: 2,
        maxBufferSize: 10,
      })

      expect(result.content).toBe('Hello world!')
      expect(result.finishReason).toBe('stop')
      expect(result.usage?.totalTokens).toBe(13)
    })
  })

  describe('StreamBuffer', () => {
    let buffer: StreamBuffer

    beforeEach(() => {
      buffer = new StreamBuffer({
        ...DEFAULT_BUFFER_CONFIG,
        maxSize: 5,
      })
    })

    it('should handle circular buffer operations', () => {
      // Fill buffer
      for (let i = 0; i < 5; i++) {
        expect(buffer.push({ type: 'delta', content: `chunk${i}` })).toBe(true)
      }

      expect(buffer.isFull()).toBe(true)
      expect(buffer.getSize()).toBe(5)

      // Test overflow handling
      expect(buffer.push({ type: 'delta', content: 'overflow' })).toBe(false)

      // Test batch retrieval
      const batch = buffer.shiftBatch(3)
      expect(batch).toHaveLength(3)
      expect(batch[0].content).toBe('chunk0')
      expect(buffer.getSize()).toBe(2)
    })

    it('should compress buffer when enabled', () => {
      const compressibleBuffer = new StreamBuffer({
        ...DEFAULT_BUFFER_CONFIG,
        maxSize: 10,
        enableCompression: true,
        compressionThreshold: 5,
      })

      // Add multiple text deltas that can be compressed
      for (let i = 0; i < 8; i++) {
        compressibleBuffer.push({ type: 'delta', content: `text${i} ` })
      }

      expect(compressibleBuffer.getSize()).toBe(8)

      // Trigger overflow to activate compression
      compressibleBuffer.push({ type: 'delta', content: 'overflow1' })
      compressibleBuffer.push({ type: 'delta', content: 'overflow2' })

      // Buffer should have been compressed
      expect(compressibleBuffer.getSize()).toBeLessThan(8)
    })

    it('should provide accurate statistics', () => {
      buffer.push({ type: 'delta', content: 'test' })
      buffer.push({ type: 'delta', content: 'test2' })

      const stats = buffer.getStats()
      expect(stats.size).toBe(2)
      expect(stats.maxSize).toBe(5)
      expect(stats.utilization).toBe(40)
      expect(stats.isEmpty).toBe(false)
      expect(stats.isFull).toBe(false)
    })
  })

  describe('BufferedStreamProcessor', () => {
    it('should process batches efficiently', async () => {
      const batches: NativeLLMStreamChunk[][] = []
      const processor = new BufferedStreamProcessor(batch => batches.push(batch), {
        ...DEFAULT_BUFFER_CONFIG,
        batchSize: 2,
        flushInterval: 10,
      })

      // Add chunks
      mockChunks.forEach(chunk => processor.addChunk(chunk))

      // Wait for processing
      await new Promise(resolve => setTimeout(resolve, 50))

      processor.stop()

      expect(batches.length).toBeGreaterThan(0)
      const allChunks = batches.flat()
      expect(allChunks.length).toBe(mockChunks.length)
    })
  })

  describe('PerformanceMonitor', () => {
    let monitor: PerformanceMonitor

    beforeEach(() => {
      monitor = new PerformanceMonitor()
    })

    it('should track performance metrics', () => {
      // Record some chunks
      monitor.recordChunk(100, 5)
      monitor.recordChunk(200, 10)
      monitor.recordChunk(150, 7)

      monitor.recordBufferUtilization(50)
      monitor.recordBufferUtilization(75)

      monitor.recordBackpressure()
      monitor.recordBufferOverflow()

      const metrics = monitor.getMetrics()

      expect(metrics.totalChunks).toBe(3)
      expect(metrics.totalBytes).toBe(450)
      expect(metrics.averageChunkSize).toBeCloseTo(150)
      expect(metrics.averageProcessingTime).toBeCloseTo(7.33, 1)
      expect(metrics.averageBufferUtilization).toBeCloseTo(62.5)
      expect(metrics.backpressureEvents).toBe(1)
      expect(metrics.bufferOverflows).toBe(1)
    })

    it('should provide performance insights', () => {
      // Simulate poor performance
      for (let i = 0; i < 100; i++) {
        monitor.recordChunk(1, 100) // High processing time
      }
      monitor.recordBufferOverflow()
      monitor.recordError()

      const insights = monitor.getInsights()
      expect(insights.length).toBeGreaterThan(0)
      expect(insights.some(insight => insight.includes('latency'))).toBe(true)
    })

    it('should reset metrics correctly', () => {
      monitor.recordChunk(100, 5)
      monitor.recordError()

      let metrics = monitor.getMetrics()
      expect(metrics.totalChunks).toBe(1)
      expect(metrics.totalErrors).toBe(1)

      monitor.reset()

      metrics = monitor.getMetrics()
      expect(metrics.totalChunks).toBe(0)
      expect(metrics.totalErrors).toBe(0)
    })
  })

  describe('Memory Efficiency', () => {
    it('should handle large streams without memory leaks', async () => {
      const largeChunkCount = 10000
      const chunks = Array.from({ length: largeChunkCount }, (_, i) => ({
        type: 'delta' as const,
        content: `Large chunk content ${i} with some substantial text to test memory usage`,
      }))

      const stream = new ReadableStream<NativeLLMStreamChunk>({
        start(controller) {
          chunks.forEach(chunk => controller.enqueue(chunk))
          controller.enqueue({ type: 'finish', finishReason: 'stop' })
          controller.close()
        },
      })

      const memoryEfficientStream = StreamProcessor.createMemoryEfficientStream(stream, {
        maxBufferSize: 50,
        batchSize: 10,
        flushInterval: 1,
      })

      let processedCount = 0
      const reader = memoryEfficientStream.getReader()

      while (true) {
        const { done, value } = await reader.read()
        if (done) break
        processedCount++
      }

      expect(processedCount).toBe(largeChunkCount + 1) // +1 for finish chunk
    })
  })

  describe('Error Handling', () => {
    it('should handle stream errors gracefully', async () => {
      const errorStream = new ReadableStream<NativeLLMStreamChunk>({
        start(controller) {
          controller.enqueue({ type: 'delta', content: 'test' })
          controller.enqueue({ type: 'error', error: 'Test error' })
          controller.close()
        },
      })

      let errorReceived = false
      const optimizedStream = StreamProcessor.createOptimizedStream(errorStream)
      const reader = optimizedStream.getReader()

      while (true) {
        const { done, value } = await reader.read()
        if (done) break

        if (value.type === 'error') {
          errorReceived = true
          expect(value.error).toBe('Test error')
        }
      }

      expect(errorReceived).toBe(true)
    })
  })
})
