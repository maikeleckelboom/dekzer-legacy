use rusqlite::params;

use crate::authority::write_lane::AdmittedWrite;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{LibraryAssetId, PrepAssignmentScopeKind, PrepScope, SourceId};

const LIBRARY_SCOPE_STORAGE_ID: &str = "library";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EncodedPrepScope {
    pub scope_kind: &'static str,
    pub scope_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrepAssignmentInput {
    pub prep_assignment_id: Option<i64>,
    pub prep_policy_id: i64,
    pub precedence_rank: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplacePrepAssignmentsInput {
    pub scope: PrepScope,
    pub assignments: Vec<PrepAssignmentInput>,
    pub changed_at: i64,
}

pub struct PrepAssignmentsAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> PrepAssignmentsAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn replace_prep_assignments(
        &self,
        input: &ReplacePrepAssignmentsInput,
    ) -> LibrarySqliteResult<()> {
        let scope = encode_prep_scope_for_storage(input.scope);

        self.tx.execute(
            "DELETE FROM PrepAssignments
             WHERE scope_kind = ?1
               AND scope_id = ?2",
            params![scope.scope_kind, scope.scope_id.as_str()],
        )?;

        for assignment in &input.assignments {
            match assignment.prep_assignment_id {
                Some(prep_assignment_id) => {
                    self.tx.execute(
                        "INSERT INTO PrepAssignments (
                             prep_assignment_id,
                             scope_kind,
                             scope_id,
                             prep_policy_id,
                             precedence_rank,
                             created_at,
                             updated_at
                         )
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                        params![
                            prep_assignment_id,
                            scope.scope_kind,
                            scope.scope_id.as_str(),
                            assignment.prep_policy_id,
                            assignment.precedence_rank,
                            input.changed_at,
                        ],
                    )?;
                }
                None => {
                    self.tx.execute(
                        "INSERT INTO PrepAssignments (
                             scope_kind,
                             scope_id,
                             prep_policy_id,
                             precedence_rank,
                             created_at,
                             updated_at
                         )
                         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                        params![
                            scope.scope_kind,
                            scope.scope_id.as_str(),
                            assignment.prep_policy_id,
                            assignment.precedence_rank,
                            input.changed_at,
                        ],
                    )?;
                }
            }
        }

        Ok(())
    }
}

pub(crate) fn encode_prep_scope_for_storage(scope: PrepScope) -> EncodedPrepScope {
    match scope {
        PrepScope::Library => EncodedPrepScope {
            scope_kind: PrepAssignmentScopeKind::Library.as_str(),
            scope_id: LIBRARY_SCOPE_STORAGE_ID.to_string(),
        },
        PrepScope::Source(id) => EncodedPrepScope {
            scope_kind: PrepAssignmentScopeKind::Source.as_str(),
            scope_id: id.get().to_string(),
        },
        PrepScope::LibraryAsset(id) => EncodedPrepScope {
            scope_kind: PrepAssignmentScopeKind::LibraryAsset.as_str(),
            scope_id: id.get().to_string(),
        },
    }
}

#[allow(dead_code)]
pub(crate) fn decode_prep_scope_from_storage(
    scope_kind: &str,
    scope_id: &str,
) -> LibrarySqliteResult<PrepScope> {
    let scope_kind = PrepAssignmentScopeKind::parse(scope_kind).ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!(
            "unknown PrepAssignments.scope_kind value: {scope_kind}"
        ))
    })?;

    match scope_kind {
        PrepAssignmentScopeKind::Library if scope_id == LIBRARY_SCOPE_STORAGE_ID => {
            Ok(PrepScope::Library)
        }
        PrepAssignmentScopeKind::Library => Err(invalid_prep_scope_id(scope_kind, scope_id)),
        PrepAssignmentScopeKind::Source => {
            let id = parse_positive_id(scope_kind, scope_id)?;
            SourceId::new(id)
                .map(PrepScope::Source)
                .ok_or_else(|| invalid_prep_scope_id(scope_kind, scope_id))
        }
        PrepAssignmentScopeKind::LibraryAsset => {
            let id = parse_positive_id(scope_kind, scope_id)?;
            LibraryAssetId::new(id)
                .map(PrepScope::LibraryAsset)
                .ok_or_else(|| invalid_prep_scope_id(scope_kind, scope_id))
        }
    }
}

#[allow(dead_code)]
fn parse_positive_id(
    scope_kind: PrepAssignmentScopeKind,
    scope_id: &str,
) -> LibrarySqliteResult<i64> {
    scope_id
        .parse::<i64>()
        .map_err(|_| invalid_prep_scope_id(scope_kind, scope_id))
}

#[allow(dead_code)]
fn invalid_prep_scope_id(
    scope_kind: PrepAssignmentScopeKind,
    scope_id: &str,
) -> LibrarySqliteError {
    LibrarySqliteError::WriteInvariant(format!(
        "invalid PrepAssignments.scope_id value for {}: {scope_id}",
        scope_kind.as_str()
    ))
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::{
        PrepAssignmentInput, PrepAssignmentsAuthorityTx, ReplacePrepAssignmentsInput,
        decode_prep_scope_from_storage, encode_prep_scope_for_storage,
    };
    use crate::LibrarySqliteError;
    use crate::authority::write_lane::admit_write;
    use crate::schema::install_baseline_schema_for_test;
    use library_domain::{LibraryAssetId, PrepScope, SourceId};

    #[test]
    fn prep_scope_storage_encoding_round_trips_for_supported_variants() {
        let scopes = [
            PrepScope::Library,
            PrepScope::Source(source_id(11)),
            PrepScope::LibraryAsset(library_asset_id(22)),
        ];

        for scope in scopes {
            let encoded = encode_prep_scope_for_storage(scope);
            let decoded = decode_prep_scope_from_storage(encoded.scope_kind, &encoded.scope_id)
                .expect("encoded scope should decode");

            assert_eq!(decoded, scope);
        }
    }

    #[test]
    fn prep_scope_storage_decoding_rejects_invalid_persisted_pairs() {
        assert_write_invariant(decode_prep_scope_from_storage("missing", "1"));
        assert_write_invariant(decode_prep_scope_from_storage("library", "1"));
        assert_write_invariant(decode_prep_scope_from_storage("source", "0"));
        assert_write_invariant(decode_prep_scope_from_storage("library_asset", "not-an-id"));
    }

    #[test]
    fn replace_prep_assignments_input_preserves_typed_scope() {
        let scope = PrepScope::Source(source_id(33));
        let input = ReplacePrepAssignmentsInput {
            scope,
            assignments: Vec::new(),
            changed_at: 10,
        };

        assert_eq!(input.scope, scope);
    }

    #[test]
    fn replace_prep_assignments_writes_typed_scope_through_to_raw_storage_pair() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline schema");

        admit_write(&mut connection, |write| {
            write.execute(
                "INSERT INTO PrepPolicies (
                     prep_policy_id,
                     policy_name,
                     is_system_policy,
                     is_user_editable,
                     created_at,
                     updated_at
                 )
                 VALUES (7, 'test-policy', 0, 1, 1, 1)",
                [],
            )?;

            PrepAssignmentsAuthorityTx::new(write).replace_prep_assignments(
                &ReplacePrepAssignmentsInput {
                    scope: PrepScope::LibraryAsset(library_asset_id(44)),
                    assignments: vec![PrepAssignmentInput {
                        prep_assignment_id: Some(8),
                        prep_policy_id: 7,
                        precedence_rank: 0,
                    }],
                    changed_at: 2,
                },
            )?;

            Ok(())
        })
        .expect("replace prep assignments");

        let row = connection
            .query_row(
                "SELECT scope_kind, scope_id, prep_policy_id, precedence_rank, updated_at
                 FROM PrepAssignments
                 WHERE prep_assignment_id = 8",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                    ))
                },
            )
            .expect("read prep assignment");

        assert_eq!(
            row,
            ("library_asset".to_string(), "44".to_string(), 7, 0, 2)
        );
    }

    fn assert_write_invariant(result: Result<PrepScope, LibrarySqliteError>) {
        match result {
            Err(LibrarySqliteError::WriteInvariant(_)) => {}
            other => panic!("expected write invariant, found {other:?}"),
        }
    }

    fn source_id(value: i64) -> SourceId {
        SourceId::new(value).expect("positive source id")
    }

    fn library_asset_id(value: i64) -> LibraryAssetId {
        LibraryAssetId::new(value).expect("positive library asset id")
    }
}
