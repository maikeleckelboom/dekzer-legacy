// Root-bound work admission is retained for the lower-layer root lifecycle
// substrate, which is currently below the maintained boundary.
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::source_media::{SourceMediaOperation, SourceMediaWritePolicy};
use crate::{LibrarySqliteError, LibrarySqliteResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MountEpochStamp {
    pub root_id: i64,
    pub mount_epoch: i64,
}

#[derive(Debug, Default)]
struct SourceAdmissionState {
    frozen: AtomicBool,
    generation: AtomicU64,
}

#[derive(Debug, Clone)]
pub(crate) struct SourceAdmissionToken {
    generation: u64,
    state: Arc<SourceAdmissionState>,
}

impl SourceAdmissionToken {
    pub(crate) fn is_cancelled(&self) -> bool {
        self.state.generation.load(Ordering::SeqCst) != self.generation
    }
}

#[derive(Debug, Default)]
pub(crate) struct SourceAdmissionGate {
    sources: Mutex<HashMap<i64, Arc<SourceAdmissionState>>>,
}

impl SourceAdmissionGate {
    pub(crate) fn try_admit(&self, source_id: i64) -> Option<SourceAdmissionToken> {
        let state = self.state_for(source_id);
        if state.frozen.load(Ordering::SeqCst) {
            return None;
        }

        Some(SourceAdmissionToken {
            generation: state.generation.load(Ordering::SeqCst),
            state,
        })
    }

    pub(crate) fn freeze_sources<'a, I>(&self, source_ids: I)
    where
        I: IntoIterator<Item = &'a i64>,
    {
        for source_id in source_ids {
            self.state_for(*source_id)
                .frozen
                .store(true, Ordering::SeqCst);
        }
    }

    pub(crate) fn cancel_sources<'a, I>(&self, source_ids: I)
    where
        I: IntoIterator<Item = &'a i64>,
    {
        for source_id in source_ids {
            let state = self.state_for(*source_id);
            let was_frozen = state.frozen.swap(true, Ordering::SeqCst);
            if !was_frozen {
                state.generation.fetch_add(1, Ordering::SeqCst);
            }
        }
    }

    pub(crate) fn thaw_sources<'a, I>(&self, source_ids: I)
    where
        I: IntoIterator<Item = &'a i64>,
    {
        for source_id in source_ids {
            self.state_for(*source_id)
                .frozen
                .store(false, Ordering::SeqCst);
        }
    }

    fn state_for(&self, source_id: i64) -> Arc<SourceAdmissionState> {
        let mut sources = self
            .sources
            .lock()
            .expect("source admission gate mutex must not be poisoned");
        sources
            .entry(source_id)
            .or_insert_with(|| Arc::new(SourceAdmissionState::default()))
            .clone()
    }
}

pub(crate) const ROOT_SCAN_SOURCE_MEDIA_WRITE_POLICY: SourceMediaWritePolicy =
    SourceMediaWritePolicy::read_only(SourceMediaOperation::RootScan);

pub(crate) fn admit_source_bound_work(
    gate: &SourceAdmissionGate,
    source_id: i64,
) -> LibrarySqliteResult<SourceAdmissionToken> {
    gate.try_admit(source_id)
        .ok_or(LibrarySqliteError::RootWorkAdmissionDenied { root_id: source_id })
}

pub(crate) fn load_mount_epoch_stamp_for_root(
    connection: &Connection,
    root_id: i64,
) -> LibrarySqliteResult<MountEpochStamp> {
    connection
        .query_row(
            "SELECT mount_epoch
             FROM source_state
             WHERE source_id = ?1",
            [root_id],
            |row| {
                Ok(MountEpochStamp {
                    root_id,
                    mount_epoch: row.get(0)?,
                })
            },
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => LibrarySqliteError::MissingRoot(root_id),
            other => other.into(),
        })
}

pub(crate) fn load_mount_epoch_stamp_for_file(
    connection: &Connection,
    file_id: i64,
) -> LibrarySqliteResult<MountEpochStamp> {
    connection
        .query_row(
            "SELECT sf.source_id,
                    lss.mount_epoch
             FROM source_files sf
             JOIN source_state lss
               ON lss.source_id = sf.source_id
             WHERE sf.source_file_id = ?1",
            [file_id],
            |row| {
                Ok(MountEpochStamp {
                    root_id: row.get(0)?,
                    mount_epoch: row.get(1)?,
                })
            },
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => LibrarySqliteError::MissingFile(file_id),
            other => other.into(),
        })
}

pub(crate) fn validate_mount_epoch_stamp(
    connection: &Connection,
    stamp: MountEpochStamp,
) -> LibrarySqliteResult<()> {
    let current = load_mount_epoch_stamp_for_root(connection, stamp.root_id)?;
    if current.mount_epoch != stamp.mount_epoch {
        return Err(LibrarySqliteError::StaleMountEpochStamp {
            root_id: stamp.root_id,
            admitted_mount_epoch: stamp.mount_epoch,
            current_mount_epoch: current.mount_epoch,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        ROOT_SCAN_SOURCE_MEDIA_WRITE_POLICY, SourceAdmissionGate, admit_source_bound_work,
    };
    use crate::source_media::SourceMediaAccessKind;

    #[test]
    fn root_scan_policy_keeps_read_only_source_access() {
        assert_eq!(
            ROOT_SCAN_SOURCE_MEDIA_WRITE_POLICY.access_kind(),
            SourceMediaAccessKind::ReadOnlySourceAccess
        );
    }

    #[test]
    fn source_admission_gate_cancels_existing_tokens() {
        let gate = SourceAdmissionGate::default();
        let source_id = 7;
        let token = admit_source_bound_work(&gate, source_id).expect("admit source work");

        assert!(!token.is_cancelled());

        gate.cancel_sources([&source_id]);

        assert!(token.is_cancelled());
    }
}
