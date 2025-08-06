/**
 * 终端工具套件 - 专为Agent框架设计的终端操作工具
 *
 * 提供完整的终端操作能力，包括命令执行、会话管理、环境监控等
 */

import type { ToolDefinition, ExecutionContext, ToolResult, FunctionCallSchema } from './HybridToolManager'
import { useTerminalStore } from '@/stores/Terminal'
import { terminal } from '@/api/terminal'
import { v4 as uuidv4 } from 'uuid'

/**
 * 创建终端命令执行工具
 */
export const createTerminalExecuteTool = (): ToolDefinition => {
  return {
    id: 'terminal_execute',
    name: 'terminal_execute',
    description: '在终端中执行命令并返回结果，支持实时输出捕获和错误处理',
    category: 'terminal',
    type: 'hybrid',
    parameters: [
      {
        name: 'command',
        type: 'string',
        description: '要执行的终端命令',
        required: true,
      },
      {
        name: 'workingDirectory',
        type: 'string',
        description: '执行命令的工作目录',
        required: false,
      },
      {
        name: 'timeout',
        type: 'number',
        description: '命令执行超时时间（秒）',
        required: false,
        default: 30,
      },
      {
        name: 'captureStreaming',
        type: 'boolean',
        description: '是否捕获流式输出',
        required: false,
        default: false,
      },
    ],
    functionCallSchema: createTerminalExecuteFunctionSchema(),
    builtinImplementation: executeTerminalCommandBuiltin,
    metadata: {
      riskLevel: 'medium',
      requiredCapabilities: ['terminal_access'],
      terminalSpecific: true,
    },
  }
}

/**
 * 创建终端会话管理工具
 */
export const createTerminalSessionTool = (): ToolDefinition => {
  return {
    id: 'terminal_session',
    name: 'terminal_session',
    description: '管理终端会话，包括创建、切换、关闭和列出会话',
    category: 'terminal',
    type: 'hybrid',
    parameters: [
      {
        name: 'action',
        type: 'string',
        description: '操作类型',
        required: true,
        enum: ['create', 'switch', 'close', 'list', 'info'],
      },
      {
        name: 'sessionId',
        type: 'string',
        description: '会话ID（用于switch, close, info操作）',
        required: false,
      },
      {
        name: 'workingDirectory',
        type: 'string',
        description: '新会话的工作目录（用于create操作）',
        required: false,
      },
      {
        name: 'sessionName',
        type: 'string',
        description: '会话名称（用于create操作）',
        required: false,
      },
    ],
    functionCallSchema: createTerminalSessionFunctionSchema(),
    builtinImplementation: executeTerminalSessionBuiltin,
    metadata: {
      riskLevel: 'low',
      requiredCapabilities: ['terminal_session_management'],
      terminalSpecific: true,
    },
  }
}

/**
 * 创建终端环境监控工具
 */
export const createTerminalMonitorTool = (): ToolDefinition => {
  return {
    id: 'terminal_monitor',
    name: 'terminal_monitor',
    description: '监控终端环境状态，包括进程、资源使用、环境变量等',
    category: 'terminal',
    type: 'hybrid',
    parameters: [
      {
        name: 'monitorType',
        type: 'string',
        description: '监控类型',
        required: true,
        enum: ['processes', 'environment', 'resources', 'history', 'status'],
      },
      {
        name: 'detailed',
        type: 'boolean',
        description: '是否返回详细信息',
        required: false,
        default: false,
      },
      {
        name: 'filterPattern',
        type: 'string',
        description: '过滤模式（正则表达式）',
        required: false,
      },
    ],
    functionCallSchema: createTerminalMonitorFunctionSchema(),
    builtinImplementation: executeTerminalMonitorBuiltin,
    metadata: {
      riskLevel: 'low',
      requiredCapabilities: ['terminal_monitoring'],
      terminalSpecific: true,
    },
  }
}

/**
 * 创建终端文件操作工具
 */
export const createTerminalFileOpsTool = (): ToolDefinition => {
  return {
    id: 'terminal_file_ops',
    name: 'terminal_file_ops',
    description: '通过终端执行文件操作，包括读取、写入、移动、删除等',
    category: 'terminal',
    type: 'hybrid',
    parameters: [
      {
        name: 'operation',
        type: 'string',
        description: '文件操作类型',
        required: true,
        enum: ['read', 'write', 'move', 'copy', 'delete', 'list', 'find', 'permissions'],
      },
      {
        name: 'path',
        type: 'string',
        description: '文件或目录路径',
        required: true,
      },
      {
        name: 'content',
        type: 'string',
        description: '文件内容（用于write操作）',
        required: false,
      },
      {
        name: 'destination',
        type: 'string',
        description: '目标路径（用于move, copy操作）',
        required: false,
      },
      {
        name: 'options',
        type: 'object',
        description: '额外选项',
        required: false,
        properties: {
          recursive: {
            name: 'recursive',
            type: 'boolean',
            description: '递归操作',
          },
          force: {
            name: 'force',
            type: 'boolean',
            description: '强制操作',
          },
          permissions: {
            name: 'permissions',
            type: 'string',
            description: '文件权限（如755）',
          },
        },
      },
    ],
    functionCallSchema: createTerminalFileOpsFunctionSchema(),
    builtinImplementation: executeTerminalFileOpsBuiltin,
    metadata: {
      riskLevel: 'high',
      requiredCapabilities: ['terminal_file_access'],
      terminalSpecific: true,
    },
  }
}

// ========== Function Call Schemas ==========

const createTerminalExecuteFunctionSchema = (): FunctionCallSchema => {
  return {
    type: 'function',
    function: {
      name: 'terminal_execute',
      description: '在终端中执行命令',
      parameters: {
        type: 'object',
        properties: {
          command: {
            type: 'string',
            description: '要执行的终端命令',
          },
          workingDirectory: {
            type: 'string',
            description: '执行命令的工作目录',
          },
          timeout: {
            type: 'number',
            description: '超时时间（秒）',
            default: 30,
          },
          captureStreaming: {
            type: 'boolean',
            description: '是否捕获流式输出',
            default: false,
          },
        },
        required: ['command'],
      },
    },
  }
}

const createTerminalSessionFunctionSchema = (): FunctionCallSchema => {
  return {
    type: 'function',
    function: {
      name: 'terminal_session',
      description: '管理终端会话',
      parameters: {
        type: 'object',
        properties: {
          action: {
            type: 'string',
            enum: ['create', 'switch', 'close', 'list', 'info'],
            description: '要执行的操作',
          },
          sessionId: {
            type: 'string',
            description: '会话ID',
          },
          workingDirectory: {
            type: 'string',
            description: '工作目录',
          },
          sessionName: {
            type: 'string',
            description: '会话名称',
          },
        },
        required: ['action'],
      },
    },
  }
}

const createTerminalMonitorFunctionSchema = (): FunctionCallSchema => {
  return {
    type: 'function',
    function: {
      name: 'terminal_monitor',
      description: '监控终端环境',
      parameters: {
        type: 'object',
        properties: {
          monitorType: {
            type: 'string',
            enum: ['processes', 'environment', 'resources', 'history', 'status'],
            description: '监控类型',
          },
          detailed: {
            type: 'boolean',
            description: '是否返回详细信息',
            default: false,
          },
          filterPattern: {
            type: 'string',
            description: '过滤模式',
          },
        },
        required: ['monitorType'],
      },
    },
  }
}

const createTerminalFileOpsFunctionSchema = (): FunctionCallSchema => {
  return {
    type: 'function',
    function: {
      name: 'terminal_file_ops',
      description: '执行文件操作',
      parameters: {
        type: 'object',
        properties: {
          operation: {
            type: 'string',
            enum: ['read', 'write', 'move', 'copy', 'delete', 'list', 'find', 'permissions'],
            description: '操作类型',
          },
          path: {
            type: 'string',
            description: '文件路径',
          },
          content: {
            type: 'string',
            description: '文件内容',
          },
          destination: {
            type: 'string',
            description: '目标路径',
          },
          options: {
            type: 'object',
            description: '操作选项',
          },
        },
        required: ['operation', 'path'],
      },
    },
  }
}

// ========== 内置实现 ==========

/**
 * 内置终端命令执行实现
 */
const executeTerminalCommandBuiltin = async (
  params: Record<string, unknown>,
  context: ExecutionContext
): Promise<ToolResult> {
  const { command, workingDirectory, timeout = 30, captureStreaming = false } = params
  const commandStr = String(command)
  const workingDirStr = workingDirectory ? String(workingDirectory) : undefined
  const timeoutNum = Number(timeout)
  const capturingBool = Boolean(captureStreaming)

  if (!isCommandSafe(commandStr)) {
    return {
      success: false,
      error: `命令被安全策略阻止: ${commandStr}`,
    }
  }

  try {
    const result = await executeCommand(commandStr, {
      workingDirectory: workingDirStr,
      timeout: timeoutNum * 1000,
      streaming: capturingBool,
    })

    return {
      success: true,
      data: {
        command: commandStr,
        exitCode: result.exitCode,
        stdout: result.stdout,
        stderr: result.stderr,
        executionTime: result.executionTime,
        workingDirectory: result.workingDirectory,
      },
    }
  } catch (error) {
    return {
      success: false,
      error: `命令执行失败: ${error instanceof Error ? error.message : String(error)}`,
    }
  }
}

/**
 * 内置终端会话管理实现
 */
const executeTerminalSessionBuiltin = async (
  params: Record<string, unknown>,
  context: ExecutionContext
): Promise<ToolResult> {
  const { action, sessionId, workingDirectory, sessionName } = params
  const actionStr = String(action)
  const sessionIdStr = sessionId ? String(sessionId) : undefined
  const workingDirStr = workingDirectory ? String(workingDirectory) : undefined
  const sessionNameStr = sessionName ? String(sessionName) : undefined
  const terminalStore = useTerminalStore()

  try {
    switch (actionStr) {
      case 'create': {
        const newSessionId = await terminalStore.createTerminal(workingDirStr)
        const newTerminal = terminalStore.terminals.find(t => t.id === newSessionId)

        if (sessionNameStr && newTerminal) {
          newTerminal.title = sessionNameStr
        }

        return {
          success: true,
          data: {
            action: 'create',
            sessionId: newSessionId,
            sessionName: sessionNameStr || `Terminal ${newSessionId}`,
            workingDirectory: workingDirStr || process.cwd(),
          },
        }
      }

      case 'switch': {
        if (!sessionIdStr) {
          return { success: false, error: 'sessionId required for switch action' }
        }

        const targetTerminal = terminalStore.terminals.find(t => t.id === sessionIdStr)
        if (!targetTerminal) {
          return { success: false, error: `Session ${sessionIdStr} not found` }
        }

        terminalStore.setActiveTerminal(sessionIdStr)
        return {
          success: true,
          data: {
            action: 'switch',
            sessionId: sessionIdStr,
            sessionName: targetTerminal.title || `Terminal ${sessionIdStr}`,
          },
        }
      }

      case 'close': {
        if (!sessionIdStr) {
          return { success: false, error: 'sessionId required for close action' }
        }

        await terminalStore.closeTerminal(sessionIdStr)
        return {
          success: true,
          data: { action: 'close', sessionId: sessionIdStr },
        }
      }

      case 'list': {
        const sessions = terminalStore.terminals.map(t => ({
          id: t.id,
          name: t.title || `Terminal ${t.id}`,
          active: t.id === terminalStore.activeTerminalId,
          backendId: t.backendId,
        }))

        return {
          success: true,
          data: {
            action: 'list',
            sessions,
            activeSessionId: terminalStore.activeTerminalId,
            totalSessions: sessions.length,
          },
        }
      }

      case 'info': {
        if (!sessionIdStr) {
          return { success: false, error: 'sessionId required for info action' }
        }

        const infoTerminal = terminalStore.terminals.find(t => t.id === sessionIdStr)
        if (!infoTerminal) {
          return { success: false, error: `Session ${sessionIdStr} not found` }
        }

        return {
          success: true,
          data: {
            action: 'info',
            sessionId: sessionIdStr,
            sessionName: infoTerminal.title || `Terminal ${sessionIdStr}`,
            active: infoTerminal.id === terminalStore.activeTerminalId,
            backendId: infoTerminal.backendId,
          },
        }
      }

      default:
        return { success: false, error: `Unknown action: ${actionStr}` }
    }
  } catch (error) {
    return {
      success: false,
      error: `Session management failed: ${error instanceof Error ? error.message : String(error)}`,
    }
  }
}

/**
 * 内置终端监控实现
 */
const executeTerminalMonitorBuiltin = async (
  params: Record<string, unknown>,
  context: ExecutionContext
): Promise<ToolResult> {
  const { monitorType, detailed = false, filterPattern } = params
  const monitorTypeStr = String(monitorType)
  const detailedBool = Boolean(detailed)
  const filterPatternStr = filterPattern ? String(filterPattern) : undefined

  try {
    switch (monitorTypeStr) {
      case 'processes': {
        const processes = await getProcessList(detailedBool, filterPatternStr)
        return { success: true, data: { monitorType: monitorTypeStr, processes } }
      }

      case 'environment': {
        const environment = await getEnvironmentInfo(detailedBool)
        return { success: true, data: { monitorType: monitorTypeStr, environment } }
      }

      case 'resources': {
        const resources = await getResourceInfo(detailedBool)
        return { success: true, data: { monitorType: monitorTypeStr, resources } }
      }

      case 'history': {
        const history = await getCommandHistory(detailedBool, filterPatternStr)
        return { success: true, data: { monitorType: monitorTypeStr, history } }
      }

      case 'status': {
        const status = await getTerminalStatus()
        return { success: true, data: { monitorType: monitorTypeStr, status } }
      }

      default:
        return { success: false, error: `Unknown monitor type: ${monitorTypeStr}` }
    }
  } catch (error) {
    return {
      success: false,
      error: `Monitoring failed: ${error instanceof Error ? error.message : String(error)}`,
    }
  }
}

/**
 * 内置文件操作实现
 */
const executeTerminalFileOpsBuiltin = async (
  params: Record<string, unknown>,
  context: ExecutionContext
): Promise<ToolResult> {
  const { operation, path: filePath, content, destination, options = {} } = params
  const pathStr = String(filePath)
  const optionsObj = options as Record<string, unknown>

  if (!isSafePath(pathStr)) {
    return { success: false, error: `Path not allowed by security policy: ${pathStr}` }
  }

  try {
    switch (operation) {
      case 'read': {
        const fileContent = await executeCommand(`cat "${pathStr}"`)
        return {
          success: true,
          data: {
            operation,
            path: pathStr,
            content: fileContent.stdout,
            size: fileContent.stdout.length,
          },
        }
      }

      case 'write': {
        if (!content) {
          return { success: false, error: 'Content required for write operation' }
        }

        const writeCmd = `echo ${JSON.stringify(content)} > "${pathStr}"`
        const writeResult = await executeCommand(writeCmd)
        return {
          success: writeResult.exitCode === 0,
          data: { operation, path: pathStr, written: writeResult.exitCode === 0 },
          error: writeResult.exitCode !== 0 ? writeResult.stderr : undefined,
        }
      }

      case 'move': {
        if (!destination) {
          return { success: false, error: 'Destination required for move operation' }
        }

        const moveCmd = `mv "${pathStr}" "${destination}"`
        const moveResult = await executeCommand(moveCmd)
        return {
          success: moveResult.exitCode === 0,
          data: { operation, from: pathStr, to: destination },
          error: moveResult.exitCode !== 0 ? moveResult.stderr : undefined,
        }
      }

      case 'copy': {
        if (!destination) {
          return { success: false, error: 'Destination required for copy operation' }
        }

        const copyFlags = optionsObj.recursive ? '-r' : ''
        const copyCmd = `cp ${copyFlags} "${pathStr}" "${destination}"`
        const copyResult = await executeCommand(copyCmd)
        return {
          success: copyResult.exitCode === 0,
          data: { operation, from: pathStr, to: destination, recursive: optionsObj.recursive },
          error: copyResult.exitCode !== 0 ? copyResult.stderr : undefined,
        }
      }

      case 'delete': {
        const deleteFlags = optionsObj.recursive ? '-rf' : '-f'
        const deleteCmd = `rm ${deleteFlags} "${pathStr}"`
        const deleteResult = await executeCommand(deleteCmd)
        return {
          success: deleteResult.exitCode === 0,
          data: { operation, path: pathStr, recursive: optionsObj.recursive },
          error: deleteResult.exitCode !== 0 ? deleteResult.stderr : undefined,
        }
      }

      case 'list': {
        const { detailed } = params
        const listFlags = detailed ? '-la' : '-l'
        const listCmd = `ls ${listFlags} "${pathStr}"`
        const listResult = await executeCommand(listCmd)
        return {
          success: listResult.exitCode === 0,
          data: {
            operation,
            path: pathStr,
            listing: listResult.stdout.split('\n').filter(line => line.trim()),
          },
          error: listResult.exitCode !== 0 ? listResult.stderr : undefined,
        }
      }

      case 'find': {
        const { filterPattern } = params
        const findPatternStr = filterPattern || '*'
        const findCmd = `find "${pathStr}" -name "${findPatternStr}"`
        const findResult = await executeCommand(findCmd)
        return {
          success: findResult.exitCode === 0,
          data: {
            operation,
            path: pathStr,
            pattern: findPatternStr,
            results: findResult.stdout.split('\n').filter(line => line.trim()),
          },
          error: findResult.exitCode !== 0 ? findResult.stderr : undefined,
        }
      }

      case 'permissions': {
        if (optionsObj.permissions) {
          const chmodCmd = `chmod ${optionsObj.permissions} "${pathStr}"`
          const chmodResult = await executeCommand(chmodCmd)
          return {
            success: chmodResult.exitCode === 0,
            data: { operation, path: pathStr, permissions: optionsObj.permissions },
            error: chmodResult.exitCode !== 0 ? chmodResult.stderr : undefined,
          }
        } else {
          const statCmd = `stat -c "%a %n" "${pathStr}"`
          const statResult = await executeCommand(statCmd)
          return {
            success: statResult.exitCode === 0,
            data: { operation, path: pathStr, permissions: statResult.stdout.trim() },
            error: statResult.exitCode !== 0 ? statResult.stderr : undefined,
          }
        }
      }

      default:
        return { success: false, error: `Unknown operation: ${operation}` }
    }
  } catch (error) {
    return {
      success: false,
      error: `File operation failed: ${error instanceof Error ? error.message : String(error)}`,
    }
  }
}

// ========== 辅助函数 ==========

const executeCommand = async (
  command: string,
  options: {
    workingDirectory?: string
    timeout?: number
    streaming?: boolean
  } = {}
): Promise<{
  exitCode: number
  stdout: string
  stderr: string
  executionTime: number
  workingDirectory?: string
}> {
  const terminalStore = useTerminalStore()
  let activeTerminal = terminalStore.activeTerminal

  if (!activeTerminal?.backendId) {
    const terminalId = await terminalStore.createTerminal(options.workingDirectory)
    activeTerminal = terminalStore.terminals.find(t => t.id === terminalId)
    if (!activeTerminal?.backendId) {
      throw new Error('Failed to create terminal instance')
    }
    await new Promise(resolve => setTimeout(resolve, 500))
  }

  const startTime = Date.now()
  const completionMarker = `CMD_EXEC_COMPLETE_${uuidv4()}`
  const fullCommand = `${command}; echo "${completionMarker}"`

  let outputBuffer = ''
  let errorBuffer = ''
  let commandCompleted = false

  const callbacks = {
    onOutput: (data: string) => {
      if (data.includes(completionMarker)) {
        commandCompleted = true
        outputBuffer += data.replace(completionMarker, '')
      } else {
        outputBuffer += data
      }
    },
    onExit: () => {
      commandCompleted = true
    },
  }

  try {
    terminalStore.registerTerminalCallbacks(activeTerminal.id, callbacks)

    await terminal.write({
      paneId: activeTerminal.backendId,
      data: fullCommand + '\n',
    })

    const timeout = options.timeout || 30000
    const waitStartTime = Date.now()

    while (!commandCompleted && Date.now() - waitStartTime < timeout) {
      await new Promise(resolve => setTimeout(resolve, 100))
    }

    if (!commandCompleted) {
      errorBuffer = 'Command execution timeout'
    }

    // 清理输出
    const commandEchoIndex = outputBuffer.indexOf(fullCommand.trim())
    let cleanOutput = outputBuffer
    if (commandEchoIndex !== -1) {
      cleanOutput = outputBuffer.substring(commandEchoIndex + fullCommand.trim().length).trim()
    }

    return {
      exitCode: errorBuffer ? 1 : 0,
      stdout: cleanOutput,
      stderr: errorBuffer,
      executionTime: Date.now() - startTime,
      workingDirectory: options.workingDirectory,
    }
  } finally {
    terminalStore.unregisterTerminalCallbacks(activeTerminal.id, callbacks)
  }
}

const isCommandSafe = (command: string): boolean => {
  const dangerousPatterns = [
    /rm\s+-rf\s+\/$/,
    /sudo\s+rm/,
    />\s*\/dev\/sd[a-z]/,
    /mkfs/,
    /dd\s+if=/,
    /:\(\)\{.*\};\s*:/,
  ]

  return !dangerousPatterns.some(pattern => pattern.test(command))
}

const isSafePath = (path: string): boolean => {
  const forbiddenPaths = ['/etc/shadow', '/etc/passwd', '/boot', '/sys', '/proc']

  return !forbiddenPaths.some(forbidden => path.startsWith(forbidden))
}

const getProcessList = async (detailed: boolean, filterPattern?: string): Promise<unknown> => {
  const command = detailed ? 'ps aux' : 'ps -e'
  const result = await executeCommand(command)

  let processes = result.stdout.split('\n').filter(line => line.trim())

  if (filterPattern) {
    const regex = new RegExp(filterPattern, 'i')
    processes = processes.filter(line => regex.test(line))
  }

  return processes
}

const getEnvironmentInfo = async (detailed: boolean): Promise<unknown> => {
  const commands = detailed
    ? ['env', 'echo $PATH', 'echo $HOME', 'echo $USER', 'uname -a']
    : ['echo $PATH', 'echo $USER']

  const results = await Promise.all(commands.map(cmd => executeCommand(cmd)))

  return {
    environment: results[0]?.stdout.split('\n').filter(line => line.trim()),
    path: results[1]?.stdout.trim(),
    home: results[2]?.stdout.trim(),
    user: results[3]?.stdout.trim(),
    system: results[4]?.stdout.trim(),
  }
}

const getResourceInfo = async (detailed: boolean): Promise<unknown> => {
  const commands = detailed ? ['free -h', 'df -h', 'uptime', 'top -bn1 | head -5'] : ['free -h', 'uptime']

  const results = await Promise.all(commands.map(cmd => executeCommand(cmd)))

  return {
    memory: results[0]?.stdout,
    disk: results[1]?.stdout,
    uptime: results[2]?.stdout,
    processes: results[3]?.stdout,
  }
}

const getCommandHistory = async (detailed: boolean, filterPattern?: string): Promise<unknown> => {
  const command = detailed ? 'history' : 'history | tail -20'
  const result = await executeCommand(command)

  let history = result.stdout.split('\n').filter(line => line.trim())

  if (filterPattern) {
    const regex = new RegExp(filterPattern, 'i')
    history = history.filter(line => regex.test(line))
  }

  return history
}

const getTerminalStatus = async (): Promise<unknown> => {
  const terminalStore = useTerminalStore()

  return {
    activeTerminalId: terminalStore.activeTerminalId,
    totalTerminals: terminalStore.terminals.length,
    terminals: terminalStore.terminals.map(t => ({
      id: t.id,
      name: t.title || `Terminal ${t.id}`,
      active: t.id === terminalStore.activeTerminalId,
    })),
  }
}

/**
 * 导出所有终端工具
 */
export const getAllTerminalTools = (): ToolDefinition[] => {
  return [
    createTerminalExecuteTool(),
    createTerminalSessionTool(),
    createTerminalMonitorTool(),
    createTerminalFileOpsTool(),
  ]
}
