import { Agent } from './base'
import { AgentContext } from '../core/context'
import { RetryLanguageModel } from '../llm'
import { LanguageModelV2Prompt } from '../types'
import config from '../config'

export class ContextCompressorAgent extends Agent {
  constructor() {
    super({
      name: 'ContextCompressor',
      description: 'Intelligent context compression agent for task execution',
      tools: [],
    })
  }

  /**
   * Intelligently compress context information
   * @param agentContext Current agent context
   * @param fullContext Full context information
   * @param targetLength Target compression length
   * @returns Compressed context
   */
  public async compressContext(
    agentContext: AgentContext,
    fullContext: string,
    targetLength: number = config.maxAgentContextLength
  ): Promise<string> {
    // If context length is within limit, return directly
    if (fullContext.length <= targetLength) {
      return fullContext
    }

    const rlm = new RetryLanguageModel(agentContext.context.config.llms, agentContext.context.config.planLlms)

    const compressionRatio = targetLength / fullContext.length
    const systemPrompt = this.buildCompressionSystemPrompt(compressionRatio)

    const messages: LanguageModelV2Prompt = [
      {
        role: 'system',
        content: systemPrompt,
      },
      {
        role: 'user',
        content: [
          {
            type: 'text',
            text: `Please compress the following context information while preserving all key information and important details:\n\n${fullContext}`,
          },
        ],
      },
    ]

    try {
      const result = await rlm.call({
        messages,
        maxTokens: Math.floor(targetLength / 4), // 估算token数量
        temperature: 0.1, // 低温度确保一致性
        abortSignal: agentContext.context.controller.signal,
      })

      const compressedText = result.text || fullContext

      // If still too long after compression, perform recursive compression
      if (compressedText.length > targetLength) {
        return await this.compressContext(
          agentContext,
          compressedText,
          Math.floor(targetLength * 0.8) // More aggressive compression
        )
      }

      return compressedText
    } catch (error) {
      console.warn('Context compression failed, using truncation fallback:', error)
      // Fallback strategy when compression fails: intelligent truncation
      return this.intelligentTruncate(fullContext, targetLength)
    }
  }

  private buildCompressionSystemPrompt(compressionRatio: number): string {
    return `You are a professional context compression expert. Your task is to compress long text to approximately ${Math.floor(compressionRatio * 100)}% of its original length while preserving all key information.

Compression principles:
1. Preserve all important facts, data, and conclusions
2. Retain key reasoning processes and analysis steps
3. Keep important code snippets, configuration information, and file paths
4. Remove redundant descriptions and duplicate information
5. Merge similar content
6. Use more concise expressions

Output requirements:
- Maintain the logical structure of the original text
- Use clear paragraph separation
- Express important information in concise and clear language
- Ensure subsequent agents can make correct decisions based on compressed content

Please output the compressed content directly without adding any explanations or meta-information.`
  }

  /**
   * Intelligent truncation: preserve important information from beginning and end
   */
  private intelligentTruncate(text: string, maxLength: number): string {
    if (text.length <= maxLength) {
      return text
    }

    const headLength = Math.floor(maxLength * 0.35) // 35% for beginning
    const tailLength = Math.floor(maxLength * 0.3) // 30% for end

    const head = text.substring(0, headLength)
    const tail = text.substring(text.length - tailLength)

    return `${head}\n\n...[truncated ${text.length - headLength - tailLength} characters]...\n\n${tail}`
  }

  /**
   * Batch compress results from multiple agents
   */
  public async compressMultipleResults(
    agentContext: AgentContext,
    agentResults: Array<{ name: string; task: string; result: string }>,
    targetLength: number = config.maxAgentContextLength
  ): Promise<string> {
    const fullContext = agentResults.map(({ name, task, result }) => `## ${task || name}\n${result}`).join('\n\n')

    if (fullContext.length <= targetLength) {
      return fullContext
    }

    // If total length exceeds limit, use specialized multi-result compression strategy
    return await this.compressMultipleResultsWithStrategy(agentContext, agentResults, targetLength)
  }

  private async compressMultipleResultsWithStrategy(
    agentContext: AgentContext,
    agentResults: Array<{ name: string; task: string; result: string }>,
    targetLength: number
  ): Promise<string> {
    const rlm = new RetryLanguageModel(agentContext.context.config.llms, agentContext.context.config.planLlms)

    const systemPrompt = `You are a professional task result integration expert. You need to integrate and compress multiple task execution results into a coherent context.

Integration principles:
1. Preserve core contributions and key findings from each agent
2. Merge related information and avoid duplication
3. Maintain logical coherence and chronological order
4. Highlight important data, configurations, and code snippets
5. Provide sufficient decision-making basis for subsequent agents

Output format:
Use clear paragraph structure, describing each important finding in concise language.`

    const userContent = agentResults
      .map(({ name, task, result }, index) => `### Task ${index + 1}: ${task || name}\n${result}`)
      .join('\n\n')

    const messages: LanguageModelV2Prompt = [
      {
        role: 'system',
        content: systemPrompt,
      },
      {
        role: 'user',
        content: [
          {
            type: 'text',
            text: `Please integrate the following execution results from multiple tasks:\n\n${userContent}`,
          },
        ],
      },
    ]

    try {
      const result = await rlm.call({
        messages,
        maxTokens: Math.floor(targetLength / 4),
        temperature: 0.1,
        abortSignal: agentContext.context.controller.signal,
      })

      return result.text || this.intelligentTruncate(userContent, targetLength)
    } catch (error) {
      console.warn('Multi-result compression failed, using fallback:', error)
      return this.intelligentTruncate(userContent, targetLength)
    }
  }
}
