import { Agent } from '../agent'
import config from '../config'
import Context from '../core/context'
import { Task, Tool } from '../types'
import { buildAgentRootXml } from '../common/xml'
import { TOOL_NAME as foreach_task } from '../tools/foreach_task'
import { TOOL_NAME as watch_trigger } from '../tools/watch_trigger'
import { TOOL_NAME as human_interact } from '../tools/human_interact'
import { TOOL_NAME as task_node_status } from '../tools/task_node_status'

const AGENT_SYSTEM_TEMPLATE = `
You are {name}, an autonomous AI agent.

# Agent Description
{description}
{prompt}

# Execution Rules
- EXECUTE your currentTask immediately without asking for clarification
- BUILD upon previous work, use conversation history to understand context
- Complete tasks step by step following the provided nodes

# Task Instructions
<root>
  <mainTask>main task</mainTask>
  <currentTask>specific task</currentTask>
  <nodes>
    <node status="todo / done">task step node</node>{nodePrompt}
  </nodes>
</root>

The output language should follow the language corresponding to the user's task.
`

const HUMAN_PROMPT = `
* HUMAN INTERACT
During the task execution process, you can use the \`${human_interact}\` tool to interact with humans, please call it in the following situations:
- When performing dangerous operations such as deleting files, confirmation from humans is required.
- When encountering obstacles such as requiring user input, password confirmation, or manual verification, you need to request manual assistance.
- Please do not use the \`${human_interact}\` tool frequently.
`

const FOR_EACH_NODE = `
    <!-- duplicate task node, items support list and variable -->
    <forEach items="list or variable name">
      <node>forEach item step node</node>
    </forEach>`

const FOR_EACH_PROMPT = `
* forEach node
repetitive tasks, when executing to the forEach node, require the use of the \`${foreach_task}\` tool.
`

const WATCH_NODE = `
    <!-- monitor task node, the loop attribute specifies whether to listen in a loop or listen once -->
    <watch event="file" loop="true">
      <description>Monitor task description</description>
      <trigger>
        <node>Trigger step node</node>
        <node>...</node>
      </trigger>
    </watch>`

const WATCH_PROMPT = `
* watch node
monitor changes in files, directories, or system events, when executing to the watch node, require the use of the \`${watch_trigger}\` tool.
`

export async function getAgentSystemPrompt(
  agent: Agent,
  task?: Task,
  _context?: Context,
  tools?: Tool[],
  extSysPrompt?: string
): Promise<string> {
  let prompt = ''
  let nodePrompt = ''
  tools = tools || agent.Tools
  let taskXml = task?.xml || ''
  let hasWatchNode = taskXml.indexOf('</watch>') > -1
  let hasForEachNode = taskXml.indexOf('</forEach>') > -1
  let hasHumanTool = tools.filter(tool => tool.name == human_interact).length > 0
  if (hasHumanTool) {
    prompt += HUMAN_PROMPT
  }
  if (hasForEachNode) {
    if (tools.filter(tool => tool.name == foreach_task).length > 0) {
      prompt += FOR_EACH_PROMPT
    }
    nodePrompt += FOR_EACH_NODE
  }
  if (hasWatchNode) {
    if (tools.filter(tool => tool.name == watch_trigger).length > 0) {
      prompt += WATCH_PROMPT
    }
    nodePrompt += WATCH_NODE
  }
  if (extSysPrompt && extSysPrompt.trim()) {
    prompt += '\n' + extSysPrompt.trim() + '\n'
  }
  prompt += '\nCurrent datetime: {datetime}'

  return AGENT_SYSTEM_TEMPLATE.replace('{name}', config.name)
    .replace('{agent}', agent.Name)
    .replace('{description}', agent.Description)
    .replace('{prompt}', '\n' + prompt.trim())
    .replace('{nodePrompt}', nodePrompt)
    .replace('{datetime}', new Date().toLocaleString())
    .trim()
}

export function getAgentUserPrompt(agent: Agent, task?: Task, context?: Context, tools?: Tool[]): string {
  const hasTaskNodeStatusTool = (tools || agent.Tools).filter(tool => tool.name == task_node_status).length > 0
  return buildAgentRootXml(task?.xml || '', context?.chain.taskPrompt || '', (_nodeId, node) => {
    if (hasTaskNodeStatusTool) {
      node.setAttribute('status', 'todo')
    }
  })
}
