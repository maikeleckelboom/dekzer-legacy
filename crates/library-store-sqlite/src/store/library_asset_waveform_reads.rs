use crate::LibrarySqliteResult;
use crate::read_models::library_asset_waveform_overview::{
    StoreLibraryAssetWaveformOverview,
    read_library_asset_waveform_overview as read_library_asset_waveform_overview_query,
};

use super::{SqliteDurableStore, bootstrap::open_connection};

impl SqliteDurableStore {
    pub fn read_library_asset_waveform_overview(
        &self,
        library_asset_id: i64,
    ) -> LibrarySqliteResult<Option<StoreLibraryAssetWaveformOverview>> {
        let connection = open_connection(&self.path)?;
        read_library_asset_waveform_overview_query(
            &connection,
            self.app_owned_state.artifact_file_store_root(),
            library_asset_id,
        )
    }
}

#[cfg(test)]
mod tests {
    use library_domain::{
        ArtifactKind, ArtifactRole, CapabilityKind, CapabilityStabilityClass, CapabilityState,
        LibraryAssetId, PrepPolicyId, PrepTargetStabilityClass, WorkPriorityClass, WorkRunOutcome,
    };
    use rusqlite::params;
    use tempfile::TempDir;

    use crate::read_models::library_asset_waveform_overview::LIBRARY_ASSET_WAVEFORM_OVERVIEW_MEDIA_TYPE;
    use crate::{
        ClaimMachineWorkBatchInput, CompleteMachineWorkInput, ComputeCapabilityPromotionInput,
        FinishWorkRunInput, LibrarySqliteError, QueueComputeCapabilityWorkInput,
        RecordArtifactInput, RecordInlineArtifactInput, ReplaceLibraryAssetCapabilityInput,
        ReplaceResolvedLibraryAssetPrepTargetsInput, ResolvedLibraryAssetPrepTargetInput,
        StartWorkRunInput,
    };

    use super::SqliteDurableStore;

    fn open_store() -> (TempDir, SqliteDurableStore) {
        let tempdir = TempDir::new().expect("create tempdir");
        let store = SqliteDurableStore::open(tempdir.path().join("library.sqlite3"))
            .expect("open durable store");
        (tempdir, store)
    }

    fn insert_library_asset(store: &SqliteDurableStore, library_asset_id: i64, updated_at: i64) {
        store
            .with_write(|write| {
                write.execute(
                    "INSERT INTO LibraryAssets (
                         library_asset_id,
                         equivalence_fingerprint,
                         retention_policy,
                         created_at,
                         updated_at
                     )
                     VALUES (?1, ?2, 'keep_metadata', ?3, ?3)",
                    params![
                        library_asset_id,
                        format!("eq:waveform-overview:{library_asset_id}"),
                        updated_at,
                    ],
                )?;
                Ok(())
            })
            .expect("insert library asset");
    }

    struct WaveformOverviewAcceptance<'a> {
        library_asset_id: i64,
        profile_key: &'a str,
        target_quality: i64,
        quality_current: Option<i64>,
        basis_fingerprint: &'a str,
        payload: Vec<u8>,
        changed_at: i64,
    }

    fn accept_waveform_overview(
        store: &SqliteDurableStore,
        input: WaveformOverviewAcceptance<'_>,
    ) -> i64 {
        accept_waveform_overview_with_media_type(
            store,
            input,
            LIBRARY_ASSET_WAVEFORM_OVERVIEW_MEDIA_TYPE,
        )
    }

    fn accept_waveform_overview_with_media_type(
        store: &SqliteDurableStore,
        input: WaveformOverviewAcceptance<'_>,
        media_type: &str,
    ) -> i64 {
        let queued = store
            .queue_compute_capability_work(QueueComputeCapabilityWorkInput {
                library_asset_id: library_asset_id(input.library_asset_id),
                capability_kind: CapabilityKind::waveform(),
                target_profile_key: input.profile_key.to_string(),
                target_quality: input.target_quality,
                basis_fingerprint: input.basis_fingerprint.to_string(),
                priority_class: WorkPriorityClass::Interactive,
                queued_at: input.changed_at,
            })
            .expect("queue waveform capability work");
        let claimed = store
            .claim_machine_work_batch(ClaimMachineWorkBatchInput {
                limit: 1,
                lease_duration_ms: 30_000,
                claimed_at: input.changed_at + 1,
            })
            .expect("claim waveform work");
        assert_eq!(claimed.len(), 1);
        assert_eq!(claimed[0].work_item_id, queued.work_item_id);

        let run = store
            .start_work_run(StartWorkRunInput {
                work_item_id: queued.work_item_id,
                adapter_key: "test.waveform".to_string(),
                adapter_version: "1.0.0".to_string(),
                started_at: input.changed_at + 2,
            })
            .expect("start waveform work run");
        let artifact = store
            .record_inline_artifact(RecordInlineArtifactInput {
                artifact: RecordArtifactInput {
                    work_run_id: run.work_run_id,
                    artifact_kind: ArtifactKind::CapabilityResult,
                    artifact_role: ArtifactRole::PrimaryResult,
                    media_type: media_type.to_string(),
                    basis_fingerprint: input.basis_fingerprint.to_string(),
                    payload_hash: format!(
                        "hash:{}:{}:{}",
                        input.library_asset_id, input.profile_key, input.changed_at
                    ),
                    created_at: input.changed_at + 3,
                },
                payload: input.payload,
            })
            .expect("record waveform artifact");
        store
            .finish_work_run(FinishWorkRunInput {
                work_run_id: run.work_run_id,
                finished_at: input.changed_at + 4,
                outcome: WorkRunOutcome::Completed,
                failure_kind: None,
                error_detail: None,
            })
            .expect("finish waveform work run");
        store
            .compute_capability(ComputeCapabilityPromotionInput {
                capability: ReplaceLibraryAssetCapabilityInput {
                    library_asset_id: library_asset_id(input.library_asset_id),
                    capability_kind: CapabilityKind::waveform(),
                    profile_key: input.profile_key.to_string(),
                    state: CapabilityState::Ready,
                    stability_class: Some(CapabilityStabilityClass::Stable),
                    quality_current: input.quality_current,
                    basis_fingerprint: Some(input.basis_fingerprint.to_string()),
                    selected_artifact_id: Some(artifact.artifact_id),
                    updated_at: input.changed_at + 5,
                },
                rebuild_projection_domains: Vec::new(),
                rebuild_priority: WorkPriorityClass::Interactive,
            })
            .expect("accept waveform capability");
        store
            .complete_machine_work_item(CompleteMachineWorkInput {
                work_item_id: queued.work_item_id,
                completed_at: input.changed_at + 6,
            })
            .expect("complete waveform work item");

        artifact.artifact_id.get()
    }

    fn overview_payload(first_min: i16, first_max: i16) -> Vec<u8> {
        overview_payload_with_schema_version(1, first_min, first_max)
    }

    fn overview_payload_with_schema_version(
        schema_version: i64,
        first_min: i16,
        first_max: i16,
    ) -> Vec<u8> {
        format!(
            r#"{{
                "schema_version": {schema_version},
                "bucket_count": 2,
                "duration_ms": 1000,
                "source_sample_count": 44100,
                "samples_per_bucket": 22050,
                "amplitude_scale": "signed_i16",
                "buckets": [
                    {{"min_amplitude_i16": {first_min}, "max_amplitude_i16": {first_max}}},
                    {{"min_amplitude_i16": -80, "max_amplitude_i16": 140}}
                ]
            }}"#
        )
        .into_bytes()
    }

    fn wrapped_overview_payload() -> Vec<u8> {
        format!(
            r#"{{
                "library_asset_waveform_overview": {}
            }}"#,
            String::from_utf8(overview_payload(-12, 34)).expect("overview payload is utf8")
        )
        .into_bytes()
    }

    fn assert_malformed_schema_state_contains(error: LibrarySqliteError, expected: &str) {
        match error {
            LibrarySqliteError::MalformedSchemaState(detail) => {
                assert!(
                    detail.contains(expected),
                    "expected {detail:?} to contain {expected:?}"
                );
            }
            other => panic!("expected MalformedSchemaState, got {other:?}"),
        }
    }

    fn delete_browser_projection_row(store: &SqliteDurableStore, library_asset_id: i64) {
        store
            .with_write(|write| {
                write.execute(
                    "DELETE FROM LibraryBrowserRows_fts WHERE rowid = ?1",
                    [library_asset_id],
                )?;
                write.execute(
                    "DELETE FROM LibraryBrowserRows WHERE library_asset_id = ?1",
                    [library_asset_id],
                )?;
                Ok(())
            })
            .expect("delete browser projection row");
    }

    #[test]
    fn overview_read_is_keyed_by_library_asset_id_not_browser_row_identity() {
        let (_tempdir, store) = open_store();
        insert_library_asset(&store, 10, 100);
        let accepted_artifact_id = accept_waveform_overview(
            &store,
            WaveformOverviewAcceptance {
                library_asset_id: 10,
                profile_key: "default",
                target_quality: 90,
                quality_current: Some(86),
                basis_fingerprint: "basis:waveform:10:default",
                payload: overview_payload(-120, 180),
                changed_at: 200,
            },
        );
        delete_browser_projection_row(&store, 10);

        let overview = store
            .read_library_asset_waveform_overview(10)
            .expect("read waveform overview")
            .expect("overview remains available without browser row");

        assert_eq!(overview.accepted_artifact_id, accepted_artifact_id);
        assert_eq!(overview.source_profile_key, "default");
        assert_eq!(overview.source_quality_current, Some(86));
        assert_eq!(overview.bucket_count, 2);
        assert_eq!(overview.buckets[0].min_amplitude_i16, -120);
    }

    #[test]
    fn overview_read_uses_selected_accepted_waveform_artifact() {
        let (_tempdir, store) = open_store();
        insert_library_asset(&store, 11, 100);
        let first_artifact_id = accept_waveform_overview(
            &store,
            WaveformOverviewAcceptance {
                library_asset_id: 11,
                profile_key: "default",
                target_quality: 80,
                quality_current: Some(70),
                basis_fingerprint: "basis:waveform:11:first",
                payload: overview_payload(-300, 300),
                changed_at: 200,
            },
        );
        let second_artifact_id = accept_waveform_overview(
            &store,
            WaveformOverviewAcceptance {
                library_asset_id: 11,
                profile_key: "default",
                target_quality: 90,
                quality_current: Some(88),
                basis_fingerprint: "basis:waveform:11:second",
                payload: overview_payload(-44, 55),
                changed_at: 300,
            },
        );

        let overview = store
            .read_library_asset_waveform_overview(11)
            .expect("read waveform overview")
            .expect("overview exists");

        assert_ne!(first_artifact_id, second_artifact_id);
        assert_eq!(overview.accepted_artifact_id, second_artifact_id);
        assert_eq!(overview.basis_fingerprint, "basis:waveform:11:second");
        assert_eq!(overview.source_quality_current, Some(88));
        assert_eq!(overview.buckets[0].min_amplitude_i16, -44);
        assert_eq!(overview.buckets[0].max_amplitude_i16, 55);
    }

    #[test]
    fn overview_profile_selection_matches_browser_summary_selection() {
        let (_tempdir, store) = open_store();
        insert_library_asset(&store, 12, 100);
        accept_waveform_overview(
            &store,
            WaveformOverviewAcceptance {
                library_asset_id: 12,
                profile_key: "default",
                target_quality: 80,
                quality_current: Some(70),
                basis_fingerprint: "basis:waveform:12:default",
                payload: overview_payload(-10, 20),
                changed_at: 200,
            },
        );
        accept_waveform_overview(
            &store,
            WaveformOverviewAcceptance {
                library_asset_id: 12,
                profile_key: "hires",
                target_quality: 95,
                quality_current: Some(88),
                basis_fingerprint: "basis:waveform:12:hires",
                payload: overview_payload(-30, 40),
                changed_at: 300,
            },
        );
        store
            .replace_resolved_library_asset_prep_targets(
                ReplaceResolvedLibraryAssetPrepTargetsInput {
                    library_asset_id: library_asset_id(12),
                    targets: vec![
                        ResolvedLibraryAssetPrepTargetInput {
                            capability_kind: CapabilityKind::waveform(),
                            target_profile_key: "default".to_string(),
                            target_quality: 80,
                            target_stability_class: PrepTargetStabilityClass::Stable,
                            priority_class: WorkPriorityClass::Interactive,
                            resolved_from_policy_id: insert_test_policy(&store),
                        },
                        ResolvedLibraryAssetPrepTargetInput {
                            capability_kind: CapabilityKind::waveform(),
                            target_profile_key: "hires".to_string(),
                            target_quality: 95,
                            target_stability_class: PrepTargetStabilityClass::Stable,
                            priority_class: WorkPriorityClass::Interactive,
                            resolved_from_policy_id: insert_test_policy(&store),
                        },
                    ],
                    updated_at: 400,
                },
            )
            .expect("replace resolved waveform targets");

        let overview = store
            .read_library_asset_waveform_overview(12)
            .expect("read waveform overview")
            .expect("overview exists");
        let browser = store
            .read_library_browser_window(0, 10)
            .expect("read browser window");
        let row = browser
            .rows
            .iter()
            .find(|row| row.library_asset_id == 12)
            .expect("browser row exists");

        assert_eq!(overview.source_profile_key, "hires");
        assert_eq!(overview.source_quality_current, Some(88));
        assert_eq!(row.waveform_quality_current, Some(88));
        assert_eq!(row.waveform_quality_target, Some(95));
    }

    #[test]
    fn missing_capability_or_non_overview_media_type_returns_none() {
        let (_tempdir, store) = open_store();
        insert_library_asset(&store, 13, 100);

        assert!(
            store
                .read_library_asset_waveform_overview(13)
                .expect("read missing waveform overview")
                .is_none()
        );

        accept_waveform_overview_with_media_type(
            &store,
            WaveformOverviewAcceptance {
                library_asset_id: 13,
                profile_key: "default",
                target_quality: 80,
                quality_current: Some(70),
                basis_fingerprint: "basis:waveform:13:empty",
                payload: overview_payload(-1, 1),
                changed_at: 200,
            },
            "application/json",
        );

        assert!(
            store
                .read_library_asset_waveform_overview(13)
                .expect("read accepted waveform without overview media type")
                .is_none()
        );
    }

    #[test]
    fn overview_artifact_schema_version_one_is_supported_and_other_versions_are_rejected() {
        let (_tempdir, store) = open_store();
        insert_library_asset(&store, 15, 100);
        accept_waveform_overview(
            &store,
            WaveformOverviewAcceptance {
                library_asset_id: 15,
                profile_key: "default",
                target_quality: 80,
                quality_current: Some(70),
                basis_fingerprint: "basis:waveform:15:v1",
                payload: overview_payload_with_schema_version(1, -10, 10),
                changed_at: 200,
            },
        );

        let overview = store
            .read_library_asset_waveform_overview(15)
            .expect("read supported schema version")
            .expect("overview exists for schema version 1");
        assert_eq!(overview.bucket_count, 2);

        insert_library_asset(&store, 16, 100);
        accept_waveform_overview(
            &store,
            WaveformOverviewAcceptance {
                library_asset_id: 16,
                profile_key: "default",
                target_quality: 80,
                quality_current: Some(70),
                basis_fingerprint: "basis:waveform:16:v2",
                payload: overview_payload_with_schema_version(2, -10, 10),
                changed_at: 300,
            },
        );

        let error = store
            .read_library_asset_waveform_overview(16)
            .expect_err("unsupported schema version must be rejected");
        assert_malformed_schema_state_contains(
            error,
            "unsupported schema_version 2; supported schema_version is 1",
        );
    }

    #[test]
    fn overview_artifact_requires_schema_version() {
        let (_tempdir, store) = open_store();
        insert_library_asset(&store, 17, 100);
        accept_waveform_overview(
            &store,
            WaveformOverviewAcceptance {
                library_asset_id: 17,
                profile_key: "default",
                target_quality: 80,
                quality_current: Some(70),
                basis_fingerprint: "basis:waveform:17:missing-version",
                payload: br#"{
                    "bucket_count": 2,
                    "duration_ms": 1000,
                    "source_sample_count": 44100,
                    "samples_per_bucket": 22050,
                    "amplitude_scale": "signed_i16",
                    "buckets": [
                        {"min_amplitude_i16": -10, "max_amplitude_i16": 10},
                        {"min_amplitude_i16": -20, "max_amplitude_i16": 20}
                    ]
                }"#
                .to_vec(),
                changed_at: 200,
            },
        );

        let error = store
            .read_library_asset_waveform_overview(17)
            .expect_err("missing schema_version must be rejected");
        assert_malformed_schema_state_contains(error, "schema_version is required");
    }

    #[test]
    fn overview_artifact_rejects_wrapped_json_shape() {
        let (_tempdir, store) = open_store();
        insert_library_asset(&store, 18, 100);
        accept_waveform_overview(
            &store,
            WaveformOverviewAcceptance {
                library_asset_id: 18,
                profile_key: "default",
                target_quality: 80,
                quality_current: Some(70),
                basis_fingerprint: "basis:waveform:18:wrapped",
                payload: wrapped_overview_payload(),
                changed_at: 200,
            },
        );

        let error = store
            .read_library_asset_waveform_overview(18)
            .expect_err("wrapped overview shape must be rejected");
        assert_malformed_schema_state_contains(
            error,
            "library_asset_waveform_overview wrapper is not supported",
        );
    }

    #[test]
    fn browser_projection_summary_is_not_waveform_overview_authority() {
        let (_tempdir, store) = open_store();
        insert_library_asset(&store, 14, 100);
        store
            .with_write(|write| {
                write.execute(
                    "INSERT INTO LibraryBrowserRows (
                         library_asset_id,
                         row_version,
                         primary_source_file_id,
                         availability_state,
                         title,
                         artist,
                         album,
                         duration_ms,
                         musical_key,
                         tempo_bpm,
                         waveform_quality_current,
                         waveform_quality_target,
                         stems_state_summary,
                         prep_readiness_summary,
                         updated_at
                     )
                     VALUES (?1, 1, NULL, 'available', NULL, NULL, NULL, NULL, NULL, NULL, 99, 100, 'missing', 'not_required', ?2)",
                    params![14, 200],
                )?;
                Ok(())
            })
            .expect("insert projection-only browser row");

        assert!(
            store
                .read_library_asset_waveform_overview(14)
                .expect("read waveform overview")
                .is_none()
        );
    }

    fn insert_test_policy(store: &SqliteDurableStore) -> PrepPolicyId {
        store
            .with_write(|write| {
                write.execute(
                    "INSERT INTO PrepPolicies (
                         policy_name,
                         is_system_policy,
                         is_user_editable,
                         created_at,
                         updated_at
                     )
                     VALUES ('test waveform overview policy', 0, 1, 1, 1)
                     ON CONFLICT(policy_name) DO UPDATE
                     SET updated_at = excluded.updated_at",
                    [],
                )?;
                let policy_id = write.query_row(
                    "SELECT prep_policy_id
                     FROM PrepPolicies
                     WHERE policy_name = 'test waveform overview policy'",
                    [],
                    |row| row.get(0),
                )?;
                Ok(PrepPolicyId::new(policy_id).expect("positive prep policy id"))
            })
            .expect("insert test prep policy")
    }

    fn library_asset_id(value: i64) -> LibraryAssetId {
        LibraryAssetId::new(value).expect("positive library asset id")
    }
}
