/**
 * Tree Plan提示词构建器
 * 生成包含多层 <subtasks> 的任务树结构
 */

import config from '../../config'
import Context from '../../core/context'
import { PromptBuilder } from './prompt-builder'
import { resolveTemplate } from '../template-engine'

export class TreePlanPromptBuilder extends PromptBuilder {
  async buildTreePlanSystemPrompt(_context: Context): Promise<string> {
    const template = `You are an autonomous Task Tree Planner.

Goal: Based on the user's requirement, create a COMPLETE yet COMPACT plan with EXACTLY TWO LEVELS under the root:
- Level 1: Main Task Groups (logical phases such as Requirements, Design, Implementation, Validation)
- Level 2: Subtasks (concrete, executable units)

Inspiration (do NOT output separate docs):
- Follow the essence of "Requirements Planning Standards":
  - Requirements thinking: user stories and acceptance criteria (EARS style) should inform task objectives.
  - Design thinking: technical choices and component boundaries should influence grouping.
  - Tasks thinking: break work into executable subtasks with verifiable outcomes and clear dependencies.

Hard Constraints:
- Two levels ONLY. Do NOT nest <subtasks> under a subtask.
- Every <task> MUST have reasonable <name> and <task> content.
- Subtasks MUST include 5–8 concrete steps in <nodes> and should be actionable (e.g., edit specific files, run specific commands, add tests, update docs).
- Indicate requirement mapping and dependencies in the <task> text using plain text, e.g., "Req: 1.2, 2.1" and "Depends on: 2.1".
- Prefer fewer, well-formed tasks over excessive fragmentation.
- Output language MUST match the user's language.

Acceptance Criteria (EARS-inspired, embed succinctly in <task> text of each Subtask):
- Ubiquitous: "The system shall ..."
- Event-driven: "When <trigger>, then the system shall ..."
- State-driven: "While <state>, the system shall ..."
- Unwanted behavior: "If <exception>, then the system shall ..."
- Optional: "Where <feature>, the system shall ..."

Output Format (TWO levels only):
<root>
  <name>Root Task Name</name>
  <thought>High-level plan and rationale (phasing, risk, and quality gates)</thought>
  <task>Overall objective and scope. Optionally note the key requirements being addressed, e.g., "Req: 1.1–1.4"</task>
  <subtasks>
    <!-- Level 1: Main Task Group (phase) -->
    <task>
      <name>Phase A · Requirements Clarification</name>
      <task>Scope and goals for this phase. Req: 1.1–1.3</task>
      <subtasks>
        <!-- Level 2: Subtasks (no deeper nesting) -->
        <task>
          <name>Define acceptance criteria</name>
          <task>Use EARS to define acceptance checks. Depends on: none. Req: 1.2</task>
          <nodes>
            <node>Draft acceptance criteria (EARS) for normal, exception, and boundary cases</node>
            <node>Review criteria with requirement examples</node>
            <node>Finalize checklist for validation</node>
          </nodes>
        </task>
      </subtasks>
    </task>
    <task>
      <name>Phase B · Implementation</name>
      <task>Implement changes with tests and docs. Req: 2.1–2.4</task>
      <subtasks>
        <task>
          <name>Implement feature module</name>
          <task>Implement code and unit tests. Depends on: Phase A outputs. Req: 2.1, 2.2</task>
          <nodes>
            <node>Create/modify files at specific paths with minimal diffs</node>
            <node>Add unit tests that assert the EARS acceptance criteria</node>
            <node>Run tests and ensure they pass locally</node>
            <node>Update docs if needed</node>
          </nodes>
        </task>
        <task>
          <name>Integration validation</name>
          <task>Verify end-to-end behavior. Depends on: Implement feature module. Req: 2.3</task>
          <nodes>
            <node>Execute integration flow (list exact commands)</node>
            <node>Validate responses/logs against acceptance criteria</node>
            <node>Adjust code/tests if gaps found</node>
          </nodes>
        </task>
      </subtasks>
    </task>
  </subtasks>
</root>`

    return resolveTemplate(template, {})
  }

  buildTreePlanUserPrompt(taskPrompt: string): string {
    const template = `User Platform: {platform}
Current datetime: {datetime}
Requirement: {taskPrompt}`

    return resolveTemplate(template, {
      platform: config.platform,
      datetime: new Date().toLocaleString(),
      taskPrompt,
    })
  }
}

export async function buildTreePlanSystemPrompt(context: Context): Promise<string> {
  const builder = new TreePlanPromptBuilder()
  return builder.buildTreePlanSystemPrompt(context)
}

export function buildTreePlanUserPrompt(taskPrompt: string): string {
  const builder = new TreePlanPromptBuilder()
  return builder.buildTreePlanUserPrompt(taskPrompt)
}
