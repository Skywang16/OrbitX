# Terminal Store Context Refactor - Implementation Summary

## Changes Made

### 1. Updated `setActiveTerminal` Method

**Before:**

```typescript
const setActiveTerminal = (id: string) => {
  // Only updated frontend state
  activeTerminalId.value = id
  sessionStore.setActiveTabId(id)
  immediateSync()
}
```

**After:**

```typescript
const setActiveTerminal = async (id: string) => {
  // Update frontend state
  activeTerminalId.value = id

  // Sync active terminal state to backend
  if (targetTerminal.backendId !== null) {
    try {
      await terminalContextApi.setActivePaneId(targetTerminal.backendId)
    } catch (error) {
      console.warn(`同步活跃终端到后端失败: ${error}`)
      // Continue execution, don't block frontend state update
    }
  }

  // Sync to session state
  sessionStore.setActiveTabId(id)
  immediateSync()
}
```

### 2. Updated All Calls to `setActiveTerminal`

All calls to `setActiveTerminal` have been updated to handle the async nature:

- `createTerminal()` - now awaits `setActiveTerminal`
- `cleanupTerminalState()` - now async and awaits `setActiveTerminal`
- `closeTerminal()` - now awaits `cleanupTerminalState`
- `createAgentTerminal()` - now awaits `setActiveTerminal`
- `createTerminalWithShell()` - now awaits `setActiveTerminal`
- `restoreFromSessionState()` - now awaits `setActiveTerminal`

### 3. Removed CWD Write-back Logic

**CWD Event Listener (Updated Comment):**

```typescript
const unlistenCwdChanged = await listen<{
  paneId: number
  cwd: string
}>('pane_cwd_changed', event => {
  try {
    const terminal = findTerminalByBackendId(event.payload.paneId)
    if (terminal) {
      // 只订阅后端CWD变化事件，不进行回写
      terminal.cwd = event.payload.cwd
      updateTerminalTitle(terminal, event.payload.cwd)
    }
  } catch (error) {
    console.error('Error handling terminal CWD change event:', error)
  }
})
```

**Updated `updateTerminalCwd` Method:**

```typescript
const updateTerminalCwd = (id: string, cwd: string) => {
  // ... existing validation ...

  // 仅更新前端UI状态，不回写到后端
  // 后端是CWD的单一数据源，前端只订阅变化事件
  terminal.cwd = cwd

  // 智能更新终端标题
  updateTerminalTitle(terminal, cwd)
  immediateSync()
}
```

### 4. Added Import for Terminal Context API

```typescript
import { shellApi, terminalApi, terminalContextApi } from '@/api'
```

## Key Behavioral Changes

### 1. Backend Synchronization

- **Before**: Frontend managed active terminal state independently
- **After**: Frontend syncs active terminal changes to backend via `terminalContextApi.setActivePaneId()`

### 2. Error Handling

- Backend sync errors are logged as warnings but don't block frontend operation
- Graceful degradation ensures UI remains responsive even if backend is unavailable

### 3. CWD Management

- **Before**: Frontend could write CWD changes back to backend
- **After**: Frontend only subscribes to backend CWD change events
- Backend is now the single source of truth for CWD information

### 4. Async Operations

- `setActiveTerminal` is now async to handle backend synchronization
- All callers have been updated to properly await the operation

## Requirements Satisfied

✅ **Requirement 1.1**: Backend active terminal management

- `setActiveTerminal` now calls `terminalContextApi.setActivePaneId()`

✅ **Requirement 3.2**: CWD single data source

- Removed frontend CWD write-back logic
- Frontend only subscribes to backend events

✅ **Requirement 3.3**: Pure subscription mode

- Frontend only listens to `pane_cwd_changed` events
- No more frontend-initiated CWD updates to backend

## Testing

### Manual Testing Steps

1. **Terminal Creation**: Create new terminal → verify backend `set_active_pane` call
2. **Terminal Switching**: Switch between terminals → verify each switch calls backend
3. **CWD Updates**: Navigate directories → verify only subscription, no write-back
4. **Error Handling**: Simulate backend errors → verify graceful degradation

### Test Files Created

- `src/stores/__tests__/Terminal.test.ts` - Unit tests with mocks
- `src/stores/__tests__/terminal-integration.test.ts` - Integration test scenarios
- Browser console helpers available via `terminalTestHelpers`

## Migration Notes

### Breaking Changes

- `setActiveTerminal` is now async - all callers must await it
- CWD updates no longer trigger backend writes

### Backward Compatibility

- All existing functionality preserved
- Error handling ensures graceful degradation
- Frontend state management remains consistent

## Next Steps

This implementation satisfies the requirements for task 10. The next tasks in the implementation plan are:

- **Task 11**: Update AI Chat Store to use new context API
- **Task 12**: Remove frontend Shell Integration CWD write-back logic

The Terminal Store is now properly integrated with the backend Terminal Context Service and follows the single-source-of-truth pattern for terminal state management.
