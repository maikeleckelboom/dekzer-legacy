pub(crate) mod artifacts;
pub(crate) mod capability_invalidation;
pub(crate) mod file_store;
pub(crate) mod prep_assignments;
pub(crate) mod prep_policies;
pub(crate) mod rebind_source;
pub(crate) mod resolved_targets;
pub(crate) mod work_items;
pub(crate) mod work_runs;

pub use artifacts::{
    ArtifactsAuthorityTx, RecordArtifactInput, RecordFileStoreArtifactInput,
    RecordInlineArtifactInput, RecordedArtifact,
};
pub(crate) use capability_invalidation::load_attached_library_asset_ids_for_source_file;
pub use capability_invalidation::{
    CapabilityInvalidationAuthorityTx, MarkCapabilitiesStaleFromBasisInput,
    MarkCapabilitiesStaleResult, MarkCapabilityStaleFromDependencyInput, StaleCapabilityChange,
};
pub use file_store::{ArtifactFileStoreReconciliationResult, ArtifactFileStoreRoot};
pub(crate) use file_store::{
    reconcile_artifact_file_store, retire_artifact_if_unreferenced_and_unclaimed,
};
pub use prep_assignments::{
    PrepAssignmentInput, PrepAssignmentsAuthorityTx, ReplacePrepAssignmentsInput,
};
pub use prep_policies::{PrepPoliciesAuthorityTx, PrepPolicyTargetInput, UpsertPrepPolicyInput};
pub use rebind_source::{QueueRebindSourceWorkInput, RebindSourceWorkAuthorityTx};
pub use resolved_targets::{
    ReplaceResolvedLibraryAssetPrepTargetsInput, ResolvedLibraryAssetPrepTargetInput,
    ResolvedTargetsAuthorityTx,
};
pub use work_items::{
    BlockMachineWorkInput, ClaimMachineWorkBatchInput, ClaimedMachineWorkItem,
    CompleteMachineWorkInput, FailMachineWorkInput, MachineWorkKey,
    QueueAcceptSegmentationWorkInput, QueueComputeCapabilityWorkInput, QueueInspectSourceWorkInput,
    QueueMachineWorkInput, QueueMachineWorkResult, QueueRebuildProjectionWorkInput,
    WorkItemsAuthorityTx,
};
pub use work_runs::{FinishWorkRunInput, StartWorkRunInput, StartedWorkRun, WorkRunsAuthorityTx};

#[cfg(test)]
mod tests {
    use rusqlite::{Connection, params};

    use crate::LibrarySqliteError;
    use crate::authority::library_asset::{
        AcceptedSourceSegmentInput, ReplaceAcceptedSourceSegmentSetInput,
        ReplaceLibraryAssetCapabilityInput,
    };
    use crate::authority::promotion::{
        AcceptSegmentationPromotionInput, AcceptSegmentationPromotionTx,
        ComputeCapabilityPromotionInput, ComputeCapabilityPromotionTx, InspectSourcePromotionInput,
        InspectSourcePromotionTx, RebindSourcePromotionInput, RebindSourcePromotionTx,
        ResolveLibraryAssetPromotionInput, ResolveLibraryAssetPromotionTx,
    };
    use crate::authority::roots::RootMountStatus;
    use crate::authority::sources::{
        CommitAcceptedSourceFactsInput, ContentHashEvidence, RecordSourceFileObservationInput,
        SourceDirectoriesAuthorityTx, SourceFilesAuthorityTx, SourceLocatorsAuthorityTx,
        SourceStateAuthorityTx, SourcesAuthorityTx, UpsertSourceDirectoryInput, UpsertSourceInput,
        UpsertSourceLocatorInput, UpsertSourceScanStateInput, UpsertSourceStateInput,
    };
    use crate::authority::write_lane::{AdmittedWrite, admit_write};
    use crate::schema::install_baseline_schema_for_test;
    use library_domain::{
        ArtifactKind, ArtifactRole, CapabilityKind, CapabilityStabilityClass, CapabilityState,
        LibraryAssetId, LibraryAssetRetentionPolicy, SourceAccessState, SourceFileId,
        SourcePresenceState, SourceScanPhase, SourceSegmentId, SourceSegmentSetId, WorkItemId,
        WorkItemState, WorkPriorityClass, WorkRunOutcome, WorkSubject,
    };

    use super::{
        ArtifactFileStoreRoot, ArtifactsAuthorityTx, BlockMachineWorkInput,
        CapabilityInvalidationAuthorityTx, ClaimMachineWorkBatchInput, CompleteMachineWorkInput,
        FailMachineWorkInput, FinishWorkRunInput, MarkCapabilityStaleFromDependencyInput,
        QueueAcceptSegmentationWorkInput, QueueComputeCapabilityWorkInput,
        QueueInspectSourceWorkInput, QueueRebindSourceWorkInput, RebindSourceWorkAuthorityTx,
        RecordArtifactInput, RecordInlineArtifactInput, StartWorkRunInput, StartedWorkRun,
        WorkItemsAuthorityTx, WorkRunsAuthorityTx,
    };

    #[derive(Clone, Copy)]
    struct SourceFixture {
        source_id: i64,
        source_directory_id: i64,
        source_file_id: i64,
    }

    #[test]
    fn compute_capability_work_boundary_preserves_typed_ids_and_capability_kind() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline schema");

        admit_write(&mut connection, |write| {
            let library_asset_id = library_asset_domain_id(42);
            let queued = WorkItemsAuthorityTx::new(write).queue_compute_capability_work(
                &QueueComputeCapabilityWorkInput {
                    library_asset_id,
                    capability_kind: CapabilityKind::waveform(),
                    target_profile_key: "default".to_string(),
                    target_quality: 90,
                    basis_fingerprint: "basis:typed-work:v1".to_string(),
                    priority_class: WorkPriorityClass::Interactive,
                    queued_at: 10,
                },
            )?;
            let claimed = claim_one_work(write, 11)?;

            assert!(queued.work_item_id.get() > 0);
            assert_eq!(claimed.work_item_id, queued.work_item_id);
            assert_eq!(claimed.subject, WorkSubject::LibraryAsset(library_asset_id));
            assert_eq!(claimed.capability_kind, Some(CapabilityKind::waveform()));
            Ok(())
        })
        .expect("preserve typed work boundary");
    }

    #[test]
    fn machine_work_path_records_runs_artifacts_and_completed_state_for_inspection() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline schema");
        let mut completed_work_item_id = 0;
        let mut accepted_artifact_id = 0;

        admit_write(&mut connection, |write| {
            seed_source_fixture(
                write,
                "C:/music-a",
                SourceFixture {
                    source_id: 100,
                    source_directory_id: 200,
                    source_file_id: 300,
                },
                "album/track-a.flac",
                10,
            )?;

            let first_queue = WorkItemsAuthorityTx::new(write).queue_inspect_source_work(
                &QueueInspectSourceWorkInput {
                    source_file_id: source_file_domain_id(300),
                    basis_fingerprint: "basis:source:v1".to_string(),
                    priority_class: WorkPriorityClass::Interactive,
                    queued_at: 20,
                },
            )?;
            assert!(first_queue.created);
            let repeated_queue = WorkItemsAuthorityTx::new(write).queue_inspect_source_work(
                &QueueInspectSourceWorkInput {
                    source_file_id: source_file_domain_id(300),
                    basis_fingerprint: "basis:source:v1".to_string(),
                    priority_class: WorkPriorityClass::Interactive,
                    queued_at: 21,
                },
            )?;
            assert!(!repeated_queue.created);
            assert_eq!(first_queue.work_item_id, repeated_queue.work_item_id);

            let claimed = claim_one_work(write, 22)?;
            let artifact = record_primary_inline_artifact(
                write,
                claimed.work_item_id,
                ArtifactKind::InspectionResult,
                "basis:source:v1",
                23,
                "hash:inspect:300:basis:source:v1",
            )?;
            InspectSourcePromotionTx::new(
                write,
                ArtifactFileStoreRoot::for_store_path("library.sqlite3"),
            )
            .inspect_source(&InspectSourcePromotionInput {
                source_facts: CommitAcceptedSourceFactsInput {
                    source_file_id: source_file_domain_id(300),
                    accepted_artifact_id: artifact.artifact_id,
                    basis_fingerprint: "basis:source:v1".to_string(),
                    observed_at_ms: 24,
                    content_hash: Some(ContentHashEvidence {
                        algorithm: "sha256".to_string(),
                        value: "track-a".to_string(),
                    }),
                    media_kind: "audio".to_string(),
                    mime_type: Some("audio/flac".to_string()),
                    duration_ms: Some(180_000),
                    sample_rate_hz: Some(44_100),
                    channels: Some(2),
                    bit_depth: Some(16),
                    codec: Some("flac".to_string()),
                    updated_at: 24,
                },
                rebuild_projection_domains: vec![],
                rebuild_priority: WorkPriorityClass::Interactive,
            })?;
            WorkItemsAuthorityTx::new(write).complete_machine_work_item(
                &CompleteMachineWorkInput {
                    work_item_id: claimed.work_item_id,
                    completed_at: 25,
                },
            )?;
            completed_work_item_id = claimed.work_item_id.get();
            accepted_artifact_id = artifact.artifact_id.get();
            Ok(())
        })
        .expect("complete inspection work path");

        let state: String = connection
            .query_row(
                "SELECT state
                 FROM WorkItems
                 WHERE work_item_id = ?1",
                [completed_work_item_id],
                |row| row.get(0),
            )
            .expect("read work item state");
        assert_eq!(state, WorkItemState::Completed.as_str());

        let completed_run_count: i64 = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM WorkRuns
                 WHERE work_item_id = ?1
                   AND outcome = ?2",
                params![completed_work_item_id, WorkRunOutcome::Completed.as_str()],
                |row| row.get(0),
            )
            .expect("count completed work runs");
        assert_eq!(completed_run_count, 1);

        let persisted_artifact_id: i64 = connection
            .query_row(
                "SELECT accepted_artifact_id
                 FROM SourceFacts
                 WHERE source_file_id = 300",
                [],
                |row| row.get(0),
            )
            .expect("read source facts artifact");
        assert_eq!(persisted_artifact_id, accepted_artifact_id);
    }

    #[test]
    fn starting_second_active_run_for_same_leased_work_item_is_rejected() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline schema");

        admit_write(&mut connection, |write| {
            seed_source_fixture(
                write,
                "C:/music-start-run",
                SourceFixture {
                    source_id: 130,
                    source_directory_id: 230,
                    source_file_id: 330,
                },
                "album/track-run-guard.flac",
                10,
            )?;

            let claimed =
                queue_and_claim_inspection_work(write, 330, "basis:source:run-guard:v1", 20)?;
            let first_run = start_test_work_run(write, claimed.work_item_id, 22)?;

            let error = start_test_work_run(write, claimed.work_item_id, 23)
                .expect_err("reject a second unfinished run");
            assert_write_invariant_contains(error, "already has unfinished work run");

            let open_run_count: i64 = write.query_row(
                "SELECT COUNT(*)
                 FROM WorkRuns
                 WHERE work_item_id = ?1
                   AND finished_at IS NULL",
                [claimed.work_item_id.get()],
                |row| row.get(0),
            )?;
            assert_eq!(open_run_count, 1);
            assert_eq!(first_run.work_item_id, claimed.work_item_id);

            Ok(())
        })
        .expect("reject second active run");
    }

    #[test]
    fn completing_machine_work_item_with_open_run_is_rejected() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline schema");

        admit_write(&mut connection, |write| {
            seed_source_fixture(
                write,
                "C:/music-open-complete",
                SourceFixture {
                    source_id: 131,
                    source_directory_id: 231,
                    source_file_id: 331,
                },
                "album/track-open-complete.flac",
                10,
            )?;

            let claimed =
                queue_and_claim_inspection_work(write, 331, "basis:source:open-complete:v1", 20)?;
            let work_run = start_test_work_run(write, claimed.work_item_id, 22)?;

            let error = WorkItemsAuthorityTx::new(write)
                .complete_machine_work_item(&CompleteMachineWorkInput {
                    work_item_id: claimed.work_item_id,
                    completed_at: 23,
                })
                .expect_err("reject completion while a run is open");
            assert_write_invariant_contains(error, "still open");

            let work_item =
                WorkItemsAuthorityTx::new(write).load_work_item(claimed.work_item_id)?;
            assert_eq!(work_item.state, WorkItemState::Leased);

            let artifact = ArtifactsAuthorityTx::new(write).record_inline_artifact(
                &RecordInlineArtifactInput {
                    artifact: RecordArtifactInput {
                        work_run_id: work_run.work_run_id,
                        artifact_kind: ArtifactKind::InspectionResult,
                        artifact_role: ArtifactRole::PrimaryResult,
                        media_type: "application/json".to_string(),
                        basis_fingerprint: "basis:source:open-complete:v1".to_string(),
                        payload_hash: "hash:inspect:331:basis:source:open-complete:v1".to_string(),
                        created_at: 24,
                    },
                    payload: b"{}".to_vec(),
                },
            )?;
            assert_eq!(artifact.work_run_id, work_run.work_run_id);

            Ok(())
        })
        .expect("reject completion with open run");
    }

    #[test]
    fn completing_machine_work_item_without_successful_run_is_rejected() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline schema");

        admit_write(&mut connection, |write| {
            seed_source_fixture(
                write,
                "C:/music-failed-complete",
                SourceFixture {
                    source_id: 132,
                    source_directory_id: 232,
                    source_file_id: 332,
                },
                "album/track-failed-complete.flac",
                10,
            )?;

            let claimed =
                queue_and_claim_inspection_work(write, 332, "basis:source:failed-complete:v1", 20)?;
            let work_run = start_test_work_run(write, claimed.work_item_id, 22)?;
            WorkRunsAuthorityTx::new(write).finish_work_run(&FinishWorkRunInput {
                work_run_id: work_run.work_run_id,
                finished_at: 23,
                outcome: WorkRunOutcome::Failed,
                failure_kind: Some("adapter_failure".to_string()),
                error_detail: Some("adapter returned a failed result".to_string()),
            })?;

            let error = WorkItemsAuthorityTx::new(write)
                .complete_machine_work_item(&CompleteMachineWorkInput {
                    work_item_id: claimed.work_item_id,
                    completed_at: 24,
                })
                .expect_err("reject completion without a successful run");
            assert_write_invariant_contains(error, "latest work run");

            let work_item =
                WorkItemsAuthorityTx::new(write).load_work_item(claimed.work_item_id)?;
            assert_eq!(work_item.state, WorkItemState::Leased);

            Ok(())
        })
        .expect("reject completion without successful run");
    }

    #[test]
    fn failing_or_blocking_machine_work_item_with_open_run_is_rejected() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline schema");

        admit_write(&mut connection, |write| {
            seed_source_fixture(
                write,
                "C:/music-open-terminal",
                SourceFixture {
                    source_id: 133,
                    source_directory_id: 233,
                    source_file_id: 333,
                },
                "album/track-open-terminal.flac",
                10,
            )?;

            let claimed =
                queue_and_claim_inspection_work(write, 333, "basis:source:open-terminal:v1", 20)?;
            let work_run = start_test_work_run(write, claimed.work_item_id, 22)?;

            let fail_error = WorkItemsAuthorityTx::new(write)
                .fail_machine_work_item(&FailMachineWorkInput {
                    work_item_id: claimed.work_item_id,
                    failed_at: 23,
                    failure_kind: "adapter_failure".to_string(),
                    error_detail: Some("adapter failed before promotion".to_string()),
                })
                .expect_err("reject failure while a run is open");
            assert_write_invariant_contains(fail_error, "still open");

            let block_error = WorkItemsAuthorityTx::new(write)
                .block_machine_work_item(&BlockMachineWorkInput {
                    work_item_id: claimed.work_item_id,
                    blocked_at: 24,
                    blocked_reason: "waiting_for_precondition".to_string(),
                    error_detail: Some("source facts are still unavailable".to_string()),
                })
                .expect_err("reject blocking while a run is open");
            assert_write_invariant_contains(block_error, "still open");

            let work_item =
                WorkItemsAuthorityTx::new(write).load_work_item(claimed.work_item_id)?;
            assert_eq!(work_item.state, WorkItemState::Leased);
            assert_eq!(work_run.work_item_id, claimed.work_item_id);

            Ok(())
        })
        .expect("reject fail and block with open run");
    }

    #[test]
    fn dependency_invalidation_marks_downstream_capability_stale_and_queues_recompute() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline schema");
        let mut library_asset_id = 0;

        admit_write(&mut connection, |write| {
            seed_source_fixture(
                write,
                "C:/music-b",
                SourceFixture {
                    source_id: 110,
                    source_directory_id: 210,
                    source_file_id: 310,
                },
                "album/track-b.flac",
                10,
            )?;
            queue_and_promote_inspection(write, 310, "basis:source:b1", 20)?;
            queue_and_promote_segmentation(write, 310, 410, 510, "basis:segments:b1", 30)?;

            library_asset_id = ResolveLibraryAssetPromotionTx::new(write)
                .resolve_library_asset(&ResolveLibraryAssetPromotionInput {
                    equivalence_fingerprint: "eq:track-b".to_string(),
                    retention_policy: LibraryAssetRetentionPolicy::KeepMetadata,
                    source_segment_ids: vec![source_segment_domain_id(510)],
                    accepted_at: 40,
                    updated_at: 40,
                    rebuild_projection_domains: vec![],
                    rebuild_priority: WorkPriorityClass::Interactive,
                })?
                .library_asset
                .library_asset_id
                .get();

            queue_and_promote_capability(
                write,
                library_asset_id,
                "beatgrid",
                90,
                90,
                "basis:beatgrid:v1",
                50,
            )?;
            queue_and_promote_capability(
                write,
                library_asset_id,
                "tempo",
                70,
                70,
                "basis:tempo:v1",
                60,
            )?;

            write.execute(
                "INSERT INTO CapabilityDependencies (
                     upstream_capability_kind,
                     downstream_capability_kind,
                     invalidation_mode,
                     created_at,
                     updated_at
                 )
                 VALUES ('beatgrid', 'tempo', 'mark_stale', 70, 70)",
                [],
            )?;

            queue_and_promote_capability(
                write,
                library_asset_id,
                "beatgrid",
                95,
                95,
                "basis:beatgrid:v2",
                80,
            )?;
            Ok(())
        })
        .expect("complete dependency invalidation flow");

        let tempo_state: String = connection
            .query_row(
                "SELECT state
                 FROM LibraryAssetCapabilities
                 WHERE library_asset_id = ?1
                   AND capability_kind = 'tempo'
                   AND profile_key = 'default'",
                [library_asset_id],
                |row| row.get(0),
            )
            .expect("read tempo state");
        assert_eq!(tempo_state, CapabilityState::Stale.as_str());

        let queued_tempo_work_count: i64 = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM WorkItems
                 WHERE subject_kind = 'library_asset'
                   AND subject_id = ?1
                   AND work_kind = 'compute_capability'
                   AND capability_kind = 'tempo'
                   AND target_profile_key = 'default'
                   AND basis_fingerprint = 'basis:beatgrid:v2'
                   AND state = 'queued'",
                [library_asset_id.to_string()],
                |row| row.get(0),
            )
            .expect("count queued tempo work");
        assert_eq!(queued_tempo_work_count, 1);
    }

    #[test]
    fn claim_machine_work_rejects_invalid_persisted_capability_kind() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline schema");

        connection
            .pragma_update(None, "foreign_keys", false)
            .expect("disable foreign keys to simulate malformed persisted state");
        connection
            .execute(
                "INSERT INTO LibraryAssets (
                 library_asset_id,
                 equivalence_fingerprint,
                 retention_policy,
                 created_at,
                 updated_at
             )
             VALUES (1, 'eq:invalid-persisted-capability-kind', 'keep_metadata', 1, 1)",
                [],
            )
            .expect("insert library asset");
        connection
            .execute(
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
             VALUES ('library_asset', '1', 'compute_capability', 'Waveform', 'default', 90,
                     'interactive', 'basis:invalid-capability-kind', 'queued', NULL, 0,
                     NULL, NULL, NULL, 10, 10)",
                [],
            )
            .expect("insert malformed work item");
        connection
            .pragma_update(None, "foreign_keys", true)
            .expect("reenable foreign keys");

        admit_write(&mut connection, |write| {
            let error = WorkItemsAuthorityTx::new(write)
                .claim_machine_work_batch(&ClaimMachineWorkBatchInput {
                    limit: 1,
                    lease_duration_ms: 30_000,
                    claimed_at: 11,
                })
                .expect_err("invalid persisted capability kind should be rejected");

            assert_write_invariant_contains(error, "invalid WorkItems.capability_kind");
            Ok(())
        })
        .expect("reject invalid persisted compute capability work");
    }

    #[test]
    fn dependency_invalidation_rejects_empty_upstream_basis_fingerprint() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline schema");

        admit_write(&mut connection, |write| {
            let error = CapabilityInvalidationAuthorityTx::new(write)
                .mark_capability_stale_from_dependency(&MarkCapabilityStaleFromDependencyInput {
                    library_asset_id: library_asset_domain_id(1),
                    upstream_capability_kind: CapabilityKind::waveform(),
                    upstream_basis_fingerprint: " ".to_string(),
                    invalidated_at: 10,
                })
                .expect_err("empty upstream basis should be rejected");

            assert_write_invariant_contains(error, "upstream_basis_fingerprint");
            Ok(())
        })
        .expect("reject empty dependency invalidation basis");
    }

    #[test]
    fn rebind_source_work_can_promote_attachment_rebind_and_trigger_recompute() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline schema");
        let mut library_asset_id = 0;

        admit_write(&mut connection, |write| {
            seed_source_fixture(
                write,
                "C:/music-c",
                SourceFixture {
                    source_id: 120,
                    source_directory_id: 220,
                    source_file_id: 320,
                },
                "album/track-c.flac",
                10,
            )?;
            seed_source_fixture(
                write,
                "C:/music-c",
                SourceFixture {
                    source_id: 120,
                    source_directory_id: 220,
                    source_file_id: 321,
                },
                "album/track-c-relocated.flac",
                11,
            )?;

            queue_and_promote_inspection(write, 320, "basis:source:c1", 20)?;
            queue_and_promote_segmentation(write, 320, 420, 520, "basis:segments:c1", 30)?;
            queue_and_promote_inspection(write, 321, "basis:source:c2", 40)?;
            queue_and_promote_segmentation(write, 321, 421, 620, "basis:segments:c2", 50)?;

            library_asset_id = ResolveLibraryAssetPromotionTx::new(write)
                .resolve_library_asset(&ResolveLibraryAssetPromotionInput {
                    equivalence_fingerprint: "eq:track-c".to_string(),
                    retention_policy: LibraryAssetRetentionPolicy::KeepMetadata,
                    source_segment_ids: vec![source_segment_domain_id(520)],
                    accepted_at: 60,
                    updated_at: 60,
                    rebuild_projection_domains: vec![],
                    rebuild_priority: WorkPriorityClass::Interactive,
                })?
                .library_asset
                .library_asset_id
                .get();
            queue_and_promote_capability(
                write,
                library_asset_id,
                "waveform",
                80,
                80,
                "basis:waveform:c1",
                70,
            )?;

            write.execute(
                "UPDATE source_files
                 SET presence_state = 'missing',
                     last_presence_change_at = 79,
                     updated_at = 79
                 WHERE source_file_id = 320",
                [],
            )?;

            let queued = RebindSourceWorkAuthorityTx::new(write).queue_rebind_source_work(
                &QueueRebindSourceWorkInput {
                    source_file_id: source_file_domain_id(320),
                    basis_fingerprint: "basis:rebind:c1".to_string(),
                    priority_class: WorkPriorityClass::Interactive,
                    queued_at: 80,
                },
            )?;
            assert!(queued.created);

            let claimed = claim_one_work(write, 81)?;
            let artifact = record_primary_inline_artifact(
                write,
                claimed.work_item_id,
                ArtifactKind::DiagnosticResult,
                "basis:rebind:c1",
                82,
                "hash:rebind:320:basis:rebind:c1",
            )?;
            RebindSourcePromotionTx::new(write).rebind_source(&RebindSourcePromotionInput {
                source_file_id: source_file_domain_id(320),
                accepted_artifact_id: artifact.artifact_id,
                basis_fingerprint: "basis:rebind:c1".to_string(),
                library_asset_id: library_asset_domain_id(library_asset_id),
                source_segment_ids: vec![source_segment_domain_id(620)],
                accepted_at: 83,
                updated_at: 83,
                rebuild_projection_domains: vec![],
                rebuild_priority: WorkPriorityClass::Interactive,
            })?;
            WorkItemsAuthorityTx::new(write).complete_machine_work_item(
                &CompleteMachineWorkInput {
                    work_item_id: claimed.work_item_id,
                    completed_at: 84,
                },
            )?;
            Ok(())
        })
        .expect("complete rebind flow");

        let rebound_segment_id: i64 = connection
            .query_row(
                "SELECT source_segment_id
                 FROM LibraryAssetAttachments
                 WHERE library_asset_id = ?1",
                [library_asset_id],
                |row| row.get(0),
            )
            .expect("read rebound attachment");
        assert_eq!(rebound_segment_id, 620);

        let waveform_state: String = connection
            .query_row(
                "SELECT state
                 FROM LibraryAssetCapabilities
                 WHERE library_asset_id = ?1
                   AND capability_kind = 'waveform'
                   AND profile_key = 'default'",
                [library_asset_id],
                |row| row.get(0),
            )
            .expect("read waveform state");
        assert_eq!(waveform_state, CapabilityState::Stale.as_str());

        let queued_waveform_work_count: i64 = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM WorkItems
                 WHERE subject_kind = 'library_asset'
                   AND subject_id = ?1
                   AND work_kind = 'compute_capability'
                   AND capability_kind = 'waveform'
                   AND target_profile_key = 'default'
                   AND state = 'queued'",
                [library_asset_id.to_string()],
                |row| row.get(0),
            )
            .expect("count queued waveform work");
        assert_eq!(queued_waveform_work_count, 1);
    }

    fn seed_source_fixture(
        write: &AdmittedWrite<'_>,
        root_path: &str,
        fixture: SourceFixture,
        relative_path: &str,
        changed_at: i64,
    ) -> crate::LibrarySqliteResult<()> {
        let source_id = SourcesAuthorityTx::new(write).upsert_source(&UpsertSourceInput {
            source_id: Some(fixture.source_id),
            source_class: "internal".to_string(),
            authority: "system".to_string(),
            identity_kind: "filesystem_uuid".to_string(),
            identity_value: format!("source-{}", fixture.source_id),
            display_name: format!("Source {}", fixture.source_id),
            medium_label: None,
            is_user_visible: true,
            browser_order_ordinal: Some(0),
            changed_at,
        })?;
        SourceLocatorsAuthorityTx::new(write).upsert_source_locator(&UpsertSourceLocatorInput {
            source_id,
            locator: crate::authority::sources::SourceLocatorInput::AbsolutePath {
                absolute_path: root_path.to_string(),
            },
        })?;
        SourceStateAuthorityTx::new(write).upsert_source_state(&UpsertSourceStateInput {
            source_id,
            mount_status: RootMountStatus::Mounted.as_str().to_string(),
            mount_epoch: 0,
            access_state: SourceAccessState::Accessible,
            access_issue_kind: None,
            access_error_detail: None,
            access_checked_at: Some(changed_at),
            mount_root: Some(root_path.to_string()),
            effective_path: Some(root_path.to_string()),
            observed_volume_label: Some("Volume".to_string()),
            filesystem_type: Some("exfat".to_string()),
            last_seen_at: Some(changed_at),
            updated_at: changed_at,
        })?;
        SourceStateAuthorityTx::new(write).upsert_source_scan_state(
            &UpsertSourceScanStateInput {
                source_id,
                scan_phase: SourceScanPhase::Idle,
                last_scan_started_at: Some(changed_at),
                last_scan_finished_at: Some(changed_at),
                last_successful_scan_at: Some(changed_at),
                scan_issue_kind: None,
                error_detail: None,
                updated_at: changed_at,
            },
        )?;
        let source_directory_id = SourceDirectoriesAuthorityTx::new(write)
            .upsert_source_directory(&UpsertSourceDirectoryInput {
                source_directory_id: Some(fixture.source_directory_id),
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
                first_created_at: Some(changed_at),
                changed_at,
            })?;
        assert_eq!(source_directory_id, fixture.source_directory_id);

        let source_file_id = SourceFilesAuthorityTx::new(write).record_source_file_observation(
            &RecordSourceFileObservationInput {
                source_file_id: Some(fixture.source_file_id),
                source_id,
                parent_source_directory_id: Some(source_directory_id),
                name: relative_path
                    .split('/')
                    .next_back()
                    .expect("file name")
                    .to_string(),
                relative_path: relative_path.to_string(),
                size_bytes: Some(123_456),
                mtime_ns: Some(changed_at * 1_000),
                presence_state: SourcePresenceState::Present,
                first_discovered_at: Some(changed_at),
                observed_at: Some(changed_at),
                presence_changed_at: changed_at,
                updated_at: changed_at,
            },
        )?;
        assert_eq!(source_file_id, fixture.source_file_id);
        Ok(())
    }

    fn claim_one_work(
        write: &mut AdmittedWrite<'_>,
        claimed_at: i64,
    ) -> crate::LibrarySqliteResult<crate::authority::work::ClaimedMachineWorkItem> {
        let claimed = WorkItemsAuthorityTx::new(write).claim_machine_work_batch(
            &ClaimMachineWorkBatchInput {
                limit: 1,
                lease_duration_ms: 30_000,
                claimed_at,
            },
        )?;
        assert_eq!(claimed.len(), 1);
        Ok(claimed.into_iter().next().expect("claimed work item"))
    }

    fn queue_and_claim_inspection_work(
        write: &mut AdmittedWrite<'_>,
        source_file_id: i64,
        basis_fingerprint: &str,
        queued_at: i64,
    ) -> crate::LibrarySqliteResult<crate::authority::work::ClaimedMachineWorkItem> {
        WorkItemsAuthorityTx::new(write).queue_inspect_source_work(
            &QueueInspectSourceWorkInput {
                source_file_id: source_file_domain_id(source_file_id),
                basis_fingerprint: basis_fingerprint.to_string(),
                priority_class: WorkPriorityClass::Interactive,
                queued_at,
            },
        )?;
        claim_one_work(write, queued_at + 1)
    }

    fn start_test_work_run(
        write: &mut AdmittedWrite<'_>,
        work_item_id: WorkItemId,
        started_at: i64,
    ) -> crate::LibrarySqliteResult<StartedWorkRun> {
        WorkRunsAuthorityTx::new(write).start_work_run(&StartWorkRunInput {
            work_item_id,
            adapter_key: "test.adapter".to_string(),
            adapter_version: "1.0.0".to_string(),
            started_at,
        })
    }

    fn record_primary_inline_artifact(
        write: &mut AdmittedWrite<'_>,
        work_item_id: WorkItemId,
        artifact_kind: ArtifactKind,
        basis_fingerprint: &str,
        started_at: i64,
        payload_hash: &str,
    ) -> crate::LibrarySqliteResult<crate::authority::work::RecordedArtifact> {
        let work_run = start_test_work_run(write, work_item_id, started_at)?;
        let artifact = ArtifactsAuthorityTx::new(write).record_inline_artifact(
            &RecordInlineArtifactInput {
                artifact: RecordArtifactInput {
                    work_run_id: work_run.work_run_id,
                    artifact_kind,
                    artifact_role: ArtifactRole::PrimaryResult,
                    media_type: "application/json".to_string(),
                    basis_fingerprint: basis_fingerprint.to_string(),
                    payload_hash: payload_hash.to_string(),
                    created_at: started_at + 1,
                },
                payload: b"{}".to_vec(),
            },
        )?;
        WorkRunsAuthorityTx::new(write).finish_work_run(&FinishWorkRunInput {
            work_run_id: work_run.work_run_id,
            finished_at: started_at + 2,
            outcome: WorkRunOutcome::Completed,
            failure_kind: None,
            error_detail: None,
        })?;
        Ok(artifact)
    }

    fn assert_write_invariant_contains(error: LibrarySqliteError, expected_fragment: &str) {
        match error {
            LibrarySqliteError::WriteInvariant(detail) => {
                assert!(
                    detail.contains(expected_fragment),
                    "expected invariant containing {expected_fragment:?}, found {detail:?}"
                );
            }
            other => panic!("expected write invariant, found {other:?}"),
        }
    }

    fn source_file_domain_id(value: i64) -> SourceFileId {
        SourceFileId::new(value).expect("positive source file id")
    }

    fn library_asset_domain_id(value: i64) -> LibraryAssetId {
        LibraryAssetId::new(value).expect("positive library asset id")
    }

    fn source_segment_set_domain_id(value: i64) -> SourceSegmentSetId {
        SourceSegmentSetId::new(value).expect("positive source segment set id")
    }

    fn source_segment_domain_id(value: i64) -> SourceSegmentId {
        SourceSegmentId::new(value).expect("positive source segment id")
    }

    fn queue_and_promote_inspection(
        write: &mut AdmittedWrite<'_>,
        source_file_id: i64,
        basis_fingerprint: &str,
        changed_at: i64,
    ) -> crate::LibrarySqliteResult<i64> {
        WorkItemsAuthorityTx::new(write).queue_inspect_source_work(
            &QueueInspectSourceWorkInput {
                source_file_id: source_file_domain_id(source_file_id),
                basis_fingerprint: basis_fingerprint.to_string(),
                priority_class: WorkPriorityClass::Interactive,
                queued_at: changed_at,
            },
        )?;
        let claimed = claim_one_work(write, changed_at + 1)?;
        let artifact = record_primary_inline_artifact(
            write,
            claimed.work_item_id,
            ArtifactKind::InspectionResult,
            basis_fingerprint,
            changed_at + 2,
            &format!("hash:inspect:{source_file_id}:{basis_fingerprint}"),
        )?;
        InspectSourcePromotionTx::new(
            write,
            ArtifactFileStoreRoot::for_store_path("library.sqlite3"),
        )
        .inspect_source(&InspectSourcePromotionInput {
            source_facts: CommitAcceptedSourceFactsInput {
                source_file_id: source_file_domain_id(source_file_id),
                accepted_artifact_id: artifact.artifact_id,
                basis_fingerprint: basis_fingerprint.to_string(),
                observed_at_ms: changed_at + 3,
                content_hash: Some(ContentHashEvidence {
                    algorithm: "sha256".to_string(),
                    value: format!("{source_file_id}:{basis_fingerprint}"),
                }),
                media_kind: "audio".to_string(),
                mime_type: Some("audio/flac".to_string()),
                duration_ms: Some(180_000),
                sample_rate_hz: Some(44_100),
                channels: Some(2),
                bit_depth: Some(16),
                codec: Some("flac".to_string()),
                updated_at: changed_at + 3,
            },
            rebuild_projection_domains: vec![],
            rebuild_priority: WorkPriorityClass::Interactive,
        })?;
        WorkItemsAuthorityTx::new(write).complete_machine_work_item(&CompleteMachineWorkInput {
            work_item_id: claimed.work_item_id,
            completed_at: changed_at + 4,
        })?;
        Ok(artifact.artifact_id.get())
    }

    fn queue_and_promote_segmentation(
        write: &mut AdmittedWrite<'_>,
        source_file_id: i64,
        segment_set_id: i64,
        segment_id: i64,
        basis_fingerprint: &str,
        changed_at: i64,
    ) -> crate::LibrarySqliteResult<i64> {
        WorkItemsAuthorityTx::new(write).queue_accept_segmentation_work(
            &QueueAcceptSegmentationWorkInput {
                source_file_id: source_file_domain_id(source_file_id),
                basis_fingerprint: basis_fingerprint.to_string(),
                priority_class: WorkPriorityClass::Interactive,
                queued_at: changed_at,
            },
        )?;
        let claimed = claim_one_work(write, changed_at + 1)?;
        let artifact = record_primary_inline_artifact(
            write,
            claimed.work_item_id,
            ArtifactKind::SegmentationResult,
            basis_fingerprint,
            changed_at + 2,
            &format!("hash:segments:{source_file_id}:{basis_fingerprint}"),
        )?;
        AcceptSegmentationPromotionTx::new(
            write,
            ArtifactFileStoreRoot::for_store_path("library.sqlite3"),
        )
        .accept_segmentation(&AcceptSegmentationPromotionInput {
            segment_set: ReplaceAcceptedSourceSegmentSetInput {
                source_segment_set_id: Some(source_segment_set_domain_id(segment_set_id)),
                source_file_id: source_file_domain_id(source_file_id),
                segment_set_kind: "accepted_primary".to_string(),
                basis_fingerprint: basis_fingerprint.to_string(),
                accepted_artifact_id: artifact.artifact_id,
                accepted_at: changed_at + 3,
                updated_at: changed_at + 3,
                segments: vec![AcceptedSourceSegmentInput {
                    source_segment_id: Some(source_segment_domain_id(segment_id)),
                    segment_kind: "track_span".to_string(),
                    ordinal: 0,
                    start_offset_ms: 0,
                    end_offset_ms: Some(180_000),
                    display_title: Some("Track".to_string()),
                    display_artist: Some("Artist".to_string()),
                    display_album: Some("Album".to_string()),
                }],
            },
            rebuild_projection_domains: vec![],
            rebuild_priority: WorkPriorityClass::Interactive,
        })?;
        WorkItemsAuthorityTx::new(write).complete_machine_work_item(&CompleteMachineWorkInput {
            work_item_id: claimed.work_item_id,
            completed_at: changed_at + 4,
        })?;
        Ok(artifact.artifact_id.get())
    }

    fn queue_and_promote_capability(
        write: &mut AdmittedWrite<'_>,
        library_asset_id: i64,
        capability_kind: &str,
        target_quality: i64,
        quality_current: i64,
        basis_fingerprint: &str,
        changed_at: i64,
    ) -> crate::LibrarySqliteResult<i64> {
        let capability_kind =
            CapabilityKind::parse(capability_kind).expect("test capability kind is valid");
        WorkItemsAuthorityTx::new(write).queue_compute_capability_work(
            &QueueComputeCapabilityWorkInput {
                library_asset_id: library_asset_domain_id(library_asset_id),
                capability_kind: capability_kind.clone(),
                target_profile_key: "default".to_string(),
                target_quality,
                basis_fingerprint: basis_fingerprint.to_string(),
                priority_class: WorkPriorityClass::Interactive,
                queued_at: changed_at,
            },
        )?;
        let claimed = claim_one_work(write, changed_at + 1)?;
        let artifact = record_primary_inline_artifact(
            write,
            claimed.work_item_id,
            ArtifactKind::CapabilityResult,
            basis_fingerprint,
            changed_at + 2,
            &format!("hash:capability:{library_asset_id}:{capability_kind}:{basis_fingerprint}"),
        )?;
        ComputeCapabilityPromotionTx::new(
            write,
            ArtifactFileStoreRoot::for_store_path("library.sqlite3"),
        )
        .compute_capability(&ComputeCapabilityPromotionInput {
            capability: ReplaceLibraryAssetCapabilityInput {
                library_asset_id: library_asset_domain_id(library_asset_id),
                capability_kind,
                profile_key: "default".to_string(),
                state: CapabilityState::Ready,
                stability_class: Some(CapabilityStabilityClass::Stable),
                quality_current: Some(quality_current),
                basis_fingerprint: Some(basis_fingerprint.to_string()),
                selected_artifact_id: Some(artifact.artifact_id),
                updated_at: changed_at + 3,
            },
            rebuild_projection_domains: vec![],
            rebuild_priority: WorkPriorityClass::Interactive,
        })?;
        WorkItemsAuthorityTx::new(write).complete_machine_work_item(&CompleteMachineWorkInput {
            work_item_id: claimed.work_item_id,
            completed_at: changed_at + 4,
        })?;
        Ok(artifact.artifact_id.get())
    }
}
