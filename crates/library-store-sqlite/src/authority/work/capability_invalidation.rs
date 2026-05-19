use std::collections::BTreeSet;

use rusqlite::{OptionalExtension, params};

use crate::authority::work::{
    QueueComputeCapabilityWorkInput, QueueMachineWorkResult, QueueRebuildProjectionWorkInput,
    WorkItemsAuthorityTx,
};
use crate::authority::write_lane::AdmittedWrite;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{
    CapabilityInvalidationMode, CapabilityKind, CapabilityState, LibraryAssetId, WorkItemId,
    WorkPriorityClass,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkCapabilitiesStaleFromBasisInput {
    pub library_asset_ids: Vec<LibraryAssetId>,
    pub invalidated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkCapabilityStaleFromDependencyInput {
    pub library_asset_id: LibraryAssetId,
    pub upstream_capability_kind: CapabilityKind,
    pub upstream_basis_fingerprint: String,
    pub invalidated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleCapabilityChange {
    pub library_asset_id: LibraryAssetId,
    pub capability_kind: CapabilityKind,
    pub profile_key: String,
    pub queued_work_item_id: Option<WorkItemId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkCapabilitiesStaleResult {
    pub stale_capabilities: Vec<StaleCapabilityChange>,
    pub projection_rebuilds: Vec<QueueMachineWorkResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CapabilityTarget {
    capability_kind: CapabilityKind,
    profile_key: String,
    quality_current: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CapabilityTargetRaw {
    capability_kind: String,
    profile_key: String,
    quality_current: Option<i64>,
}

pub struct CapabilityInvalidationAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> CapabilityInvalidationAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn mark_capabilities_stale_from_basis(
        &self,
        input: &MarkCapabilitiesStaleFromBasisInput,
    ) -> LibrarySqliteResult<MarkCapabilitiesStaleResult> {
        let mut stale_capabilities = Vec::new();
        let mut projection_rebuilds = Vec::new();
        let work_items = WorkItemsAuthorityTx::new(self.tx);

        for library_asset_id in input
            .library_asset_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
        {
            let basis_fingerprint =
                load_library_asset_basis_fingerprint(self.tx, library_asset_id)?;
            let stale_start = stale_capabilities.len();
            for capability in load_capability_targets(self.tx, library_asset_id, None)? {
                mark_capability_row_stale(
                    self.tx,
                    library_asset_id,
                    &capability.capability_kind,
                    &capability.profile_key,
                    input.invalidated_at,
                )?;
                let queued_work_item_id =
                    if let Some(current_basis_fingerprint) = basis_fingerprint.as_deref() {
                        let target_quality = resolve_target_quality(
                            self.tx,
                            library_asset_id,
                            &capability.capability_kind,
                            &capability.profile_key,
                            capability.quality_current,
                        )?;
                        let priority_class = resolve_target_priority(
                            self.tx,
                            library_asset_id,
                            &capability.capability_kind,
                            &capability.profile_key,
                        )?;
                        Some(
                            work_items
                                .queue_compute_capability_work(&QueueComputeCapabilityWorkInput {
                                    library_asset_id,
                                    capability_kind: capability.capability_kind.clone(),
                                    target_profile_key: capability.profile_key.clone(),
                                    target_quality,
                                    basis_fingerprint: current_basis_fingerprint.to_string(),
                                    priority_class,
                                    queued_at: input.invalidated_at,
                                })?
                                .work_item_id,
                        )
                    } else {
                        None
                    };
                stale_capabilities.push(StaleCapabilityChange {
                    library_asset_id,
                    capability_kind: capability.capability_kind,
                    profile_key: capability.profile_key,
                    queued_work_item_id,
                });
            }

            if stale_capabilities.len() > stale_start {
                projection_rebuilds.push(work_items.queue_rebuild_projection_work(
                    &QueueRebuildProjectionWorkInput {
                        projection_domain: library_domain::ProjectionDomain::LibraryBrowser,
                        basis_fingerprint: stale_projection_basis(
                            library_asset_id,
                            basis_fingerprint,
                        ),
                        priority_class: WorkPriorityClass::Interactive,
                        queued_at: input.invalidated_at,
                    },
                )?);
            }
        }

        Ok(MarkCapabilitiesStaleResult {
            stale_capabilities,
            projection_rebuilds,
        })
    }

    pub fn mark_capability_stale_from_dependency(
        &self,
        input: &MarkCapabilityStaleFromDependencyInput,
    ) -> LibrarySqliteResult<MarkCapabilitiesStaleResult> {
        if input.upstream_basis_fingerprint.trim().is_empty() {
            return Err(LibrarySqliteError::WriteInvariant(
                "dependency invalidation requires upstream_basis_fingerprint".to_string(),
            ));
        }
        require_supported_dependency_modes(self.tx, &input.upstream_capability_kind)?;
        let downstream_capability_kinds =
            load_downstream_capability_kinds(self.tx, &input.upstream_capability_kind)?;
        if downstream_capability_kinds.is_empty() {
            return Ok(MarkCapabilitiesStaleResult {
                stale_capabilities: Vec::new(),
                projection_rebuilds: Vec::new(),
            });
        }

        let work_items = WorkItemsAuthorityTx::new(self.tx);
        let mut stale_capabilities = Vec::new();
        for capability in load_capability_targets(
            self.tx,
            input.library_asset_id,
            Some(&downstream_capability_kinds),
        )? {
            mark_capability_row_stale(
                self.tx,
                input.library_asset_id,
                &capability.capability_kind,
                &capability.profile_key,
                input.invalidated_at,
            )?;
            let target_quality = resolve_target_quality(
                self.tx,
                input.library_asset_id,
                &capability.capability_kind,
                &capability.profile_key,
                capability.quality_current,
            )?;
            let priority_class = resolve_target_priority(
                self.tx,
                input.library_asset_id,
                &capability.capability_kind,
                &capability.profile_key,
            )?;
            let queued_work_item_id = work_items
                .queue_compute_capability_work(&QueueComputeCapabilityWorkInput {
                    library_asset_id: input.library_asset_id,
                    capability_kind: capability.capability_kind.clone(),
                    target_profile_key: capability.profile_key.clone(),
                    target_quality,
                    basis_fingerprint: input.upstream_basis_fingerprint.clone(),
                    priority_class,
                    queued_at: input.invalidated_at,
                })?
                .work_item_id;
            stale_capabilities.push(StaleCapabilityChange {
                library_asset_id: input.library_asset_id,
                capability_kind: capability.capability_kind,
                profile_key: capability.profile_key,
                queued_work_item_id: Some(queued_work_item_id),
            });
        }

        let projection_rebuild = if stale_capabilities.is_empty() {
            Vec::new()
        } else {
            vec![
                work_items.queue_rebuild_projection_work(&QueueRebuildProjectionWorkInput {
                    projection_domain: library_domain::ProjectionDomain::LibraryBrowser,
                    basis_fingerprint: format!(
                        "dependency_stale:v1:{}:{}",
                        input.library_asset_id.get(),
                        input.upstream_basis_fingerprint
                    ),
                    priority_class: WorkPriorityClass::Interactive,
                    queued_at: input.invalidated_at,
                })?,
            ]
        };

        Ok(MarkCapabilitiesStaleResult {
            stale_capabilities,
            projection_rebuilds: projection_rebuild,
        })
    }
}

pub(crate) fn load_attached_library_asset_ids_for_source_file(
    tx: &AdmittedWrite<'_>,
    source_file_id: i64,
) -> LibrarySqliteResult<Vec<LibraryAssetId>> {
    let ids = tx
        .prepare(
            "SELECT DISTINCT attachment.library_asset_id
         FROM LibraryAssetAttachments attachment
         JOIN SourceSegments segment
           ON segment.source_segment_id = attachment.source_segment_id
         JOIN SourceSegmentSets segment_set
           ON segment_set.source_segment_set_id = segment.source_segment_set_id
         WHERE segment_set.source_file_id = ?1
         ORDER BY attachment.library_asset_id ASC",
        )?
        .query_map([source_file_id], |row| row.get(0))?
        .collect::<Result<Vec<i64>, _>>()?;

    ids.into_iter().map(parse_library_asset_id).collect()
}

fn load_capability_targets(
    tx: &AdmittedWrite<'_>,
    library_asset_id: LibraryAssetId,
    capability_kinds: Option<&[CapabilityKind]>,
) -> LibrarySqliteResult<Vec<CapabilityTarget>> {
    let capabilities = tx
        .prepare(
            "SELECT capability_kind,
                    profile_key,
                    quality_current
             FROM LibraryAssetCapabilities
             WHERE library_asset_id = ?1
             ORDER BY capability_kind ASC, profile_key ASC",
        )?
        .query_map([library_asset_id.get()], |row| {
            Ok(CapabilityTargetRaw {
                capability_kind: row.get(0)?,
                profile_key: row.get(1)?,
                quality_current: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut capabilities = capabilities
        .into_iter()
        .map(|capability| {
            Ok(CapabilityTarget {
                capability_kind: parse_capability_kind(&capability.capability_kind)?,
                profile_key: capability.profile_key,
                quality_current: capability.quality_current,
            })
        })
        .collect::<LibrarySqliteResult<Vec<_>>>()?;

    if let Some(filter) = capability_kinds {
        capabilities.retain(|capability| filter.contains(&capability.capability_kind));
    }

    Ok(capabilities)
}

fn mark_capability_row_stale(
    tx: &AdmittedWrite<'_>,
    library_asset_id: LibraryAssetId,
    capability_kind: &CapabilityKind,
    profile_key: &str,
    invalidated_at: i64,
) -> LibrarySqliteResult<()> {
    tx.execute(
        "UPDATE LibraryAssetCapabilities
         SET state = ?4,
             updated_at = ?5
         WHERE library_asset_id = ?1
           AND capability_kind = ?2
           AND profile_key = ?3",
        params![
            library_asset_id.get(),
            capability_kind.as_str(),
            profile_key,
            CapabilityState::Stale.as_str(),
            invalidated_at,
        ],
    )?;
    Ok(())
}

fn resolve_target_quality(
    tx: &AdmittedWrite<'_>,
    library_asset_id: LibraryAssetId,
    capability_kind: &CapabilityKind,
    profile_key: &str,
    quality_current: Option<i64>,
) -> LibrarySqliteResult<i64> {
    tx.query_row(
        "SELECT target_quality
         FROM ResolvedLibraryAssetPrepTargets
         WHERE library_asset_id = ?1
           AND capability_kind = ?2
           AND target_profile_key = ?3",
        params![
            library_asset_id.get(),
            capability_kind.as_str(),
            profile_key
        ],
        |row| row.get(0),
    )
    .optional()?
    .or(quality_current)
    .ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!(
            "no target_quality exists for library_asset {} capability {} profile {}",
            library_asset_id.get(),
            capability_kind.as_str(),
            profile_key
        ))
    })
}

fn resolve_target_priority(
    tx: &AdmittedWrite<'_>,
    library_asset_id: LibraryAssetId,
    capability_kind: &CapabilityKind,
    profile_key: &str,
) -> LibrarySqliteResult<WorkPriorityClass> {
    let priority = tx
        .query_row(
            "SELECT priority_class
             FROM ResolvedLibraryAssetPrepTargets
             WHERE library_asset_id = ?1
               AND capability_kind = ?2
               AND target_profile_key = ?3",
            params![
                library_asset_id.get(),
                capability_kind.as_str(),
                profile_key
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    match priority.as_deref() {
        Some(value) => WorkPriorityClass::parse(value).ok_or_else(|| {
            LibrarySqliteError::WriteInvariant(format!(
                "unknown resolved priority_class value: {value}"
            ))
        }),
        None => Ok(WorkPriorityClass::Background),
    }
}

fn load_library_asset_basis_fingerprint(
    tx: &AdmittedWrite<'_>,
    library_asset_id: LibraryAssetId,
) -> LibrarySqliteResult<Option<String>> {
    let components = tx
        .prepare(
            "SELECT attachment.source_segment_id,
                    segment_set.basis_fingerprint,
                    source_facts.basis_fingerprint
             FROM LibraryAssetAttachments attachment
             JOIN SourceSegments segment
               ON segment.source_segment_id = attachment.source_segment_id
             JOIN SourceSegmentSets segment_set
               ON segment_set.source_segment_set_id = segment.source_segment_set_id
             LEFT JOIN SourceFacts source_facts
               ON source_facts.source_file_id = segment_set.source_file_id
             WHERE attachment.library_asset_id = ?1
             ORDER BY attachment.source_segment_id ASC",
        )?
        .query_map([library_asset_id.get()], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if components.is_empty() {
        return Ok(None);
    }

    let rendered = components
        .into_iter()
        .map(|(segment_id, segment_basis, source_basis)| {
            format!(
                "{segment_id}:{segment_basis}:{}",
                source_basis.unwrap_or_else(|| "missing_source_facts".to_string())
            )
        })
        .collect::<Vec<_>>()
        .join("|");
    Ok(Some(format!(
        "library_asset_basis:v1:{}:{rendered}",
        library_asset_id.get()
    )))
}

fn stale_projection_basis(
    library_asset_id: LibraryAssetId,
    basis_fingerprint: Option<String>,
) -> String {
    match basis_fingerprint {
        Some(basis_fingerprint) => {
            format!(
                "capability_stale:v1:{}:{basis_fingerprint}",
                library_asset_id.get()
            )
        }
        None => format!(
            "capability_stale:v1:{}:missing_basis",
            library_asset_id.get()
        ),
    }
}

fn require_supported_dependency_modes(
    tx: &AdmittedWrite<'_>,
    upstream_capability_kind: &CapabilityKind,
) -> LibrarySqliteResult<()> {
    if let Some(mode) = tx
        .query_row(
            "SELECT invalidation_mode
             FROM CapabilityDependencies
             WHERE upstream_capability_kind = ?1
               AND invalidation_mode <> ?2
             LIMIT 1",
            params![
                upstream_capability_kind.as_str(),
                CapabilityInvalidationMode::MarkStale.as_str()
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()?
    {
        return Err(LibrarySqliteError::WriteInvariant(format!(
            "unsupported capability dependency invalidation_mode: {mode}"
        )));
    }
    Ok(())
}

fn load_downstream_capability_kinds(
    tx: &AdmittedWrite<'_>,
    upstream_capability_kind: &CapabilityKind,
) -> LibrarySqliteResult<Vec<CapabilityKind>> {
    let kinds = tx
        .prepare(
            "WITH RECURSIVE downstream(capability_kind) AS (
             SELECT downstream_capability_kind
             FROM CapabilityDependencies
             WHERE upstream_capability_kind = ?1
               AND invalidation_mode = ?2

             UNION

             SELECT edge.downstream_capability_kind
             FROM CapabilityDependencies edge
             JOIN downstream
               ON edge.upstream_capability_kind = downstream.capability_kind
             WHERE edge.invalidation_mode = ?2
         )
         SELECT capability_kind
         FROM downstream
         ORDER BY capability_kind ASC",
        )?
        .query_map(
            params![
                upstream_capability_kind.as_str(),
                CapabilityInvalidationMode::MarkStale.as_str()
            ],
            |row| row.get(0),
        )?
        .collect::<Result<Vec<String>, _>>()
        .map_err(LibrarySqliteError::from)?;

    kinds
        .into_iter()
        .map(|kind| parse_capability_kind(&kind))
        .collect()
}

fn parse_library_asset_id(value: i64) -> LibrarySqliteResult<LibraryAssetId> {
    LibraryAssetId::new(value).ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!(
            "invalid library_asset_id value loaded for capability invalidation: {value}"
        ))
    })
}

fn parse_capability_kind(value: &str) -> LibrarySqliteResult<CapabilityKind> {
    CapabilityKind::parse(value).ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!(
            "invalid capability_kind loaded for capability invalidation: {value:?}"
        ))
    })
}
