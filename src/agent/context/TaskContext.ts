/**
 * @file TaskContext.ts
 * @description Defines the global context for a single task execution lifecycle.
 */

import { WorkflowDefinition } from '../types/workflow'
import { MemoryManager } from '../core/MemoryManager'
import { AgentFrameworkConfig } from '..'

/**
 * Represents the global context for a task.
 * It holds all the state and configuration for the duration of a task execution.
 */
export class TaskContext {
  public readonly taskId: string
  public readonly config: AgentFrameworkConfig
  public readonly memory: MemoryManager
  public workflow?: WorkflowDefinition
  public variables: Map<string, any> = new Map()
  public readonly controller: AbortController

  constructor(
    taskId: string,
    config: AgentFrameworkConfig,
    workflow?: WorkflowDefinition,
    initialVariables?: Record<string, any>
  ) {
    this.taskId = taskId
    this.config = config
    this.workflow = workflow
    this.controller = new AbortController()

    if (initialVariables) {
      for (const [key, value] of Object.entries(initialVariables)) {
        this.variables.set(key, value)
      }
    }

    // Each task context gets its own memory manager, which knows about its parent context
    this.memory = new MemoryManager({}, {}, this)
  }

  /**
   * Sets a global variable for the task.
   * @param key - The variable key.
   * @param value - The variable value.
   */
  public setVariable(key: string, value: any): void {
    this.variables.set(key, value)
  }

  /**
   * Gets a global variable.
   * @param key - The variable key.
   * @returns The value of the variable, or undefined if not found.
   */
  public getVariable<T = any>(key: string): T | undefined {
    return this.variables.get(key) as T | undefined
  }

  /**
   * Aborts the task execution.
   */
  public abort(): void {
    this.controller.abort()
  }
}
