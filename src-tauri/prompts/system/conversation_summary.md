Your task is to create a detailed summary of the conversation so far, paying close attention to the user's explicit requests and your previous actions.

This summary should be thorough in capturing technical details, code patterns, and architectural decisions that would be essential for continuing with the conversation and supporting any continuing tasks.

Your summary should be structured as follows:

## Context

If applicable based on the current task, this should include:

### 1. Previous Conversation

High level details about what was discussed throughout the entire conversation with the user. This should be written to allow someone to be able to follow the general overarching conversation flow.

### 2. Current Work

Describe in detail what was being worked on prior to this request to summarize the conversation. Pay special attention to the more recent messages in the conversation.

### 3. Key Technical Concepts

List all important technical concepts, technologies, coding conventions, and frameworks discussed, which might be relevant for continuing with this work.

### 4. Relevant Files and Code

If applicable, enumerate specific files and code sections examined, modified, or created for the task continuation. Pay special attention to the most recent messages and changes.

### 5. Tools Used

List the tools that were used and what information was obtained from them (especially file paths, directory structures, command outputs).

### 6. Problem Solving

Document problems solved thus far and any ongoing troubleshooting efforts.

### 7. Pending Tasks and Next Steps

Outline all pending tasks that you have explicitly been asked to work on, as well as list the next steps you will take for all outstanding work, if applicable. Include direct quotes from the most recent conversation showing exactly what task you were working on and where you left off.

## Example Summary Structure

1. Previous Conversation:
   [Detailed description]

2. Current Work:
   [Detailed description]

3. Key Technical Concepts:
   - [Concept 1]
   - [Concept 2]

4. Relevant Files and Code:
   - src/main.rs: Core application logic, modified function xyz()
   - config.json: Application configuration

5. Tools Used:
   - read_file: Read src/main.rs (500 lines), src/lib.rs (300 lines)
   - list_files: Listed src/ directory (20 files)
   - execute_command: Ran 'cargo build' (successful)

6. Problem Solving:
   [Detailed description]

7. Pending Tasks and Next Steps:
   - [Task 1 details & next steps]

Output only the summary of the conversation so far, without any additional commentary or explanation.
