Create a new skill to extend the agent's capabilities with specialized knowledge and instructions.

{{input}}

## Skill Creation Guidelines

### 1. Skill Metadata

Define clear metadata for the skill:

- **Name**: Short, descriptive identifier (kebab-case)
- **Description**: One-line summary of what the skill does
- **Version**: Semantic version (e.g., 1.0.0)
- **Author**: Creator information (optional)
- **Tags**: Relevant categories for discoverability

### 2. Skill Instructions

Provide detailed, step-by-step instructions for the AI agent:

- Clear objectives and goals
- Specific actions to take
- Decision-making criteria
- Edge cases and error handling
- Expected outcomes

### 3. Usage Examples

Include concrete examples showing:

- Typical use cases
- Input formats and parameters
- Expected outputs
- Common variations

### 4. Technical Specifications

Document technical details:

- Required tools or dependencies
- File structure and locations
- Configuration requirements
- Integration points with existing code

### 5. Best Practices

Include guidance on:

- When to use this skill
- When NOT to use this skill
- Common pitfalls to avoid
- Performance considerations

## Output Format

Create a markdown file following this structure:

```markdown
# Skill: [Name]

**Description**: [One-line description]
**Version**: [Version number]

## Instructions

[Detailed step-by-step instructions for the AI agent]

## Usage Examples

### Example 1: [Scenario]

[Input and expected output]

## Technical Details

[Dependencies, file locations, configuration]

## Best Practices

[Guidelines and recommendations]
```

Save the skill to `.orbitx/skills/[skill-name]/SKILL.md` in the workspace.
