import { invoke } from '@/utils/request'

export interface CodeDefItem {
  file: string
  kind: 'function' | 'class' | 'interface' | 'type' | 'enum' | 'default' | 'var-function'
  name: string
  line: number
  exported?: boolean
  isDefault?: boolean
}

export class CodeApi {
  listDefinitionNames = async (params: { path: string }): Promise<CodeDefItem[]> => {
    return await invoke<CodeDefItem[]>('code_list_definition_names', { path: params.path })
  }
}

export const codeApi = new CodeApi()
export type { CodeDefItem as CodeDefinition }
