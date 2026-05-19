use rusqlite::{OptionalExtension, params};

use crate::authority::write_lane::AdmittedWrite;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{CapabilityKind, PrepPolicyId, PrepTargetStabilityClass, WorkPriorityClass};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrepPolicyTargetInput {
    pub capability_kind: CapabilityKind,
    pub target_profile_key: String,
    pub target_quality: i64,
    pub target_stability_class: PrepTargetStabilityClass,
    pub priority_class: WorkPriorityClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertPrepPolicyInput {
    pub prep_policy_id: Option<PrepPolicyId>,
    pub policy_name: String,
    pub is_system_policy: bool,
    pub is_user_editable: bool,
    pub changed_at: i64,
    pub targets: Vec<PrepPolicyTargetInput>,
}

pub struct PrepPoliciesAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> PrepPoliciesAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn upsert_prep_policy(
        &self,
        input: &UpsertPrepPolicyInput,
    ) -> LibrarySqliteResult<PrepPolicyId> {
        let prep_policy_id = if let Some(prep_policy_id) = self.resolve_existing_id(input)? {
            self.tx.execute(
                "UPDATE PrepPolicies
                 SET policy_name = ?2,
                     is_system_policy = ?3,
                     is_user_editable = ?4,
                     updated_at = ?5
                 WHERE prep_policy_id = ?1",
                params![
                    prep_policy_id.get(),
                    input.policy_name,
                    if input.is_system_policy { 1 } else { 0 },
                    if input.is_user_editable { 1 } else { 0 },
                    input.changed_at,
                ],
            )?;
            prep_policy_id
        } else if let Some(explicit_id) = input.prep_policy_id {
            self.tx.execute(
                "INSERT INTO PrepPolicies (
                     prep_policy_id,
                     policy_name,
                     is_system_policy,
                     is_user_editable,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![
                    explicit_id.get(),
                    input.policy_name,
                    if input.is_system_policy { 1 } else { 0 },
                    if input.is_user_editable { 1 } else { 0 },
                    input.changed_at,
                ],
            )?;
            explicit_id
        } else {
            self.tx.execute(
                "INSERT INTO PrepPolicies (
                     policy_name,
                     is_system_policy,
                     is_user_editable,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?4)",
                params![
                    input.policy_name,
                    if input.is_system_policy { 1 } else { 0 },
                    if input.is_user_editable { 1 } else { 0 },
                    input.changed_at,
                ],
            )?;
            parse_prep_policy_id(self.tx.last_insert_rowid())?
        };

        self.replace_policy_targets(prep_policy_id, &input.targets)?;
        Ok(prep_policy_id)
    }

    fn resolve_existing_id(
        &self,
        input: &UpsertPrepPolicyInput,
    ) -> LibrarySqliteResult<Option<PrepPolicyId>> {
        if let Some(prep_policy_id) = input.prep_policy_id
            && self.tx.query_row(
                "SELECT EXISTS(
                         SELECT 1
                         FROM PrepPolicies
                         WHERE prep_policy_id = ?1
                     )",
                [prep_policy_id.get()],
                |row| row.get::<_, i64>(0),
            )? != 0
        {
            return Ok(Some(prep_policy_id));
        }

        self.tx
            .query_row(
                "SELECT prep_policy_id
                 FROM PrepPolicies
                 WHERE policy_name = ?1",
                [input.policy_name.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map(|maybe_id| maybe_id.map(parse_prep_policy_id).transpose())?
    }

    fn replace_policy_targets(
        &self,
        prep_policy_id: PrepPolicyId,
        targets: &[PrepPolicyTargetInput],
    ) -> LibrarySqliteResult<()> {
        self.tx.execute(
            "DELETE FROM PrepPolicyTargets
             WHERE prep_policy_id = ?1",
            [prep_policy_id.get()],
        )?;

        for target in targets {
            self.tx.execute(
                "INSERT INTO PrepPolicyTargets (
                     prep_policy_id,
                     capability_kind,
                     target_profile_key,
                     target_quality,
                     target_stability_class,
                     priority_class
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    prep_policy_id.get(),
                    target.capability_kind.as_str(),
                    target.target_profile_key,
                    target.target_quality,
                    target.target_stability_class.as_str(),
                    target.priority_class.as_str(),
                ],
            )?;
        }

        Ok(())
    }
}

fn parse_prep_policy_id(value: i64) -> LibrarySqliteResult<PrepPolicyId> {
    PrepPolicyId::new(value).ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!(
            "invalid PrepPolicies.prep_policy_id value: {value}"
        ))
    })
}
