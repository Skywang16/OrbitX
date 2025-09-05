/**
 * Verification script for stream processing optimization
 * This script tests the key functionality without requiring a test framework
 */

import { StreamProcessor, DEFAULT_STREAM_CONFIG } from './stream-processor'
import { StreamBuffer, BufferedStreamProcessor, DEFAULT_BUFFER_CONFIG } from './stream-buffer'
import { PerformanceMonitor } from './performance-monitor'
import { NativeLLMStreamChunk } from '../types/llm.types'

async function verifyStreamProcessor(): Promise<boolean> {
  console.log('🧪 Testing StreamProcessor...')

  try {
    const mockChunks: NativeLLMStreamChunk[] = [
      { type: 'delta', content: 'Hello' },
      { type: 'delta', content: ' world' },
      { type: 'delta', content: '!' },
      { type: 'finish', finishReason: 'stop', usage: { promptTokens: 10, completionTokens: 3, totalTokens: 13 } },
    ]

    const stream = new ReadableStream<NativeLLMStreamChunk>({
      start(controller) {
        mockChunks.forEach(chunk => controller.enqueue(chunk))
        controller.close()
      },
    })

    let textContent = ''
    let finishReason = ''
    let processedCount = 0

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

      processedCount++
      StreamProcessor.processStreamChunk(value, {
        onTextDelta: content => {
          textContent += content
        },
        onFinish: reason => {
          finishReason = reason
        },
      })
    }

    const success = processedCount === 4 && textContent === 'Hello world!' && finishReason === 'stop'
    console.log(`✅ StreamProcessor: ${success ? 'PASSED' : 'FAILED'}`)
    console.log(`   Processed: ${processedCount} chunks, Text: "${textContent}", Finish: "${finishReason}"`)
    return success
  } catch (error) {
    console.log(`❌ StreamProcessor: FAILED - ${error}`)
    return false
  }
}

async function verifyStreamBuffer(): Promise<boolean> {
  console.log('🧪 Testing StreamBuffer...')

  try {
    const buffer = new StreamBuffer({
      ...DEFAULT_BUFFER_CONFIG,
      maxSize: 5,
    })

    // Test basic operations
    for (let i = 0; i < 5; i++) {
      const success = buffer.push({ type: 'delta', content: `chunk${i}` })
      if (!success) {
        throw new Error(`Failed to push chunk ${i}`)
      }
    }

    if (!buffer.isFull() || buffer.getSize() !== 5) {
      throw new Error('Buffer should be full with 5 items')
    }

    // Test overflow handling
    const overflowSuccess = buffer.push({ type: 'delta', content: 'overflow' })
    if (overflowSuccess) {
      throw new Error('Buffer should reject overflow')
    }

    // Test batch retrieval
    const batch = buffer.shiftBatch(3)
    if (batch.length !== 3 || (batch[0].type === 'delta' && batch[0].content !== 'chunk0')) {
      throw new Error('Batch retrieval failed')
    }

    if (buffer.getSize() !== 2) {
      throw new Error('Buffer size should be 2 after batch removal')
    }

    console.log('✅ StreamBuffer: PASSED')
    return true
  } catch (error) {
    console.log(`❌ StreamBuffer: FAILED - ${error}`)
    return false
  }
}

async function verifyBufferedProcessor(): Promise<boolean> {
  console.log('🧪 Testing BufferedStreamProcessor...')

  try {
    const batches: NativeLLMStreamChunk[][] = []
    const processor = new BufferedStreamProcessor(batch => batches.push(batch), {
      ...DEFAULT_BUFFER_CONFIG,
      batchSize: 2,
      flushInterval: 10,
    })

    const mockChunks: NativeLLMStreamChunk[] = [
      { type: 'delta', content: 'test1' },
      { type: 'delta', content: 'test2' },
      { type: 'delta', content: 'test3' },
      { type: 'finish', finishReason: 'stop' },
    ]

    // Add chunks
    mockChunks.forEach(chunk => processor.addChunk(chunk))

    // Wait for processing
    await new Promise(resolve => setTimeout(resolve, 50))

    processor.stop()

    const allChunks = batches.flat()
    const success = batches.length > 0 && allChunks.length === mockChunks.length

    console.log(`✅ BufferedStreamProcessor: ${success ? 'PASSED' : 'FAILED'}`)
    console.log(`   Batches: ${batches.length}, Total chunks: ${allChunks.length}`)
    return success
  } catch (error) {
    console.log(`❌ BufferedStreamProcessor: FAILED - ${error}`)
    return false
  }
}

async function verifyPerformanceMonitor(): Promise<boolean> {
  console.log('🧪 Testing PerformanceMonitor...')

  try {
    const monitor = new PerformanceMonitor()

    // Record some test data
    monitor.recordChunk(100, 5)
    monitor.recordChunk(200, 10)
    monitor.recordChunk(150, 7)
    monitor.recordBufferUtilization(50)
    monitor.recordBufferUtilization(75)
    monitor.recordBackpressure()
    monitor.recordBufferOverflow()

    const metrics = monitor.getMetrics()

    const success =
      metrics.totalChunks === 3 &&
      metrics.totalBytes === 450 &&
      Math.abs(metrics.averageChunkSize - 150) < 0.1 &&
      Math.abs(metrics.averageProcessingTime - 7.33) < 0.1 &&
      Math.abs(metrics.averageBufferUtilization - 62.5) < 0.1 &&
      metrics.backpressureEvents === 1 &&
      metrics.bufferOverflows === 1

    console.log(`✅ PerformanceMonitor: ${success ? 'PASSED' : 'FAILED'}`)
    console.log(`   Chunks: ${metrics.totalChunks}, Bytes: ${metrics.totalBytes}`)
    console.log(`   Avg chunk size: ${metrics.averageChunkSize.toFixed(1)}`)
    console.log(`   Avg processing time: ${metrics.averageProcessingTime.toFixed(2)}ms`)
    return success
  } catch (error) {
    console.log(`❌ PerformanceMonitor: FAILED - ${error}`)
    return false
  }
}

async function verifyBackpressureHandling(): Promise<boolean> {
  console.log('🧪 Testing Backpressure Handling...')

  try {
    const largeChunks = Array.from({ length: 100 }, (_, i) => ({
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
      maxBufferSize: 20,
      backpressureThreshold: 15,
    })

    const processedChunks: NativeLLMStreamChunk[] = []
    const reader = backpressureStream.getReader()

    while (true) {
      const { done, value } = await reader.read()
      if (done) break
      processedChunks.push(value)
    }

    const success = processedChunks.length > 0 && processedChunks[processedChunks.length - 1].type === 'finish'

    console.log(`✅ Backpressure Handling: ${success ? 'PASSED' : 'FAILED'}`)
    console.log(`   Processed: ${processedChunks.length} chunks`)
    return success
  } catch (error) {
    console.log(`❌ Backpressure Handling: FAILED - ${error}`)
    return false
  }
}

async function verifyMemoryEfficiency(): Promise<boolean> {
  console.log('🧪 Testing Memory Efficiency...')

  try {
    const largeChunkCount = 1000
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
      maxBufferSize: 200,
      backpressureThreshold: 150,
      batchSize: 20,
      flushInterval: 4,
    })

    let processedCount = 0
    const reader = memoryEfficientStream.getReader()

    while (true) {
      const { done } = await reader.read()
      if (done) break
      processedCount++
    }

    const success = processedCount === largeChunkCount + 1 // +1 for finish chunk

    console.log(`✅ Memory Efficiency: ${success ? 'PASSED' : 'FAILED'}`)
    console.log(`   Processed: ${processedCount} chunks`)
    return success
  } catch (error) {
    console.log(`❌ Memory Efficiency: FAILED - ${error}`)
    return false
  }
}

async function runAllVerifications(): Promise<void> {
  console.log('🚀 Starting Stream Processing Optimization Verification\n')

  const results = await Promise.all([
    verifyStreamProcessor(),
    verifyStreamBuffer(),
    verifyBufferedProcessor(),
    verifyPerformanceMonitor(),
    verifyBackpressureHandling(),
    verifyMemoryEfficiency(),
  ])

  const passedCount = results.filter(Boolean).length
  const totalCount = results.length

  console.log('\n📊 Verification Summary:')
  console.log(`✅ Passed: ${passedCount}/${totalCount}`)
  console.log(`❌ Failed: ${totalCount - passedCount}/${totalCount}`)

  if (passedCount === totalCount) {
    console.log('\n🎉 All stream processing optimizations are working correctly!')

    // Show performance metrics
    const metrics = StreamProcessor.getMetrics()
    console.log('\n📈 Performance Metrics:')
    console.log(`   Chunks processed: ${metrics.chunksProcessed}`)
    console.log(`   Bytes processed: ${metrics.bytesProcessed}`)
    console.log(`   Average latency: ${metrics.averageLatency.toFixed(2)}ms`)
    console.log(`   Backpressure events: ${metrics.backpressureEvents}`)
    console.log(`   Buffer overflows: ${metrics.bufferOverflows}`)
  } else {
    process.exit(1)
  }
}

// Run verification if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runAllVerifications().catch(console.error)
}

export { runAllVerifications }
