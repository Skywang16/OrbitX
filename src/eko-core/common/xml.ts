// Use browser DOM APIs directly
import { Task, TaskNode, TaskTextNode } from '../types/core.types'

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

    const name = nameElement?.textContent?.trim() || 'Untitled Task'
    const thought = thoughtElement?.textContent?.trim() || ''

    let description = ''
    let nodes: TaskNode[] = []
    let status: 'init' | 'running' | 'done' | 'error' = 'init'
    let agentXml = ''

    if (agentElement) {
      const taskElement = agentElement.querySelector('task')
      description = taskElement?.textContent?.trim() || name

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
      // Fallback: treat entire content as description
      description = name
      agentXml = xml
    }

    return {
      taskId,
      name,
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
