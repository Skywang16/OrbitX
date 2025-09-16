/**
 * Plan提示词构建器
 * 专门用于构建规划系统提示词
 */

import config from '../../config'
import Context from '../../core/context'

import { PromptBuilder } from './prompt-builder'
import { resolveTemplate } from '../template-engine'
import type { Agent } from '../../agent'

/**
 * Plan提示词构建器
 */
export class PlanPromptBuilder extends PromptBuilder {
  /**
   * 构建规划系统提示词
   */
  async buildPlanSystemPrompt(context: Context): Promise<string> {
    const agent = context.agent as Agent & { ignorePlan?: boolean }
    const tools = await agent.loadTools(context)

    // 生成agent信息
    let agentPrompt = ''
    if (!agent.ignorePlan) {
      agentPrompt = `<agent name="${agent.Name}">
Description: ${agent.PlanDescription || agent.Description}
Tools:
${tools
  .filter(tool => !tool.noPlan)
  .map(tool => `  - ${tool.name}: ${tool.planDescription || tool.description || ''}`)
  .join('\n')}
</agent>`
    }

    // 生成示例
    const examplePrompt = this.generateExamples(agent.Name === 'Chat')

    // 使用模板
    const template = `You are {name}, an autonomous AI Task Planner.

## Task Description
Your task is to understand the user's requirements and plan the execution steps. Please follow the steps below:
1. Understand the user's requirements.
2. Analyze what tools and capabilities are needed based on the user's requirements.
3. Generate a step-by-step execution plan.
4. Use single node for simple tasks (greetings, basic questions). Use multiple nodes only when truly necessary.
5. Please strictly follow the output format and example output.
6. The output language should follow the language corresponding to the user's task.

## Planning Guidelines
- **Adaptive planning**: Match complexity to task - single node for simple tasks, multiple nodes for complex ones.
- **Sequential execution**: Break down complex tasks into logical sequential steps only when necessary.
- **Tool utilization**: Make use of available tools and capabilities.
- **Efficient planning**: Focus on the most direct path to complete the user's task.

## Agent Information
{agent}

## Output Rules and Format
<root>
  <!-- Task Name (Short) -->
  <name>Task Name</name>
  <!-- Think step by step and output a detailed thought process for task planning. -->
  <thought>Your thought process on user demand planning</thought>
  <!-- Execution plan -->
  <agent name="{agentName}">
    <!-- Task description for the agent -->
    <task>Describe what the agent needs to accomplish</task>
    <nodes>
      <!-- Each node represents a specific step in task execution. Context is preserved through conversation history. -->
      <node>Complete the corresponding step nodes of the task</node>
      <!-- When including duplicate tasks, \`forEach\` can be used -->
      <forEach items="list">
        <node>forEach step node</node>
      </forEach>
      <!-- When you need to monitor changes in webpage DOM elements, you can use \`Watch\`, the loop attribute specifies whether to listen in a loop or listen once. -->
      <watch event="dom" loop="true">
        <description>Monitor task description</description>
        <trigger>
          <node>Trigger step node</node>
          <node>...</node>
        </trigger>
      </watch>
    </nodes>
  </agent>
</root>

{examplePrompt}`

    return resolveTemplate(template, {
      name: agent.Name,
      agent: agentPrompt.trim(),
      agentName: agent.Name,
      examplePrompt,
    })
  }

  /**
   * 构建规划用户提示词
   */
  buildPlanUserPrompt(taskPrompt: string): string {
    const template = `User Platform: {platform}
Current datetime: {datetime}
Task Description: {taskPrompt}`

    return resolveTemplate(template, {
      platform: config.platform,
      datetime: new Date().toLocaleString(),
      taskPrompt,
    })
  }

  /**
   * 生成示例
   */
  private generateExamples(hasChatAgent: boolean): string {
    const chatExample = `User: hello.
Output result:
<root>
  <name>Chat</name>
  <thought>The user is greeting me with "hello". This is a simple greeting that requires a direct, friendly response without any complex planning.</thought>
  <agent name="Chat">
    <task>Respond to user's greeting</task>
    <nodes>
      <node>Generate a friendly greeting response</node>
    </nodes>
  </agent>
</root>`

    const examples = [
      `User: Who are you?
Output result:
<root>
  <name>Self Introduction</name>
  <thought>The user is asking about my identity. This is a simple, direct question that requires a straightforward response about who I am and what I can do. No complex planning or multiple steps needed.</thought>
  <agent name="Chat">
    <task>Introduce myself and explain my capabilities</task>
    <nodes>
      <node>Explain that I am Orbit, an AI assistant, and describe my main functions and capabilities</node>
    </nodes>
  </agent>
</root>`,
      `User: Create a backup script that compresses all project files in the current directory and saves them with timestamp.
Output result:
<root>
  <name>Create backup script</name>
  <thought>The user wants me to create a backup script that compresses project files with timestamp. This is a complex task involving multiple file operations and shell commands that need to be executed in sequence to achieve the goal.</thought>
  <agent name="Shell">
    <task>Create a backup script that compresses all project files in the current directory and saves them with timestamp.</task>
    <nodes>
      <node>Get current directory path</node>
      <node>Generate timestamp for backup filename</node>
      <node>Create tar.gz archive of all project files</node>
      <node>Verify backup file was created successfully</node>
      <node>Save backup file path for reference</node>
    </nodes>
  </agent>
</root>`,
    ]

    const exampleList = hasChatAgent ? [chatExample, ...examples] : examples
    let examplePrompt = ''

    for (let i = 0; i < exampleList.length; i++) {
      examplePrompt += `## Example ${i + 1}\n${exampleList[i]}\n\n`
    }

    return examplePrompt
  }
}

/**
 * 便捷函数：构建规划系统提示词
 */
export async function buildPlanSystemPrompt(context: Context): Promise<string> {
  const builder = new PlanPromptBuilder()
  return builder.buildPlanSystemPrompt(context)
}

/**
 * 便捷函数：构建规划用户提示词
 */
export function buildPlanUserPrompt(taskPrompt: string): string {
  const builder = new PlanPromptBuilder()
  return builder.buildPlanUserPrompt(taskPrompt)
}
