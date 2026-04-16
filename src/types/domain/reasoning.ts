/**
 * Reasoning/Thinking context types for multi-provider support.
 *
 * This module provides unified data structures for handling reasoning content
 * from different LLM providers (Anthropic Extended Thinking, OpenAI Reasoning).
 *
 * Design based on opencode-dev's reasoning architecture.
 */

/**
 * Unified reasoning part that works across all providers.
 */
export interface ReasoningPart {
  /** Unique identifier for this reasoning part */
  id: string
  /** Thread ID this part belongs to */
  threadId: number
  /** Message ID this part belongs to */
  messageId: string
  /** The reasoning/thinking text content */
  text: string
  /** Provider-specific metadata */
  metadata?: ReasoningMetadata
  /** Timing information */
  time: ReasoningTime
}

/**
 * Provider-specific metadata for reasoning parts.
 */
export interface ReasoningMetadata {
  /** OpenAI Responses API item ID (for item_reference) */
  itemId?: string
  /** OpenAI encrypted reasoning content */
  encryptedContent?: string
  /** Anthropic thinking signature for verification */
  signature?: string
  /** Provider identifier (e.g., "openai", "anthropic") */
  provider?: string
}

/**
 * Timing information for reasoning parts.
 */
export interface ReasoningTime {
  /** When reasoning started (Unix timestamp ms) */
  start: number
  /** When reasoning ended (Unix timestamp ms), undefined if still in progress */
  end?: number
}

/**
 * Stream event types for reasoning processing.
 */
export type ReasoningEvent = ReasoningStartEvent | ReasoningDeltaEvent | ReasoningEndEvent

export interface ReasoningStartEvent {
  type: 'reasoning_start'
  /** Unique ID for this reasoning block */
  id: string
  /** Provider-specific metadata */
  metadata?: ReasoningMetadata
}

export interface ReasoningDeltaEvent {
  type: 'reasoning_delta'
  /** ID of the reasoning block */
  id: string
  /** Text delta to append */
  delta: string
  /** Optional metadata update */
  metadata?: ReasoningMetadata
}

export interface ReasoningEndEvent {
  type: 'reasoning_end'
  /** ID of the reasoning block */
  id: string
  /** Final metadata (may include signature) */
  metadata?: ReasoningMetadata
}

/**
 * Helper functions for reasoning parts
 */
export const ReasoningUtils = {
  /**
   * Create a new reasoning part with start time.
   */
  create(id: string, threadId: number, messageId: string): ReasoningPart {
    return {
      id,
      threadId,
      messageId,
      text: '',
      time: {
        start: Date.now(),
      },
    }
  },

  /**
   * Check if reasoning is still in progress.
   */
  isStreaming(part: ReasoningPart): boolean {
    return part.time.end === undefined
  },

  /**
   * Get duration in milliseconds if completed.
   */
  durationMs(part: ReasoningPart): number | undefined {
    if (part.time.end === undefined) return undefined
    return part.time.end - part.time.start
  },

  /**
   * Get OpenAI item_id for item_reference.
   */
  getOpenAIItemId(part: ReasoningPart): string | undefined {
    return part.metadata?.itemId
  },

  /**
   * Get Anthropic signature.
   */
  getAnthropicSignature(part: ReasoningPart): string | undefined {
    return part.metadata?.signature
  },

  /**
   * Check if reasoning has OpenAI metadata.
   */
  isOpenAI(part: ReasoningPart): boolean {
    return part.metadata?.provider === 'openai'
  },

  /**
   * Check if reasoning has Anthropic metadata.
   */
  isAnthropic(part: ReasoningPart): boolean {
    return part.metadata?.provider === 'anthropic'
  },
}
