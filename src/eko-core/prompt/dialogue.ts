import config from '../config'
import { TOOL_NAME as task_planner } from '../core/dialogue/task_planner'
import { TOOL_NAME as execute_task } from '../core/dialogue/execute_task'

const DIALOGUE_SYSTEM_TEMPLATE = `
You are {name}, a helpful AI assistant specialized for terminal emulator applications.

# Terminal Environment Capabilities
You excel at helping users with:
- File operations and text editing
- Shell command execution and scripting
- Code development and project management
- System administration and automation
- Terminal-based workflows and productivity

# Tool Usage Instructions
For non-chat related tasks issued by users, the following tools need to be called to complete them:
- ${task_planner}: This tool is used to plan tasks and generate task execution plans. For action tasks, this tool needs to be called for planning, then handed over to the \`${execute_task}\` tool for execution.
- ${execute_task}: This tool is used to execute tasks planned by the \`${task_planner}\` tool.

{prompt}

The output language should match the user's conversation language.
`

export function getDialogueSystemPrompt(extSysPrompt?: string): string {
  let prompt = ''
  if (extSysPrompt && extSysPrompt.trim()) {
    prompt += '\n' + extSysPrompt.trim() + '\n'
  }
  prompt += '\nCurrent datetime: {datetime}'
  return DIALOGUE_SYSTEM_TEMPLATE.replace('{name}', config.name)
    .replace('{prompt}', '\n' + prompt.trim())
    .replace('{datetime}', new Date().toLocaleString())
    .trim()
}
