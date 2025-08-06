/**
 * @file memory.ts
 * @description Defines the data structures for agent memory, including chat history and working memory.
 */

/**
 * The role of the message sender.
 */
export enum ChatMessageRole {
  USER = 'user',
  ASSISTANT = 'assistant',
  SYSTEM = 'system',
  TOOL = 'tool',
}

/**
 * Represents a single message in the chat history.
 */
export interface ChatMessage {
  role: ChatMessageRole
  content: string
}

/**
 * Represents the chat history of a session.
 */
export type ChatHistory = ChatMessage[]

/**
 * A key-value store for the agent's intermediate thoughts, observations, and results during a task.
 * This helps the agent to keep track of its state and progress.
 */
export type WorkingMemory = Record<string, any>

/**
 * Represents the complete memory of an agent for a given session.
 */
export interface Memory {
  /**
   * The history of conversation between the user and the agent.
   */
  chatHistory: ChatHistory

  /**
   * The agent's internal state and intermediate results for the current task.
   */
  workingMemory: WorkingMemory
}
