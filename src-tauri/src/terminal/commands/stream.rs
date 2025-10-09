use serde::Deserialize;
use tauri::{ipc::Channel, State};

use crate::api_success;
use crate::utils::{EmptyData, TauriApiResult};

use super::super::channel_state::TerminalChannelState;
use super::super::types::TerminalChannelMessage;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaneArgs {
    #[serde(alias = "paneId", alias = "pane_id")]
    pane_id: u32,
}

#[tauri::command]
pub async fn terminal_subscribe_output(
    args: PaneArgs,
    channel: Channel<TerminalChannelMessage>,
    state: State<'_, TerminalChannelState>,
) -> TauriApiResult<EmptyData> {
    state.manager.register(args.pane_id, channel);
    Ok(api_success!())
}

#[tauri::command]
pub async fn terminal_subscribe_output_cancel(
    args: PaneArgs,
    state: State<'_, TerminalChannelState>,
) -> TauriApiResult<EmptyData> {
    state.manager.close(args.pane_id);
    Ok(api_success!())
}
