use std::time::{SystemTime, UNIX_EPOCH};

use crate::LibrarySqliteResult;

pub(crate) fn unix_time_ms() -> LibrarySqliteResult<i64> {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH)?;
    Ok(duration.as_millis() as i64)
}
