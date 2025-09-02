import config from '../config'
import Context from '../core/context'

const PLAN_SYSTEM_TEMPLATE = `
You are {name}, an autonomous AI Task Planner.

## Task Description
Your task is to understand the user's requirements and plan the execution steps. Please follow the steps below:
1. Understand the user's requirements.
2. Analyze what tools and capabilities are needed based on the user's requirements.
3. Generate a step-by-step execution plan.
4. You only need to provide the steps to complete the user's task, key steps only, no need to be too detailed.
5. Please strictly follow the output format and example output.
6. The output language should follow the language corresponding to the user's task.

## Planning Guidelines
- **Sequential execution**: Break down the task into logical sequential steps.
- **Tool utilization**: Make use of available tools and capabilities.
- **Context preservation**: Each step can reference results from previous steps.
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
  <agent name="{agent_name}">
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

{example_prompt}
`

const PLAN_CHAT_EXAMPLE = `User: hello.
Output result:
<root>
  <name>Chat</name>
  <thought>Alright, the user wrote "hello". That's pretty straightforward. I need to respond in a friendly and welcoming manner.</thought>
  <agent name="Chat">
    <task>Respond to user's greeting</task>
    <nodes>
      <node>Generate a friendly greeting response</node>
    </nodes>
  </agent>
</root>`

const PLAN_EXAMPLE_LIST = [
  `User: Create a backup script that compresses all project files in the current directory and saves them with timestamp.
Output result:
<root>
  <name>Create backup script</name>
  <thought>The user wants me to create a backup script that compresses project files with timestamp. This involves file operations and shell commands.</thought>
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
  `User: Find all Python files in the project, analyze their imports, and generate a dependency report.
Output result:
<root>
  <name>Python Dependency Analysis</name>
  <thought>The user wants to analyze Python files and their imports to generate a dependency report. This involves file operations and text processing.</thought>
  <agent name="File">
    <task>Find and analyze Python files for imports and generate dependency report</task>
    <nodes>
      <node>Search for all .py files in the project directory</node>
      <node>Read each Python file content</node>
      <forEach items="python_files">
        <node>Extract import statements from file</node>
        <node>Parse import dependencies</node>
      </forEach>
      <node>Compile dependency information</node>
      <node>Process dependency data and format report</node>
      <node>Save report to dependency_report.txt</node>
    </nodes>
  </agent>
</root>`,
  `User: Monitor system logs for error patterns and create an automated alert system.
Output result:
<root>
  <name>System Log Monitor and Alert System</name>
  <thought>The user wants to monitor system logs for error patterns and create alerts. This involves file monitoring, text processing, and system operations.</thought>
  <agent name="Shell">
    <task>Set up log monitoring and pattern detection</task>
    <nodes>
      <node>Identify system log file locations</node>
      <node>Create log monitoring script</node>
      <node>Define error patterns to watch for</node>
      <node output="logPatterns">Save pattern definitions</node>
      <forEach items="log_files">
        <node>Set up file monitoring for each log</node>
        <node>Configure pattern matching rules</node>
      </forEach>
      <node>Test monitoring system functionality</node>
    </nodes>
  </agent>
</root>`,
  `User: Set up a development environment with Node.js, install project dependencies, and run tests.
Output result:
<root>
  <name>Development Environment Setup</name>
  <thought>The user wants to set up a development environment with Node.js, install dependencies, and run tests. This involves shell commands and file operations.</thought>
  <agent name="Shell">
    <task>Set up Node.js development environment</task>
    <nodes>
      <node>Check if Node.js is installed</node>
      <node>Install Node.js if not present</node>
      <node>Verify npm is available</node>
      <node>Navigate to project directory</node>
      <node>Install project dependencies using npm install</node>
      <node>Run project tests using npm test</node>
      <node>Generate test coverage report</node>
    </nodes>
  </agent>
</root>`,
  `User: Analyze code quality across multiple programming languages in a project, generate reports, and set up automated code formatting.
Output result:
<root>
<name>Code Quality Analysis and Formatting Setup</name>
<thought>The user wants to analyze code quality across multiple programming languages, generate reports, and set up automated formatting. This involves file operations, shell commands, and text processing.</thought>
<agent name="File">
  <task>Analyze code quality and set up automated formatting</task>
  <nodes>
    <node>Identify all source code files in the project</node>
    <node>Categorize files by programming language</node>
    <node>Count lines of code for each language</node>
    <node>Analyze file structure and organization</node>
    <node>Install necessary code analysis tools (eslint, pylint, etc.)</node>
    <node>Run language-specific linters and analyzers</node>
    <node>Collect code quality metrics and issues</node>
    <node>Install code formatters (prettier, black, gofmt, etc.)</node>
    <node>Configure formatting rules for each language</node>
    <node>Create pre-commit hooks for automatic formatting</node>
    <node>Compile analysis results into comprehensive report</node>
    <node>Create summary statistics and recommendations</node>
    <node>Save report as 'Code_Quality_Analysis_Report.md'</node>
  </nodes>
</agent>
</root>`,
]

const PLAN_USER_TEMPLATE = `
User Platform: {platform}
Current datetime: {datetime}
Task Description: {task_prompt}
`

const PLAN_USER_TASK_WEBSITE_TEMPLATE = `
User Platform: {platform}
Task Website: {task_website}
Current datetime: {datetime}
Task Description: {task_prompt}
`

export async function getPlanSystemPrompt(context: Context): Promise<string> {
  let agent_prompt = ''
  let agent = context.agent
  let tools = await agent.loadTools(context)

  // Generate agent prompt if agent should be included in planning
  if (!(agent as any).ignorePlan) {
    agent_prompt +=
      `<agent name="${agent.Name}">\n` +
      `Description: ${agent.PlanDescription || agent.Description}\n` +
      'Tools:\n' +
      tools
        .filter((tool: any) => !tool.noPlan)
        .map((tool: any) => `  - ${tool.name}: ${tool.planDescription || tool.description || ''}`)
        .join('\n') +
      '\n</agent>\n\n'
  }
  let plan_example_list = PLAN_EXAMPLE_LIST
  let hasChatAgent = agent.Name == 'Chat'
  let example_prompt = ''
  const example_list = hasChatAgent ? [PLAN_CHAT_EXAMPLE, ...plan_example_list] : [...plan_example_list]
  for (let i = 0; i < example_list.length; i++) {
    example_prompt += `## Example ${i + 1}\n${example_list[i]}\n\n`
  }
  return PLAN_SYSTEM_TEMPLATE.replace('{name}', config.name)
    .replace('{agent}', agent_prompt.trim())
    .replace('{agent_name}', agent.Name)
    .replace('{example_prompt}', example_prompt)
    .trim()
}

export function getPlanUserPrompt(task_prompt: string): string {
  const prompt = PLAN_USER_TEMPLATE.replace('{task_prompt}', task_prompt)
    .replace('{platform}', config.platform)
    .replace('{datetime}', new Date().toLocaleString())
    .trim()
  return prompt
}
