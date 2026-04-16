You are about to enter orchestrate mode for a task that may benefit from coordinated parallel task execution.

{{input}}

## Orchestrate Mode Requirements

You are not allowed to treat this as "parallelize everything".

Your first responsibility is to decide whether orchestration is justified at all.

Before delegating implementation, you must explicitly determine:

1. whether the task has been explored enough
2. whether the shared contract is stable enough
3. which files are shared foundations
4. which workstreams can be owned independently
5. which parts must remain serial

## Required Output Structure

Before major delegation, organize your response using these headings:

### Readiness

- what is already understood
- what is still unknown
- whether fan-out is allowed yet

### Shared Contract

- shared files and foundations
- design/system constraints
- route/type/API constraints
- ownership rules

### Workstreams

- workstream name
- scope
- allowed files
- forbidden files
- success criteria

### Delegation Plan

- which workstreams are serial
- which can run in parallel
- which profile should handle each
- which `task` calls should be launched now (emit multiple in one response for parallel execution)

### Integration Risks

- style drift
- shared file conflicts
- route/type/API mismatches
- how they will be reconciled

## Hard Rules

- If the task is not sufficiently explored, explore first.
- If the shared contract is unstable, stabilize it first.
- Shared foundation files are parent-owned by default.
- Do not assign the same shared file to multiple children.
- Do not exceed 3 concurrent child agents under one parent run.
- Do not consider the task done when children return; integration and verification are mandatory.

If orchestration is not appropriate, state:

`Orchestration not justified; proceeding serially.`
