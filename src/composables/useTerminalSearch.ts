import { reactive, ref } from 'vue'
import type { SearchAddon } from '@xterm/addon-search'
import type { Terminal } from '@xterm/xterm'

export interface SearchOptions {
  caseSensitive: boolean
  wholeWord: boolean
}

export interface SearchResult {
  line: number
  startCol: number
  endCol: number
}

export interface SearchState {
  visible: boolean
  query: string
  options: SearchOptions
  results: SearchResult[]
  currentIndex: number
}

export const useTerminalSearch = () => {
  const searchState = reactive<SearchState>({
    visible: false,
    query: '',
    options: { caseSensitive: false, wholeWord: false },
    results: [],
    currentIndex: -1,
  })

  const searchBoxRef = ref<{ setSearchState: (total: number, current: number) => void } | null>(null)

  const closeSearch = (searchAddon: SearchAddon | null) => {
    searchState.visible = false
    searchState.query = ''
    searchState.results = []
    searchState.currentIndex = -1
    if (searchAddon) {
      searchAddon.clearDecorations()
    }
  }

  const performSearch = (
    terminal: Terminal | null,
    searchAddon: SearchAddon | null,
    query: string,
    options: SearchOptions
  ) => {
    if (!terminal) return

    if (searchAddon) {
      searchAddon.clearDecorations()
    }

    const results: SearchResult[] = []
    const buffer = terminal.buffer.active

    for (let lineIndex = 0; lineIndex < buffer.length; lineIndex++) {
      const line = buffer.getLine(lineIndex)
      if (line) {
        const lineText = line.translateToString(false)
        let searchText = options.caseSensitive ? lineText : lineText.toLowerCase()
        let queryText = options.caseSensitive ? query : query.toLowerCase()

        if (options.wholeWord) {
          const regex = new RegExp(
            `\\b${queryText.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\b`,
            options.caseSensitive ? 'g' : 'gi'
          )
          let match
          while ((match = regex.exec(lineText)) !== null) {
            results.push({
              line: lineIndex,
              startCol: match.index,
              endCol: match.index + match[0].length,
            })
          }
        } else {
          let startIndex = 0
          while (true) {
            const foundIndex = searchText.indexOf(queryText, startIndex)
            if (foundIndex === -1) break

            results.push({
              line: lineIndex,
              startCol: foundIndex,
              endCol: foundIndex + query.length,
            })

            startIndex = foundIndex + 1
          }
        }
      }
    }

    searchState.results = results

    if (results.length > 0) {
      searchState.currentIndex = 0
      if (searchAddon) {
        searchAddon.findNext(query, {
          caseSensitive: options.caseSensitive,
          wholeWord: options.wholeWord,
          regex: false,
        })
      }
    } else {
      searchState.currentIndex = -1
    }

    searchBoxRef.value?.setSearchState(results.length, searchState.currentIndex)
  }

  const handleSearch = (
    terminal: Terminal | null,
    searchAddon: SearchAddon | null,
    query: string,
    options: SearchOptions
  ) => {
    if (!terminal || !query.trim()) {
      searchState.results = []
      searchState.currentIndex = -1
      searchBoxRef.value?.setSearchState(0, -1)
      if (searchAddon) {
        searchAddon.clearDecorations()
      }
      return
    }

    searchState.query = query
    searchState.options = options
    performSearch(terminal, searchAddon, query, options)
  }

  const findNext = (searchAddon: SearchAddon | null) => {
    if (!searchAddon || !searchState.query || searchState.results.length === 0) return

    searchState.currentIndex = (searchState.currentIndex + 1) % searchState.results.length

    searchAddon.findNext(searchState.query, {
      caseSensitive: searchState.options.caseSensitive,
      wholeWord: searchState.options.wholeWord,
      regex: false,
    })

    searchBoxRef.value?.setSearchState(searchState.results.length, searchState.currentIndex)
  }

  const findPrevious = (searchAddon: SearchAddon | null) => {
    if (!searchAddon || !searchState.query || searchState.results.length === 0) return

    searchState.currentIndex =
      searchState.currentIndex <= 0 ? searchState.results.length - 1 : searchState.currentIndex - 1

    searchAddon.findPrevious(searchState.query, {
      caseSensitive: searchState.options.caseSensitive,
      wholeWord: searchState.options.wholeWord,
      regex: false,
    })

    searchBoxRef.value?.setSearchState(searchState.results.length, searchState.currentIndex)
  }

  const handleOpenTerminalSearch = (isActive: boolean, searchAddon: SearchAddon | null) => {
    if (isActive) {
      searchState.visible = !searchState.visible

      if (!searchState.visible) {
        searchState.query = ''
        searchState.results = []
        searchState.currentIndex = -1
        if (searchAddon) {
          searchAddon.clearDecorations()
        }
      }
    }
  }

  return {
    searchState,
    searchBoxRef,
    closeSearch,
    handleSearch,
    findNext,
    findPrevious,
    handleOpenTerminalSearch,
  }
}
