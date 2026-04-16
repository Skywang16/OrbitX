/*!
 * Tool execution logger placeholder.
 *
 * The rollout-first refactor no longer persists tool execution rows in SQL.
 * Tool lifecycle is captured through thread rollout events instead.
 */

pub struct ToolExecutionLogger;

impl ToolExecutionLogger {
    pub fn new(_verbose: bool) -> Self {
        Self
    }
}
