/**
 * 规划器组件
 *
 * 负责将自然语言任务转换为JSON格式的工作流定义
 * 参考eko的Planner设计理念，专门负责规划而不参与执行
 */

import { llmManager } from '../llm/LLMProvider'
import type { IPlanner, LLMCallOptions, WorkflowDefinition } from '../types'
import { StepType } from '../types'
import { MemoryManager } from './MemoryManager'

/**
 * 规划结果 - 直接返回新的 WorkflowDefinition
 */
export interface PlanningResult {
  success: boolean
  workflow?: WorkflowDefinition
  error?: string
  reasoning?: string
  prompt: string
  rawResponse: string
}

/**
 * 规划器类
 *
 * 将自然语言转换为JSON工作流
 */
export class Planner implements IPlanner {
  private _templates: Map<string, string> = new Map()
  private memoryManager: MemoryManager

  constructor() {
    this.initializeTemplates()
    this.memoryManager = new MemoryManager()
  }

  /**
   * 规划新任务
   */
  async planTask(
    userInput: string,
    options?: {
      model?: string
      includeThought?: boolean
      useTemplate?: string
    }
  ): Promise<PlanningResult> {
    const prompt = this.generatePlanningPrompt(userInput, options)
    return this._executePlanning(prompt, options)
  }

  /**
   * 根据现有计划和新指令，重新规划任务
   */
  async replanTask(
    newUserInput: string,
    previousResult: PlanningResult,
    options?: {
      model?: string
      includeThought?: boolean
    }
  ): Promise<PlanningResult> {
    const prompt = this.generateReplanningPrompt(newUserInput, previousResult)
    return this._executePlanning(prompt, options)
  }

  /**
   * 执行规划的核心逻辑
   */
  private async _executePlanning(prompt: string, options?: { model?: string }): Promise<PlanningResult> {
    try {
      const llmOptions: LLMCallOptions = {
        model: options?.model || 'claude-3-sonnet',
        temperature: 0.1,
        maxTokens: 4000,
      }

      const response = await llmManager.call(prompt, llmOptions)
      const parseResult = this.parseWorkflowJSON(response.content)

      if (parseResult.success && parseResult.workflow) {
        return {
          success: true,
          workflow: parseResult.workflow,
          reasoning: parseResult.workflow.thought || '',
          prompt: prompt,
          rawResponse: response.content,
        }
      } else {
        return {
          success: false,
          error: parseResult.error || '工作流解析失败',
          prompt: prompt,
          rawResponse: response.content,
        }
      }
    } catch (error) {
      const errorMessage = `规划失败: ${error instanceof Error ? error.message : String(error)}`
      return {
        success: false,
        error: errorMessage,
        prompt: prompt,
        rawResponse: '',
      }
    }
  }

  /**
   * 生成初次规划的提示词 - 简化版，参考eko但使用JSON
   */
  private generatePlanningPrompt(
    userInput: string,
    options?: { includeThought?: boolean; useTemplate?: string }
  ): string {
    const includeThought = options?.includeThought ?? true

    return `作为专业的任务规划器，请将用户的自然语言任务转换为JSON格式的工作流。

用户任务: "${userInput}"

请分析任务需求，将任务分解为多个AI Agent来执行，每个Agent都是通过LLM模型来处理任务的智能助手。

可用的Agent类型：
- Terminal: 专门处理终端命令执行和系统操作
- File: 专门处理文件和目录操作
- System: 专门处理系统信息查询和监控
- Network: 专门处理网络相关操作
- General: 通用AI助手，处理其他类型任务

JSON工作流格式：
\`\`\`json
{
  "taskId": "task_unique_id",
  "name": "工作流名称",
  ${includeThought ? '"thought": "详细的分析思考过程，说明如何分解任务和安排Agent",' : ''}
  "agents": [
    {
      "id": "agent_1",
      "name": "Agent显示名称",
      "task": "该Agent要完成的具体任务描述，要详细清晰，因为LLM会根据这个描述来执行任务",
      "type": "Terminal",
      "dependsOn": [], // 依赖的其他Agent ID数组，如果需要等待其他Agent完成才能执行
      "parallel": false, // 是否可以与其他Agent并行执行
      "status": "init",
      "config": {
        "model": "claude-3-sonnet", // 可选：指定使用的LLM模型
        "context": "任何需要传递给Agent的额外上下文信息"
      }
    }
  ],
  "variables": {}, // 工作流级别的变量
  "taskPrompt": "${userInput}"
}
\`\`\`

重要规划原则：
1. 每个Agent的task字段要非常详细和具体，因为LLM会直接根据这个描述来执行
2. 合理分解任务，避免单个Agent任务过于复杂
3. 通过dependsOn数组指定Agent间的依赖关系，确保执行顺序正确
4. 独立的任务可以设置parallel为true来并行执行，提高效率
5. 在config中可以指定特殊配置，如使用的模型、上下文信息等
6. Agent的name要简洁明了，task要详细具体

请生成完整的JSON工作流：`
  }

  /**
   * 生成重规划的提示词
   */
  private generateReplanningPrompt(newUserInput: string, previousResult: PlanningResult): string {
    return `你是一个任务规划器。你之前为用户生成了一个计划，现在用户给出了新的指令。\n请根据新的指令，对原有的计划进行修改和完善。\n\n这是你上次生成的计划（作为背景知识）:\n\`\`\`json\n${previousResult.rawResponse}\n\`\`\`\n\n这是用户最新的指令:\n"${newUserInput}"\n\n请充分理解用户的意图，生成一个【完整并且更新后】的JSON工作流。\n不要只生成修改的部分，我需要一个可以直接执行的、全新的完整JSON。\n请严格按照之前的JSON格式要求输出。`
  }

  /**
   * 解析JSON工作流
   */
  private parseWorkflowJSON(jsonContent: string): {
    success: boolean
    workflow?: WorkflowDefinition
    error?: string
  } {
    try {
      const jsonMatch = jsonContent.match(/\{[\s\S]*\}/i)
      if (!jsonMatch) {
        return {
          success: false,
          error: '未找到有效的JSON工作流定义',
        }
      }

      const jsonStr = jsonMatch[0]
      const workflow = JSON.parse(jsonStr) as WorkflowDefinition

      // 基本验证，不转换
      if (!workflow.id) workflow.id = `workflow_${Date.now()}`
      if (!workflow.name) workflow.name = '未命名工作流'
      if (!workflow.agents) workflow.agents = []

      return {
        success: true,
        workflow,
      }
    } catch (error) {
      return {
        success: false,
        error: `JSON解析错误: ${error instanceof Error ? error.message : String(error)}`,
      }
    }
  }

  /**
   * 初始化工作流模板
   */
  private initializeTemplates(): void {
    this._templates.set(
      'terminal',
      `{
  "id": "terminal_template",
  "name": "终端操作模板",
  "description": "执行终端命令的标准模板",
  "thought": "用户需要执行终端命令，使用Terminal Agent来处理",
  "executionStrategy": "parallel",
  "agents": [
    {
      "id": "terminal_agent",
      "name": "Terminal Agent",
      "type": "Terminal",
      "description": "执行终端命令",
      "steps": [
        {
          "id": "execute_command",
          "type": "TOOL_CALL",
          "name": "执行命令",
          "description": "执行指定的终端命令",
          "config": {
            "toolId": "terminal_execute",
            "parameters": {
              "command": "要执行的命令"
            }
          },
          "priority": 1,
          "canRunInParallel": true
        }
      ]
    }
  ],
  "variables": {},
  "maxConcurrency": 3,
  "expectedOutput": "命令执行结果"
}`
    )

    // 文件操作模板
    this._templates.set(
      'file',
      `{
  "id": "file_template",
  "name": "文件操作模板",
  "description": "文件和目录操作的标准模板",
  "thought": "用户需要进行文件操作，使用File Agent来处理",
  "executionStrategy": "parallel",
  "agents": [
    {
      "id": "file_agent",
      "name": "File Agent",
      "type": "File",
      "description": "执行文件操作",
      "steps": [
        {
          "id": "file_operation",
          "type": "TOOL_CALL",
          "name": "文件操作",
          "description": "执行文件操作",
          "config": {
            "toolId": "file_manager",
            "parameters": {
              "operation": "操作类型",
              "path": "文件路径"
            }
          },
          "priority": 1,
          "canRunInParallel": true
        }
      ]
    }
  ],
  "variables": {},
  "maxConcurrency": 3,
  "expectedOutput": "文件操作结果"
}`
    )
  }

  /**
   * 获取可用模板列表
   */
  getAvailableTemplates(): string[] {
    return Array.from(this._templates.keys())
  }

  /**
   * 添加自定义模板
   */
  addTemplate(name: string, template: string): void {
    this._templates.set(name, template)
  }

  /**
   * 验证工作流定义
   */
  validateWorkflow(workflow: WorkflowDefinition): {
    valid: boolean
    errors: string[]
  } {
    const errors: string[] = []

    // 基本验证
    if (!workflow.id) errors.push('工作流ID不能为空')
    if (!workflow.name) errors.push('工作流名称不能为空')
    if (!workflow.agents || workflow.agents.length === 0) {
      errors.push('至少需要一个Agent')
    }

    // 验证每个Agent
    workflow.agents?.forEach((agent, agentIndex) => {
      if (!agent.type) errors.push(`Agent ${agentIndex + 1} 缺少类型`)
      if (!agent.steps || agent.steps.length === 0) {
        errors.push(`Agent ${agent.type} 缺少执行步骤`)
      }

      // 验证步骤
      const stepIds = new Set<string>()
      agent.steps?.forEach((step, stepIndex) => {
        if (!step.id) {
          errors.push(`Agent ${agent.type} 的步骤 ${stepIndex + 1} 缺少ID`)
        } else if (stepIds.has(step.id)) {
          errors.push(`Agent ${agent.type} 中存在重复的步骤ID: ${step.id}`)
        } else {
          stepIds.add(step.id)
        }

        if (!step.type) {
          errors.push(`步骤 ${step.id} 缺少type定义`)
        }

        // 验证工具调用配置
        if (step.type === StepType.TOOL_CALL) {
          if (!step.config?.toolId) {
            errors.push(`工具调用步骤 ${step.id} 缺少toolId配置`)
          }
        }
        // 验证依赖关系
        if (step.dependsOn) {
          step.dependsOn.forEach(dependency => {
            const depStepId = dependency.stepId
            // 这里需要在所有Agent的步骤中查找
            const allStepIdsInWorkflow = workflow.agents.flatMap(a => a.steps.map(s => s.id))
            if (!allStepIdsInWorkflow.includes(depStepId)) {
              errors.push(`步骤 ${step.id} 依赖的步骤 ${depStepId} 不存在`)
            }
          })
        }
      })
    })

    // 验证依赖图的有效性 (一个简单的检查)
    const allStepIds = workflow.agents.flatMap(a => a.steps.map(s => s.id))
    if (allStepIds.length > 0) {
      const hasRootSteps = workflow.agents.some(agent =>
        agent.steps.some(step => !step.dependsOn || step.dependsOn.length === 0)
      )

      if (!hasRootSteps) {
        errors.push('工作流中没有找到任何根步骤（即无任何依赖的步骤），这很可能意味着存在循环依赖。')
      }
    }

    return {
      valid: errors.length === 0,
      errors,
    }
  }

  /**
   * 获取Memory管理器
   */
  getMemoryManager(): MemoryManager {
    return this.memoryManager
  }
}
