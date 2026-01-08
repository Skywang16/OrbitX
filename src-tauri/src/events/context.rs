use serde::Serialize;

use crate::mux::PaneId;
use crate::terminal::TerminalContext;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TerminalContextEvent {
    ActivePaneChanged {
        old_pane_id: Option<PaneId>,
        new_pane_id: Option<PaneId>,
    },
    PaneContextUpdated {
        pane_id: PaneId,
        context: TerminalContext,
    },
    PaneShellIntegrationChanged {
        pane_id: PaneId,
        enabled: bool,
    },
    PaneCwdChanged {
        pane_id: PaneId,
        old_cwd: Option<String>,
        new_cwd: String,
    },
}
