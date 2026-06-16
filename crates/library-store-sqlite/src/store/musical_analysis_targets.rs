use std::path::PathBuf;

use library_domain::SourceFileId;
use rusqlite::OptionalExtension;
use thiserror::Error;

use crate::{LibrarySqliteError, LibrarySqliteResult};

use super::SqliteDurableStore;
use super::source_file_hash::{ResolveSourceFilePathError, SourceFileHashBasis};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorePlayableMediaAnalysisTarget {
    pub playable_media_id: i64,
    pub attachment_id: i64,
    pub media_kind: String,
    pub mime_type: Option<String>,
    pub codec: Option<String>,
    pub source_file_basis: StorePlayableMediaAnalysisSourceFileBasis,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorePlayableMediaAnalysisSourceFileBasis {
    pub source_file_id: i64,
    pub source_id: i64,
    pub relative_path: String,
    pub file_kind: String,
}

#[derive(Debug, Error)]
pub enum StorePlayableMediaAnalysisTargetError {
    #[error(transparent)]
    Store(#[from] LibrarySqliteError),
    #[error("playable media {playable_media_id} does not exist")]
    PlayableMediaNotFound { playable_media_id: i64 },
    #[error(
        "playable media {playable_media_id} has invalid evidence_source_file_id {source_file_id}"
    )]
    InvalidEvidenceSourceFileId {
        playable_media_id: i64,
        source_file_id: i64,
    },
    #[error("source file {source_file_id} does not exist")]
    SourceFileNotFound { source_file_id: i64 },
    #[error(
        "source file {source_file_id} is not resolvable because presence_state is {presence_state}"
    )]
    SourceFileUnavailable {
        source_file_id: i64,
        presence_state: String,
    },
    #[error("source root is unavailable for source file {source_file_id}")]
    SourceRootUnavailable {
        source_file_id: i64,
        source_id: i64,
        mount_status: Option<String>,
        access_state: Option<String>,
        access_issue_kind: Option<String>,
    },
    #[error("source root is missing for source file {source_file_id}: {detail:?}")]
    SourceRootMissing {
        source_file_id: i64,
        source_id: i64,
        detail: Option<String>,
    },
    #[error("source root is blocked for source file {source_file_id}: {detail:?}")]
    SourceRootBlocked {
        source_file_id: i64,
        source_id: i64,
        access_issue_kind: Option<String>,
        detail: Option<String>,
    },
    #[error("source file {source_file_id} has invalid relative_path {relative_path:?}: {reason}")]
    InvalidRelativePath {
        source_file_id: i64,
        relative_path: String,
        reason: String,
    },
    #[error("source file {source_file_id} path escapes the source root")]
    SourceFilePathEscapesRoot { source_file_id: i64 },
    #[error("physical source file {source_file_id} is missing at {path:?}")]
    PhysicalFileMissing { source_file_id: i64, path: PathBuf },
    #[error("physical source file {source_file_id} is blocked at {path:?}: {detail}")]
    PhysicalFileBlocked {
        source_file_id: i64,
        path: PathBuf,
        detail: String,
    },
}

impl SqliteDurableStore {
    pub fn resolve_playable_media_analysis_target(
        &self,
        playable_media_id: i64,
    ) -> Result<StorePlayableMediaAnalysisTarget, StorePlayableMediaAnalysisTargetError> {
        let row = self
            .read_playable_media_analysis_target_row(playable_media_id)?
            .ok_or(
                StorePlayableMediaAnalysisTargetError::PlayableMediaNotFound { playable_media_id },
            )?;
        let source_file_id = SourceFileId::new(row.evidence_source_file_id).ok_or_else(|| {
            StorePlayableMediaAnalysisTargetError::InvalidEvidenceSourceFileId {
                playable_media_id,
                source_file_id: row.evidence_source_file_id,
            }
        })?;
        let resolved = self
            .resolve_source_file_path_for_observation(source_file_id)
            .map_err(StorePlayableMediaAnalysisTargetError::from)?;

        Ok(StorePlayableMediaAnalysisTarget {
            playable_media_id: row.playable_media_id,
            attachment_id: row.attachment_id,
            media_kind: row.media_kind,
            mime_type: row.mime_type,
            codec: row.codec,
            source_file_basis: source_file_basis(resolved.basis),
            path: resolved.path,
        })
    }

    fn read_playable_media_analysis_target_row(
        &self,
        playable_media_id: i64,
    ) -> LibrarySqliteResult<Option<PlayableMediaAnalysisTargetRow>> {
        let connection = self.open_read_connection()?;
        connection
            .query_row(
                "SELECT playable_media_id,
                        attachment_id,
                        evidence_source_file_id,
                        media_kind,
                        mime_type,
                        codec
                 FROM playable_media
                 WHERE playable_media_id = ?1",
                [playable_media_id],
                |row| {
                    Ok(PlayableMediaAnalysisTargetRow {
                        playable_media_id: row.get(0)?,
                        attachment_id: row.get(1)?,
                        evidence_source_file_id: row.get(2)?,
                        media_kind: row.get(3)?,
                        mime_type: row.get(4)?,
                        codec: row.get(5)?,
                    })
                },
            )
            .optional()
            .map_err(LibrarySqliteError::from)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlayableMediaAnalysisTargetRow {
    playable_media_id: i64,
    attachment_id: i64,
    evidence_source_file_id: i64,
    media_kind: String,
    mime_type: Option<String>,
    codec: Option<String>,
}

fn source_file_basis(basis: SourceFileHashBasis) -> StorePlayableMediaAnalysisSourceFileBasis {
    StorePlayableMediaAnalysisSourceFileBasis {
        source_file_id: basis.source_file_id.get(),
        source_id: basis.source_id,
        relative_path: basis.relative_path,
        file_kind: basis.file_kind,
    }
}

impl From<ResolveSourceFilePathError> for StorePlayableMediaAnalysisTargetError {
    fn from(error: ResolveSourceFilePathError) -> Self {
        match error {
            ResolveSourceFilePathError::Store(error) => Self::Store(error),
            ResolveSourceFilePathError::SourceFileNotFound { source_file_id } => {
                Self::SourceFileNotFound { source_file_id }
            }
            ResolveSourceFilePathError::SourceFileUnavailable {
                source_file_id,
                presence_state,
            } => Self::SourceFileUnavailable {
                source_file_id,
                presence_state,
            },
            ResolveSourceFilePathError::SourceRootUnavailable {
                source_file_id,
                source_id,
                mount_status,
                access_state,
                access_issue_kind,
            } => Self::SourceRootUnavailable {
                source_file_id,
                source_id,
                mount_status,
                access_state,
                access_issue_kind,
            },
            ResolveSourceFilePathError::SourceRootMissing {
                source_file_id,
                source_id,
                detail,
            } => Self::SourceRootMissing {
                source_file_id,
                source_id,
                detail,
            },
            ResolveSourceFilePathError::SourceRootBlocked {
                source_file_id,
                source_id,
                access_issue_kind,
                detail,
            } => Self::SourceRootBlocked {
                source_file_id,
                source_id,
                access_issue_kind,
                detail,
            },
            ResolveSourceFilePathError::InvalidRelativePath {
                source_file_id,
                relative_path,
                reason,
            } => Self::InvalidRelativePath {
                source_file_id,
                relative_path,
                reason,
            },
            ResolveSourceFilePathError::SourceFilePathEscapesRoot { source_file_id } => {
                Self::SourceFilePathEscapesRoot { source_file_id }
            }
            ResolveSourceFilePathError::PhysicalFileMissing {
                source_file_id,
                path,
            } => Self::PhysicalFileMissing {
                source_file_id,
                path,
            },
            ResolveSourceFilePathError::PhysicalFileBlocked {
                source_file_id,
                path,
                source,
            } => Self::PhysicalFileBlocked {
                source_file_id,
                path,
                detail: source.to_string(),
            },
        }
    }
}
