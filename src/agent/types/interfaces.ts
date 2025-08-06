/**
 * @file Defines the core interfaces for pluggable components within the Agent framework.
 * This promotes loose coupling and allows for different implementations of key components.
 */

import type { PlanningResult } from './execution'
import type { ExecutionResult, WorkflowDefinition, ExecutionEvent } from './workflow'

/**
 * Defines the contract for a Planner component.
 * The Planner is responsible for converting a user's task into an executable workflow.
 */
export interface IPlanner {
  planTask(
    userInput: string,
    options?: {
      model?: string
      includeThought?: boolean
    }
  ): Promise<PlanningResult>
}

/**
 * Defines the contract for an Execution Engine component.
 * The Engine is responsible for running a workflow and producing a result.
 */
export interface IExecutionEngine {
  execute(
    workflow: WorkflowDefinition,
    contextParams?: Record<string, unknown>,
    callback?: (event: ExecutionEvent) => Promise<void>
  ): Promise<ExecutionResult>
}
