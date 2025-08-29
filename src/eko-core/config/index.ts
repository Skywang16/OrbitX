type GlobalConfig = {
  name: string // product name
  platform: 'windows' | 'mac' | 'linux'
  maxReactNum: number
  maxTokens: number
  maxRetryNum: number

  compressThreshold: number // Dialogue context compression threshold (message count)
  largeTextLength: number
  fileTextMaxLength: number
  maxDialogueImgFileNum: number
  toolResultMultimodal: boolean
  expertMode: boolean
  expertModeTodoLoopNum: number
  maxAgentContextLength: number // Maximum context length for single agent
  enableIntelligentCompression: boolean // Enable intelligent context compression
}

const config: GlobalConfig = {
  name: 'OrbitX',
  platform: 'mac',
  maxReactNum: 500,
  maxTokens: 16000,
  maxRetryNum: 3,
  // agentParallel removed - single agent mode only
  compressThreshold: 80,
  largeTextLength: 5000,
  fileTextMaxLength: 20000,
  maxDialogueImgFileNum: 1,
  toolResultMultimodal: true,
  expertMode: false,
  expertModeTodoLoopNum: 10,
  maxAgentContextLength: 50000, // Maximum context length for single agent (50k characters)
  enableIntelligentCompression: true, // Enable intelligent compression
}

export default config
