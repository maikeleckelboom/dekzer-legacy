use rusqlite::{OptionalExtension, params};

use crate::LibrarySqliteResult;
use crate::authority::write_lane::AdmittedWrite;
use library_domain::SourceAccessIssueKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceRootNavigationWindowState {
    Unknown,
    Established,
    Empty,
    Missing,
    Blocked,
    Failed,
}

impl SourceRootNavigationWindowState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Established => "established",
            Self::Empty => "empty",
            Self::Missing => "missing",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "established" => Self::Established,
            "empty" => Self::Empty,
            "missing" => Self::Missing,
            "blocked" => Self::Blocked,
            "failed" => Self::Failed,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRootNavigationStateRecord {
    pub source_id: i64,
    pub root_reach_state: SourceRootNavigationWindowState,
    pub immediate_child_directory_count: i64,
    pub issue_kind: Option<String>,
    pub detail: Option<String>,
    pub checked_at: Option<i64>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertSourceRootNavigationStateInput {
    pub source_id: i64,
    pub root_reach_state: SourceRootNavigationWindowState,
    pub immediate_child_directory_count: i64,
    pub issue_kind: Option<SourceAccessIssueKind>,
    pub detail: Option<String>,
    pub checked_at: Option<i64>,
    pub updated_at: i64,
}

pub struct SourceRootNavigationStateAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> SourceRootNavigationStateAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn upsert_source_root_navigation_state(
        &self,
        input: &UpsertSourceRootNavigationStateInput,
    ) -> LibrarySqliteResult<()> {
        self.tx.execute(
            "INSERT INTO source_root_navigation_state (
                 source_id,
                 root_reach_state,
                 immediate_child_directory_count,
                 issue_kind,
                 detail,
                 checked_at,
                 updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(source_id) DO UPDATE SET
                 root_reach_state = excluded.root_reach_state,
                 immediate_child_directory_count = excluded.immediate_child_directory_count,
                 issue_kind = excluded.issue_kind,
                 detail = excluded.detail,
                 checked_at = excluded.checked_at,
                 updated_at = excluded.updated_at",
            params![
                input.source_id,
                input.root_reach_state.as_str(),
                input.immediate_child_directory_count,
                input.issue_kind.map(SourceAccessIssueKind::as_str),
                input.detail.as_deref(),
                input.checked_at,
                input.updated_at,
            ],
        )?;
        Ok(())
    }
}

pub(crate) fn read_source_root_navigation_state(
    connection: &rusqlite::Connection,
    source_id: i64,
) -> LibrarySqliteResult<Option<SourceRootNavigationStateRecord>> {
    connection
        .query_row(
            "SELECT source_id,
                    root_reach_state,
                    immediate_child_directory_count,
                    issue_kind,
                    detail,
                    checked_at,
                    updated_at
             FROM source_root_navigation_state
             WHERE source_id = ?1",
            [source_id],
            |row| {
                let root_reach_state = row.get::<_, String>(1)?;
                Ok(SourceRootNavigationStateRecord {
                    source_id: row.get(0)?,
                    root_reach_state: SourceRootNavigationWindowState::parse(&root_reach_state),
                    immediate_child_directory_count: row.get(2)?,
                    issue_kind: row.get(3)?,
                    detail: row.get(4)?,
                    checked_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}
