use rusqlite::params;

use crate::LibrarySqliteResult;
use crate::authority::write_lane::AdmittedWrite;
use library_domain::{
    CapabilityKind, LibraryAssetId, PrepPolicyId, PrepTargetStabilityClass, WorkPriorityClass,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedLibraryAssetPrepTargetInput {
    pub capability_kind: CapabilityKind,
    pub target_profile_key: String,
    pub target_quality: i64,
    pub target_stability_class: PrepTargetStabilityClass,
    pub priority_class: WorkPriorityClass,
    pub resolved_from_policy_id: PrepPolicyId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceResolvedLibraryAssetPrepTargetsInput {
    pub library_asset_id: LibraryAssetId,
    pub targets: Vec<ResolvedLibraryAssetPrepTargetInput>,
    pub updated_at: i64,
}

pub struct ResolvedTargetsAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> ResolvedTargetsAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn replace_resolved_library_asset_prep_targets(
        &self,
        input: &ReplaceResolvedLibraryAssetPrepTargetsInput,
    ) -> LibrarySqliteResult<()> {
        self.tx.execute(
            "DELETE FROM ResolvedLibraryAssetPrepTargets
             WHERE library_asset_id = ?1",
            [input.library_asset_id.get()],
        )?;

        for target in &input.targets {
            self.tx.execute(
                "INSERT INTO ResolvedLibraryAssetPrepTargets (
                     library_asset_id,
                     capability_kind,
                     target_profile_key,
                     target_quality,
                     target_stability_class,
                     priority_class,
                     resolved_from_policy_id,
                     updated_at
                 )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    input.library_asset_id.get(),
                    target.capability_kind.as_str(),
                    target.target_profile_key,
                    target.target_quality,
                    target.target_stability_class.as_str(),
                    target.priority_class.as_str(),
                    target.resolved_from_policy_id.get(),
                    input.updated_at,
                ],
            )?;
        }

        Ok(())
    }
}
