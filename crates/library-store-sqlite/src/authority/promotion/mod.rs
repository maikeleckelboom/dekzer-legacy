pub(crate) mod accept_segmentation;
pub(crate) mod compute_capability;
pub(crate) mod inspect_source;
pub(crate) mod rebind_source;
pub(crate) mod rebuild_projection;
pub(crate) mod resolve_library_asset;

pub use accept_segmentation::{
    AcceptSegmentationPromotionInput, AcceptSegmentationPromotionResult,
    AcceptSegmentationPromotionTx,
};
pub use compute_capability::{
    ComputeCapabilityPromotionInput, ComputeCapabilityPromotionResult, ComputeCapabilityPromotionTx,
};
pub use inspect_source::{
    InspectSourcePromotionInput, InspectSourcePromotionResult, InspectSourcePromotionTx,
};
pub use rebind_source::{
    RebindSourcePromotionInput, RebindSourcePromotionResult, RebindSourcePromotionTx,
};
pub use rebuild_projection::{
    RebuildProjectionPromotionInput, RebuildProjectionPromotionResult, RebuildProjectionPromotionTx,
};
pub use resolve_library_asset::{
    ResolveLibraryAssetPromotionInput, ResolveLibraryAssetPromotionResult,
    ResolveLibraryAssetPromotionTx,
};

#[cfg(test)]
mod tests {
    use rusqlite::{Connection, params};

    use crate::authority::library_asset::{
        ApplyLibraryAssetMetadataCorrectionInput, ReplaceLibraryAssetCapabilityInput,
    };
    use crate::authority::roots::RootMountStatus;
    use crate::authority::sources::{
        RecordSourceFileObservationInput, SourceDirectoriesAuthorityTx, SourceFilesAuthorityTx,
        SourceLocatorsAuthorityTx, SourceStateAuthorityTx, SourcesAuthorityTx,
        UpsertSourceDirectoryInput, UpsertSourceInput, UpsertSourceLocatorInput,
        UpsertSourceScanStateInput, UpsertSourceStateInput,
    };
    use crate::authority::work::{
        ArtifactFileStoreRoot, PrepAssignmentInput, PrepAssignmentsAuthorityTx,
        PrepPoliciesAuthorityTx, PrepPolicyTargetInput, ReplacePrepAssignmentsInput,
        ReplaceResolvedLibraryAssetPrepTargetsInput, ResolvedLibraryAssetPrepTargetInput,
        ResolvedTargetsAuthorityTx, UpsertPrepPolicyInput,
    };
    use crate::authority::write_lane::{AdmittedWrite, admit_write};
    use crate::schema::install_baseline_schema_for_test;
    use library_domain::{
        ArtifactId, CapabilityKind, CapabilityStabilityClass, CapabilityState,
        LibraryAssetRetentionPolicy, PrepScope, PrepTargetStabilityClass, ProjectionDomain,
        SourceAccessState, SourceFileId, SourcePresenceState, SourceScanPhase, SourceSegmentId,
        SourceSegmentSetId, WorkPriorityClass,
    };

    use super::{
        AcceptSegmentationPromotionInput, AcceptSegmentationPromotionTx,
        ComputeCapabilityPromotionInput, ComputeCapabilityPromotionTx, InspectSourcePromotionInput,
        InspectSourcePromotionTx, RebuildProjectionPromotionInput, RebuildProjectionPromotionTx,
        ResolveLibraryAssetPromotionInput, ResolveLibraryAssetPromotionTx,
    };
    use crate::authority::library_asset::LibraryAssetMetadataCorrectionsAuthorityTx;
    use crate::authority::library_asset::{
        AcceptedSourceSegmentInput, ReplaceAcceptedSourceSegmentSetInput,
    };
    use crate::authority::sources::{CommitAcceptedSourceFactsInput, ContentHashEvidence};

    #[test]
    fn authority_modules_own_canonical_write_side_rows() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline schema");
        let mut promoted_library_asset_id = None;
        let mut promoted_segmentation_artifact_id = 0;
        let file_store_root = ArtifactFileStoreRoot::for_store_path("library.sqlite3");

        admit_write(&mut connection, |write| {
            let source_id = SourcesAuthorityTx::new(write)
                .upsert_source(&UpsertSourceInput {
                    source_id: Some(100),
                    source_class: "internal".to_string(),
                    authority: "system".to_string(),
                    identity_kind: "filesystem_uuid".to_string(),
                    identity_value: "music-volume-1".to_string(),
                    display_name: "Music".to_string(),
                    medium_label: None,
                    is_user_visible: true,
                    browser_order_ordinal: Some(0),
                    changed_at: 10,
                })
                .expect("upsert source");
            SourceLocatorsAuthorityTx::new(write)
                .upsert_source_locator(&UpsertSourceLocatorInput {
                    source_id,
                    locator: crate::authority::sources::SourceLocatorInput::AbsolutePath {
                        absolute_path: "C:/music".to_string(),
                    },
                })
                .expect("upsert source locator");
            SourceStateAuthorityTx::new(write)
                .upsert_source_state(&UpsertSourceStateInput {
                    source_id,
                    mount_status: RootMountStatus::Mounted.as_str().to_string(),
                    mount_epoch: 0,
                    access_state: SourceAccessState::Accessible,
                    access_issue_kind: None,
                    access_error_detail: None,
                    access_checked_at: Some(13),
                    mount_root: Some("C:/music".to_string()),
                    effective_path: Some("C:/music".to_string()),
                    observed_volume_label: Some("USB".to_string()),
                    filesystem_type: Some("exfat".to_string()),
                    last_seen_at: Some(11),
                    updated_at: 13,
                })
                .expect("upsert source state");
            SourceStateAuthorityTx::new(write)
                .upsert_source_scan_state(&UpsertSourceScanStateInput {
                    source_id,
                    scan_phase: SourceScanPhase::Idle,
                    last_scan_started_at: Some(12),
                    last_scan_finished_at: Some(13),
                    last_successful_scan_at: Some(13),
                    scan_issue_kind: None,
                    error_detail: None,
                    updated_at: 13,
                })
                .expect("upsert source state");
            let source_directory_id = SourceDirectoriesAuthorityTx::new(write)
                .upsert_source_directory(&UpsertSourceDirectoryInput {
                    source_directory_id: Some(200),
                    source_id,
                    parent_source_directory_id: None,
                    name: "album".to_string(),
                    relative_path: "album".to_string(),
                    presence_state: SourcePresenceState::Present,
                    dir_scan_state: None,
                    dir_scan_issue_kind: None,
                    dir_scan_error_detail: None,
                    scanned_at: None,
                    mtime_ns: None,
                    first_created_at: Some(14),
                    changed_at: 14,
                })
                .expect("upsert source directory");
            let source_file_id = SourceFilesAuthorityTx::new(write)
                .record_source_file_observation(&RecordSourceFileObservationInput {
                    source_file_id: Some(300),
                    source_id,
                    parent_source_directory_id: Some(source_directory_id),
                    name: "track.flac".to_string(),
                    relative_path: "album/track.flac".to_string(),
                    size_bytes: Some(12_345),
                    mtime_ns: Some(555),
                    presence_state: SourcePresenceState::Present,
                    first_discovered_at: Some(15),
                    observed_at: Some(15),
                    presence_changed_at: 15,
                    updated_at: 15,
                })
                .expect("record source file");

            let inspection_artifact_id = insert_artifact(
                write,
                TestArtifactInput {
                    subject_kind: "source_file",
                    subject_id: source_file_id.to_string(),
                    work_kind: "inspect_source",
                    artifact_kind: "inspection_result",
                    capability_kind: None,
                    profile_key: None,
                    basis_fingerprint: "basis:source:v1",
                    created_at: 20,
                },
            )
            .expect("insert inspection artifact");
            InspectSourcePromotionTx::new(write, file_store_root.clone())
                .inspect_source(&InspectSourcePromotionInput {
                    source_facts: CommitAcceptedSourceFactsInput {
                        source_file_id: source_file_domain_id(source_file_id),
                        accepted_artifact_id: artifact_domain_id(inspection_artifact_id),
                        basis_fingerprint: "basis:source:v1".to_string(),
                        observed_at_ms: 21,
                        content_hash: Some(ContentHashEvidence {
                            algorithm: "sha256".to_string(),
                            value: "source-a".to_string(),
                        }),
                        media_kind: "audio".to_string(),
                        mime_type: Some("audio/flac".to_string()),
                        duration_ms: Some(180_000),
                        sample_rate_hz: Some(44_100),
                        channels: Some(2),
                        bit_depth: Some(16),
                        codec: Some("flac".to_string()),
                        updated_at: 21,
                    },
                    rebuild_projection_domains: vec![ProjectionDomain::LibraryBrowser],
                    rebuild_priority: WorkPriorityClass::Interactive,
                })
                .expect("promote inspect_source");

            let segmentation_artifact_id = insert_artifact(
                write,
                TestArtifactInput {
                    subject_kind: "source_file",
                    subject_id: source_file_id.to_string(),
                    work_kind: "accept_segmentation",
                    artifact_kind: "segmentation_result",
                    capability_kind: None,
                    profile_key: None,
                    basis_fingerprint: "basis:segments:v1",
                    created_at: 30,
                },
            )
            .expect("insert segmentation artifact");
            promoted_segmentation_artifact_id = segmentation_artifact_id;
            let segmentation = AcceptSegmentationPromotionTx::new(write, file_store_root.clone())
                .accept_segmentation(&AcceptSegmentationPromotionInput {
                    segment_set: ReplaceAcceptedSourceSegmentSetInput {
                        source_segment_set_id: Some(source_segment_set_domain_id(400)),
                        source_file_id: source_file_domain_id(source_file_id),
                        segment_set_kind: "accepted_primary".to_string(),
                        basis_fingerprint: "basis:segments:v1".to_string(),
                        accepted_artifact_id: artifact_domain_id(segmentation_artifact_id),
                        accepted_at: 31,
                        updated_at: 31,
                        segments: vec![
                            AcceptedSourceSegmentInput {
                                source_segment_id: Some(source_segment_domain_id(500)),
                                segment_kind: "track_span".to_string(),
                                ordinal: 0,
                                start_offset_ms: 0,
                                end_offset_ms: Some(90_000),
                                display_title: Some("Intro".to_string()),
                                display_artist: Some("Artist".to_string()),
                                display_album: Some("Album".to_string()),
                            },
                            AcceptedSourceSegmentInput {
                                source_segment_id: Some(source_segment_domain_id(501)),
                                segment_kind: "track_span".to_string(),
                                ordinal: 1,
                                start_offset_ms: 90_000,
                                end_offset_ms: Some(180_000),
                                display_title: Some("Main".to_string()),
                                display_artist: Some("Artist".to_string()),
                                display_album: Some("Album".to_string()),
                            },
                        ],
                    },
                    rebuild_projection_domains: vec![ProjectionDomain::LibraryBrowser],
                    rebuild_priority: WorkPriorityClass::Interactive,
                })
                .expect("promote accept_segmentation");
            assert_eq!(
                segmentation.source_segment_set_id,
                source_segment_set_domain_id(400)
            );

            let resolved = ResolveLibraryAssetPromotionTx::new(write)
                .resolve_library_asset(&ResolveLibraryAssetPromotionInput {
                    equivalence_fingerprint: "eq:track-a".to_string(),
                    retention_policy: LibraryAssetRetentionPolicy::KeepMetadata,
                    source_segment_ids: vec![
                        source_segment_domain_id(500),
                        source_segment_domain_id(501),
                    ],
                    accepted_at: 40,
                    updated_at: 40,
                    rebuild_projection_domains: vec![
                        ProjectionDomain::LibraryBrowser,
                        ProjectionDomain::Navigation,
                    ],
                    rebuild_priority: WorkPriorityClass::Interactive,
                })
                .expect("promote resolve_library_asset initial");
            let library_asset_id = resolved.library_asset.library_asset_id;
            assert!(resolved.library_asset.created);
            promoted_library_asset_id = Some(library_asset_id);

            let reused = ResolveLibraryAssetPromotionTx::new(write)
                .resolve_library_asset(&ResolveLibraryAssetPromotionInput {
                    equivalence_fingerprint: "eq:track-a".to_string(),
                    retention_policy: LibraryAssetRetentionPolicy::KeepMetadata,
                    source_segment_ids: vec![source_segment_domain_id(501)],
                    accepted_at: 41,
                    updated_at: 41,
                    rebuild_projection_domains: vec![ProjectionDomain::LibraryBrowser],
                    rebuild_priority: WorkPriorityClass::Interactive,
                })
                .expect("promote resolve_library_asset reuse");
            assert!(!reused.library_asset.created);
            assert_eq!(reused.library_asset.library_asset_id, library_asset_id);

            let capability_artifact_id = insert_artifact(
                write,
                TestArtifactInput {
                    subject_kind: "library_asset",
                    subject_id: library_asset_id.get().to_string(),
                    work_kind: "compute_capability",
                    artifact_kind: "capability_result",
                    capability_kind: Some("waveform"),
                    profile_key: Some("default"),
                    basis_fingerprint: "basis:capability:v1",
                    created_at: 50,
                },
            )
            .expect("insert capability artifact");
            ComputeCapabilityPromotionTx::new(write, file_store_root.clone())
                .compute_capability(&ComputeCapabilityPromotionInput {
                    capability: ReplaceLibraryAssetCapabilityInput {
                        library_asset_id,
                        capability_kind: CapabilityKind::waveform(),
                        profile_key: "default".to_string(),
                        state: CapabilityState::Ready,
                        stability_class: Some(CapabilityStabilityClass::Stable),
                        quality_current: Some(90),
                        basis_fingerprint: Some("basis:capability:v1".to_string()),
                        selected_artifact_id: Some(
                            ArtifactId::new(capability_artifact_id).expect("inserted artifact id"),
                        ),
                        updated_at: 51,
                    },
                    rebuild_projection_domains: vec![ProjectionDomain::LibraryBrowser],
                    rebuild_priority: WorkPriorityClass::Urgent,
                })
                .expect("promote compute_capability");

            let correction_id = LibraryAssetMetadataCorrectionsAuthorityTx::new(write)
                .apply_metadata_correction(&ApplyLibraryAssetMetadataCorrectionInput {
                    library_asset_id,
                    field_name: "title".to_string(),
                    value_text: Some("Corrected Title".to_string()),
                    value_int: None,
                    is_null_correction: false,
                    source_kind: "user_edit".to_string(),
                    applied_at: 60,
                })?;
            assert!(correction_id > 0);

            let prep_policy_id = PrepPoliciesAuthorityTx::new(write)
                .upsert_prep_policy(&UpsertPrepPolicyInput {
                    prep_policy_id: Some(library_domain::PrepPolicyId::new(600).unwrap()),
                    policy_name: "dj-default".to_string(),
                    is_system_policy: false,
                    is_user_editable: true,
                    changed_at: 70,
                    targets: vec![PrepPolicyTargetInput {
                        capability_kind: CapabilityKind::waveform(),
                        target_profile_key: "default".to_string(),
                        target_quality: 95,
                        target_stability_class: PrepTargetStabilityClass::Stable,
                        priority_class: WorkPriorityClass::Interactive,
                    }],
                })
                .expect("upsert prep policy");
            PrepAssignmentsAuthorityTx::new(write)
                .replace_prep_assignments(&ReplacePrepAssignmentsInput {
                    scope: PrepScope::LibraryAsset(library_asset_id),
                    assignments: vec![PrepAssignmentInput {
                        prep_assignment_id: Some(700),
                        prep_policy_id: prep_policy_id.get(),
                        precedence_rank: 0,
                    }],
                    changed_at: 71,
                })
                .expect("replace prep assignments");
            ResolvedTargetsAuthorityTx::new(write)
                .replace_resolved_library_asset_prep_targets(
                    &ReplaceResolvedLibraryAssetPrepTargetsInput {
                        library_asset_id,
                        targets: vec![ResolvedLibraryAssetPrepTargetInput {
                            capability_kind: CapabilityKind::waveform(),
                            target_profile_key: "default".to_string(),
                            target_quality: 95,
                            target_stability_class: PrepTargetStabilityClass::Stable,
                            priority_class: WorkPriorityClass::Interactive,
                            resolved_from_policy_id: prep_policy_id,
                        }],
                        updated_at: 72,
                    },
                )
                .expect("replace resolved targets");

            RebuildProjectionPromotionTx::new(write)
                .rebuild_projection(&RebuildProjectionPromotionInput {
                    projection_domain: ProjectionDomain::Navigation,
                    basis_fingerprint: "basis:projection:nav:v1".to_string(),
                    priority_class: WorkPriorityClass::Background,
                    queued_at: 80,
                })
                .expect("enqueue rebuild_projection");

            Ok(())
        })
        .expect("promote canonical write side");
        let library_asset_id =
            promoted_library_asset_id.expect("resolve_library_asset should mint a library asset");

        let source_facts = connection
            .query_row(
                "SELECT content_hash_algorithm, content_hash_value, media_kind
                 FROM SourceFacts
                 WHERE source_file_id = 300",
                [],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .expect("read source facts");
        assert_eq!(
            source_facts,
            (
                Some("sha256".to_string()),
                Some("source-a".to_string()),
                "audio".to_string()
            )
        );

        let segment_set_row = connection
            .query_row(
                "SELECT source_file_id, segment_set_kind, accepted_artifact_id
                 FROM SourceSegmentSets
                 WHERE source_segment_set_id = 400",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .expect("read segment set row");
        assert_eq!(
            segment_set_row,
            (
                300,
                "accepted_primary".to_string(),
                promoted_segmentation_artifact_id
            )
        );

        let attachment_rows = connection
            .prepare(
                "SELECT source_segment_id
                 FROM LibraryAssetAttachments
                 WHERE library_asset_id = ?1
                 ORDER BY source_segment_id",
            )
            .expect("prepare attachment query")
            .query_map([library_asset_id.get()], |row| row.get::<_, i64>(0))
            .expect("query attachment rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect attachment rows");
        assert_eq!(attachment_rows, vec![501]);

        let capability_row = connection
            .query_row(
                "SELECT state, stability_class, quality_current
                 FROM LibraryAssetCapabilities
                 WHERE library_asset_id = ?1
                   AND capability_kind = 'waveform'
                   AND profile_key = 'default'",
                [library_asset_id.get()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                    ))
                },
            )
            .expect("read capability row");
        assert_eq!(
            capability_row,
            ("ready".to_string(), Some("stable".to_string()), Some(90))
        );

        let active_correction_count: i64 = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM LibraryAssetMetadataCorrections
                 WHERE library_asset_id = ?1
                   AND retracted_at IS NULL",
                [library_asset_id.get()],
                |row| row.get(0),
            )
            .expect("count active corrections");
        assert_eq!(active_correction_count, 1);

        let resolved_target_count: i64 = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM ResolvedLibraryAssetPrepTargets
                 WHERE library_asset_id = ?1",
                [library_asset_id.get()],
                |row| row.get(0),
            )
            .expect("count resolved targets");
        assert_eq!(resolved_target_count, 1);

        let projection_work_count: i64 = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM WorkItems
                 WHERE subject_kind = 'projection_domain'
                   AND work_kind = 'rebuild_projection'",
                [],
                |row| row.get(0),
            )
            .expect("count projection work items");
        assert!(projection_work_count >= 4);
    }

    struct TestArtifactInput<'a> {
        subject_kind: &'a str,
        subject_id: String,
        work_kind: &'a str,
        artifact_kind: &'a str,
        capability_kind: Option<&'a str>,
        profile_key: Option<&'a str>,
        basis_fingerprint: &'a str,
        created_at: i64,
    }

    fn insert_artifact(
        write: &AdmittedWrite<'_>,
        input: TestArtifactInput<'_>,
    ) -> crate::LibrarySqliteResult<i64> {
        write.execute(
            "INSERT INTO WorkItems (
                 subject_kind,
                 subject_id,
                 work_kind,
                 capability_kind,
                 target_profile_key,
                 target_quality,
                 priority_class,
                 basis_fingerprint,
                 state,
                 leased_until,
                 attempt_count,
                 blocked_reason,
                 failure_kind,
                 error_detail,
                 created_at,
                 updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'interactive', ?7, 'completed', NULL, 1, NULL, NULL, NULL, ?8, ?8)",
            params![
                input.subject_kind,
                input.subject_id.as_str(),
                input.work_kind,
                input.capability_kind,
                input.profile_key,
                input.capability_kind.map(|_| 95i64),
                input.basis_fingerprint,
                input.created_at,
            ],
        )?;
        let work_item_id = write.last_insert_rowid();

        write.execute(
            "INSERT INTO WorkRuns (
                 work_item_id,
                 adapter_key,
                 adapter_version,
                 started_at,
                 finished_at,
                 outcome,
                 failure_kind,
                 error_detail
             )
             VALUES (?1, 'test.adapter', '1.0.0', ?2, ?2, 'completed', NULL, NULL)",
            params![work_item_id, input.created_at],
        )?;
        let work_run_id = write.last_insert_rowid();

        write.execute(
            "INSERT INTO Artifacts (
                 work_run_id,
                 subject_kind,
                 subject_id,
                 capability_kind,
                 profile_key,
                 artifact_kind,
                 artifact_role,
                 adapter_key,
                 adapter_version,
                 basis_fingerprint,
                 media_type,
                 storage_kind,
                 payload_hash,
                 created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'primary_result', 'test.adapter', '1.0.0', ?7, 'application/json', 'inline_payload', ?8, ?9)",
            params![
                work_run_id,
                input.subject_kind,
                input.subject_id.as_str(),
                input.capability_kind,
                input.profile_key,
                input.artifact_kind,
                input.basis_fingerprint,
                format!(
                    "hash:{}:{}:{}",
                    input.artifact_kind,
                    input.subject_id.as_str(),
                    input.basis_fingerprint
                ),
                input.created_at,
            ],
        )?;
        Ok(write.last_insert_rowid())
    }

    fn source_file_domain_id(value: i64) -> SourceFileId {
        SourceFileId::new(value).expect("positive source file id")
    }

    fn source_segment_set_domain_id(value: i64) -> SourceSegmentSetId {
        SourceSegmentSetId::new(value).expect("positive source segment set id")
    }

    fn source_segment_domain_id(value: i64) -> SourceSegmentId {
        SourceSegmentId::new(value).expect("positive source segment id")
    }

    fn artifact_domain_id(value: i64) -> ArtifactId {
        ArtifactId::new(value).expect("positive artifact id")
    }
}
