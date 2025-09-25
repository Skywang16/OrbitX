// Use browser DOM APIs directly
import { Task, TaskNode, TaskTextNode } from '../types/core.types'
import type { PlannedTask, PlannedTaskTree } from '../types/core.types'

// All Workflow-related functions removed - Task-only architecture

export function parseTask(taskId: string, xml: string, done: boolean): Task | null {
  // Parse XML directly for single-agent task mode
  try {
    const parser = new DOMParser()
    const xmlToParse = xml.trim().startsWith('<root') ? xml : `<root>${xml}</root>`
    const doc = parser.parseFromString(xmlToParse, 'text/xml')
    const root = doc.documentElement

    if (root.tagName !== 'root') {
      return null
    }

    const nameElement = root.querySelector('name')
    const thoughtElement = root.querySelector('thought')
    const agentElement = root.querySelector('agent')

    const thought = thoughtElement?.textContent?.trim() || ''

    let description = ''
    let nodes: TaskNode[] = []
    let status: 'init' | 'running' | 'done' | 'error' = 'init'
    let agentXml = ''

    if (agentElement) {
      const taskElement = agentElement.querySelector('task')
      const taskText = taskElement?.textContent?.trim() || ''
      const name = nameElement?.textContent?.trim() || taskText || 'Untitled Task'
      description = taskText || name

      // Parse nodes from agent
      const nodeElements = agentElement.querySelectorAll('nodes > node, node')
      nodes = Array.from(nodeElements).map((nodeEl: Element): TaskNode => {
        const nodeText = nodeEl.textContent?.trim() || ''
        return {
          type: 'normal',
          text: nodeText,
        } as TaskTextNode
      })

      agentXml = agentElement.outerHTML
      status = done ? 'done' : 'init'
    } else {
      // Fallback: treat provided xml/name as description
      const nameText = nameElement?.textContent?.trim() || ''
      description = nameText || 'Untitled Task'
      agentXml = xml
    }

    // name fallback when agentElement missing
    const fallbackName = nameElement?.textContent?.trim() || description || 'Untitled Task'

    return {
      taskId,
      name: fallbackName,
      thought,
      description,
      nodes,
      status,
      xml: agentXml,
      modified: false,
      taskPrompt: undefined,
    }
  } catch (error) {
    console.error('Failed to parse task XML:', error)
    return null
  }
}

// Parse hierarchical task tree output (<root> with nested <subtasks><task>...)
export function parseTaskTree(xml: string): PlannedTaskTree | null {
  try {
    const parser = new DOMParser()
    const xmlToParse = xml.trim().startsWith('<root') ? xml : `<root>${xml}</root>`
    const doc = parser.parseFromString(xmlToParse, 'text/xml')
    const root = doc.documentElement
    if (root.tagName !== 'root') return null

    const parseNodes = (container: Element): { text: string }[] => {
      const nodesEl = container.querySelector(':scope > nodes')
      if (!nodesEl) return []
      const nodeEls = nodesEl.querySelectorAll(':scope > node')
      return Array.from(nodeEls)
        .map(el => ({ text: (el.textContent || '').trim() }))
        .filter(n => n.text)
    }

    const parsePlannedTask = (taskEl: Element): PlannedTask => {
      const name = taskEl.querySelector(':scope > name')?.textContent?.trim()
      const desc = taskEl.querySelector(':scope > task')?.textContent?.trim()
      const nodes = parseNodes(taskEl)
      const subtasks: PlannedTask[] = []
      const subsEl = taskEl.querySelector(':scope > subtasks')
      if (subsEl) {
        const taskChildren = subsEl.querySelectorAll(':scope > task')
        taskChildren.forEach(child => {
          subtasks.push(parsePlannedTask(child))
        })
      }
      return {
        name,
        description: desc,
        nodes,
        subtasks: subtasks.length > 0 ? subtasks : undefined,
      }
    }

    // root level
    const name = root.querySelector(':scope > name')?.textContent?.trim()
    const thought = root.querySelector(':scope > thought')?.textContent?.trim()
    const desc = root.querySelector(':scope > task')?.textContent?.trim()
    const nodes = parseNodes(root)
    const subtasks: PlannedTask[] = []
    const subsEl = root.querySelector(':scope > subtasks')
    if (subsEl) {
      subsEl.querySelectorAll(':scope > task').forEach(t => subtasks.push(parsePlannedTask(t)))
    }

    const tree: PlannedTaskTree = {
      name,
      thought,
      description: desc,
      nodes,
      subtasks: subtasks.length > 0 ? subtasks : undefined,
    }
    return tree
  } catch (e) {
    console.error('Failed to parse task tree XML:', e)
    return null
  }
}

// Build minimal <agent> XML string from planned task data
export function buildAgentXmlFromPlanned(planned: PlannedTask): string {
  try {
    const doc = new DOMParser().parseFromString('<agent></agent>', 'text/xml')
    const agent = doc.documentElement
    const t = doc.createElement('task')
    t.textContent = (planned.description || planned.name || '').trim()
    agent.appendChild(t)
    if (planned.nodes && planned.nodes.length > 0) {
      const nodesEl = doc.createElement('nodes')
      for (const n of planned.nodes) {
        const nodeEl = doc.createElement('node')
        nodeEl.textContent = (n.text || '').trim()
        nodesEl.appendChild(nodeEl)
      }
      agent.appendChild(nodesEl)
    }
    return agent.outerHTML || '<agent />'
  } catch {
    return '<agent />'
  }
}

export function resetTaskXml(task: Task) {
  task.modified = false
}

export function buildAgentRootXml(
  agentXml: string,
  taskPrompt: string,
  nodeCallback?: (nodeId: string, node: Element) => void
): string {
  try {
    const parser = new DOMParser()
    const doc = parser.parseFromString(`<root>${agentXml}</root>`, 'text/xml')
    const root = doc.documentElement

    // Insert user's instruction to ensure LLM gets the actual task context
    if (taskPrompt && taskPrompt.trim().length > 0) {
      const instructionEl = doc.createElement('instruction')
      // Use textContent to avoid breaking XML with special chars
      instructionEl.textContent = taskPrompt
      // Prepend instruction before nodes for visibility
      if (root.firstChild) {
        root.insertBefore(instructionEl, root.firstChild)
      } else {
        root.appendChild(instructionEl)
      }
    }

    // Apply node callback if provided
    if (nodeCallback) {
      const nodes = root.querySelectorAll('node')
      for (let i = 0; i < nodes.length; i++) {
        const node = nodes[i] as Element
        nodeCallback(`node-${i}`, node)
      }
    }

    return root.outerHTML || root.toString()
  } catch (error) {
    console.error('Failed to build agent root XML:', error)
    return agentXml
  }
}

export function getOuterXML(node: Element): string {
  return node.outerHTML || node.toString()
}

export function extractAgentXmlNode(agentXml: string, nodeSelector: string): Element | null {
  try {
    const parser = new DOMParser()
    const doc = parser.parseFromString(`<root>${agentXml}</root>`, 'text/xml')
    const node = doc.querySelector(nodeSelector)
    return node as Element | null
  } catch (error) {
    console.error('Failed to extract agent XML node:', error)
    return null
  }
}
