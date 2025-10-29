use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::mux::{PaneId, PtySize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::enum_variant_names)]
pub enum MuxNotification {
    PaneOutput { pane_id: PaneId, data: Bytes },
    PaneAdded(PaneId),
    PaneRemoved(PaneId),
    PaneResized { pane_id: PaneId, size: PtySize },
    PaneExited {
        pane_id: PaneId,
        exit_code: Option<i32>,
    },
}
