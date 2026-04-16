use chrono::{DateTime, TimeZone, Utc};
use tracing::warn;

pub(crate) fn timestamp_to_datetime(ts: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(ts, 0).single().unwrap_or_else(|| {
        warn!("Invalid UTC timestamp '{}', falling back to UNIX_EPOCH", ts);
        DateTime::<Utc>::from(std::time::UNIX_EPOCH)
    })
}
