use rusqlite::Connection;

use crate::LibrarySqliteResult;
use crate::authority::write_lane::admit_write;

use super::seeds;

pub(crate) const CANONICAL_BASELINE_GENERATION: &str = "20260502000000_substrate_baseline";
pub(crate) const BASELINE_SCHEMA_SQL: &str =
    include_str!("../../migrations/20260502000000_substrate_baseline.sql");

pub(crate) fn install_canonical_baseline(
    connection: &mut Connection,
    created_at_ms: i64,
) -> LibrarySqliteResult<()> {
    connection.execute_batch("PRAGMA auto_vacuum = INCREMENTAL; VACUUM;")?;
    admit_write(connection, |write| {
        write.execute_batch(BASELINE_SCHEMA_SQL)?;
        seeds::install_seed_rows(write, created_at_ms)?;
        Ok(())
    })
}
