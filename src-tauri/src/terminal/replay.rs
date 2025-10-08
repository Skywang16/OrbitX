use crate::completion::output_analyzer::OutputAnalyzer;
use crate::mux::{singleton::get_mux, PaneId, PtySize};
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReplayEvent {
    pub data: String,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProcessReplayEvent {
    pub events: Vec<ReplayEvent>,
}

impl ProcessReplayEvent {
    pub fn empty() -> Self {
        Self { events: Vec::new() }
    }

    pub fn from_buffer(buffer: String, size: PtySize) -> Self {
        if buffer.is_empty() {
            debug!("ProcessReplayEvent::from_buffer -> empty buffer");
            return Self::empty();
        }

        Self {
            events: vec![ReplayEvent {
                data: buffer,
                cols: size.cols,
                rows: size.rows,
            }],
        }
    }
}

pub fn build_replay(pane_id: u32) -> anyhow::Result<ProcessReplayEvent> {
    let text = OutputAnalyzer::global()
        .get_pane_buffer(pane_id)
        .map_err(|err| anyhow::anyhow!("Failed to read terminal buffer: {err}"))?;

    let mux = get_mux();
    let size = mux
        .get_pane(PaneId::new(pane_id))
        .map(|pane| pane.get_size())
        .unwrap_or_else(PtySize::default);

    Ok(ProcessReplayEvent::from_buffer(text, size))
}

pub fn mark_replay_complete(pane_id: u32) {
    debug!(pane_id, "mark_replay_complete");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_buffer() {
        let replay = ProcessReplayEvent::from_buffer(String::new(), PtySize::default());
        assert!(replay.events.is_empty());
    }

    #[test]
    fn test_non_empty_buffer() {
        let replay = ProcessReplayEvent::from_buffer(
            "test".to_string(),
            PtySize {
                cols: 80,
                rows: 24,
                pixel_width: 0,
                pixel_height: 0,
            },
        );
        assert_eq!(replay.events.len(), 1);
        assert_eq!(replay.events[0].data, "test");
    }
}
