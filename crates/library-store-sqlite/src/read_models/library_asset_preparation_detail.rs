use rusqlite::{Connection, OptionalExtension, params};

use crate::{LibrarySqliteError, LibrarySqliteResult};

pub const PREPARATION_DETAIL_CAPABILITY_TEMPO: &str = "tempo";
pub const PREPARATION_DETAIL_CAPABILITY_MUSICAL_KEY: &str = "musical_key";
pub const PREPARATION_DETAIL_CAPABILITY_BEATGRID: &str = "beatgrid";
pub const PREPARATION_DETAIL_CAPABILITY_WAVEFORM: &str = "waveform";
pub const PREPARATION_DETAIL_CAPABILITY_STEMS: &str = "stems";

const WAVEFORM_OVERVIEW_MEDIA_TYPE: &str =
    "application/vnd.music-library.library-asset-waveform-overview+json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreLibraryAssetPreparationDetail {
    pub library_asset_id: i64,
    pub aggregate_readiness_summary: String,
    pub groups: Vec<StoreLibraryAssetPreparationDetailGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreLibraryAssetPreparationDetailGroup {
    pub group_key: String,
    pub rows: Vec<StoreLibraryAssetPreparationDetailRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreLibraryAssetPreparationDetailRow {
    pub capability_kind: String,
    pub label: String,
    pub requirement_class: String,
    pub outcome_kind: String,
    pub work_state: String,
    pub outcome_state: String,
    pub satisfaction_state: String,
    pub display_value_summary: Option<String>,
    pub target_summary: Option<String>,
    pub explanation_summary: Option<String>,
    pub artifact_coverage_state: Option<String>,
    pub progress: Option<StoreLibraryAssetPreparationProgress>,
    pub sort_order: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreLibraryAssetPreparationProgress {
    Indeterminate {
        active_stage: Option<String>,
    },
    BoundedStage {
        active_stage: Option<String>,
        completed_units: i64,
        total_units: i64,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct PreparationDetailRowRecord {
    capability_kind: String,
    label: String,
    outcome_kind: String,
    sort_order: i64,
    default_profile_key: String,
    target_profile_key: Option<String>,
    target_quality: Option<i64>,
    target_stability_class: Option<String>,
    target_updated_at: Option<i64>,
    capability_state: Option<String>,
    stability_class: Option<String>,
    quality_current: Option<i64>,
    basis_fingerprint: Option<String>,
    selected_artifact_id: Option<i64>,
    capability_updated_at: Option<i64>,
    media_type: Option<String>,
    inline_payload: Option<Vec<u8>>,
    work_item_state: Option<String>,
    blocked_reason: Option<String>,
    failure_kind: Option<String>,
    error_detail: Option<String>,
}

pub fn read_library_asset_preparation_detail(
    connection: &Connection,
    library_asset_id: i64,
) -> LibrarySqliteResult<Option<StoreLibraryAssetPreparationDetail>> {
    if !library_asset_exists(connection, library_asset_id)? {
        return Ok(None);
    }

    let mut rows = read_preparation_detail_rows(connection, library_asset_id)?
        .into_iter()
        .map(compose_preparation_detail_row)
        .collect::<LibrarySqliteResult<Vec<_>>>()?;
    rows.sort_by_key(|row| row.sort_order);

    let aggregate_readiness_summary = aggregate_readiness_summary(&rows);
    let groups = group_preparation_detail_rows(rows);

    Ok(Some(StoreLibraryAssetPreparationDetail {
        library_asset_id,
        aggregate_readiness_summary,
        groups,
    }))
}

fn library_asset_exists(
    connection: &Connection,
    library_asset_id: i64,
) -> LibrarySqliteResult<bool> {
    let exists = connection
        .query_row(
            "SELECT 1 FROM LibraryAssets WHERE library_asset_id = ?1",
            [library_asset_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    Ok(exists)
}

fn read_preparation_detail_rows(
    connection: &Connection,
    library_asset_id: i64,
) -> LibrarySqliteResult<Vec<PreparationDetailRowRecord>> {
    let query = "
        WITH initial_capabilities(capability_kind, label, outcome_kind, sort_order) AS (
            VALUES
                ('tempo', 'BPM', 'fact', 0),
                ('musical_key', 'Musical Key', 'fact', 1),
                ('beatgrid', 'Beatgrid', 'structure', 2),
                ('waveform', 'Waveform', 'artifact', 3),
                ('stems', 'Stems', 'artifact', 4)
        ),
        target_candidates AS (
            SELECT rpt.library_asset_id,
                   rpt.capability_kind,
                   rpt.target_profile_key,
                   rpt.target_quality,
                   rpt.target_stability_class,
                   rpt.updated_at,
                   ROW_NUMBER() OVER (
                       PARTITION BY rpt.capability_kind
                       ORDER BY rpt.target_quality DESC,
                                CASE rpt.target_stability_class WHEN 'stable' THEN 0 ELSE 1 END,
                                CASE
                                    WHEN rpt.target_profile_key = specs.default_profile_key THEN 0
                                    ELSE 1
                                END,
                                rpt.target_profile_key ASC
                   ) AS target_rank
            FROM ResolvedLibraryAssetPrepTargets rpt
            JOIN CapabilitySpecs specs
              ON specs.capability_kind = rpt.capability_kind
            JOIN initial_capabilities initial
              ON initial.capability_kind = rpt.capability_kind
            WHERE rpt.library_asset_id = ?1
        ),
        selected_targets AS (
            SELECT capability_kind,
                   target_profile_key,
                   target_quality,
                   target_stability_class,
                   updated_at
            FROM target_candidates
            WHERE target_rank = 1
        ),
        row_basis AS (
            SELECT initial.capability_kind,
                   initial.label,
                   initial.outcome_kind,
                   initial.sort_order,
                   specs.default_profile_key,
                   selected_targets.target_profile_key,
                   selected_targets.target_quality,
                   selected_targets.target_stability_class,
                   selected_targets.updated_at AS target_updated_at,
                   COALESCE(selected_targets.target_profile_key, specs.default_profile_key)
                       AS selected_profile_key
            FROM initial_capabilities initial
            JOIN CapabilitySpecs specs
              ON specs.capability_kind = initial.capability_kind
            LEFT JOIN selected_targets
              ON selected_targets.capability_kind = initial.capability_kind
        ),
        work_candidates AS (
            SELECT wi.capability_kind,
                   wi.target_profile_key,
                   wi.state,
                   wi.blocked_reason,
                   wi.failure_kind,
                   wi.error_detail,
                   ROW_NUMBER() OVER (
                       PARTITION BY wi.capability_kind, wi.target_profile_key
                       ORDER BY CASE wi.state
                                    WHEN 'leased' THEN 0
                                    WHEN 'queued' THEN 1
                                    WHEN 'blocked' THEN 2
                                    WHEN 'failed' THEN 3
                                    ELSE 4
                                END,
                                wi.updated_at DESC,
                                wi.work_item_id DESC
                   ) AS work_rank
            FROM WorkItems wi
            JOIN initial_capabilities initial
              ON initial.capability_kind = wi.capability_kind
            WHERE wi.subject_kind = 'library_asset'
              AND wi.subject_id = CAST(?1 AS TEXT)
              AND wi.work_kind = 'compute_capability'
              AND wi.state IN ('queued', 'leased', 'blocked', 'failed')
        )
        SELECT row_basis.capability_kind,
               row_basis.label,
               row_basis.outcome_kind,
               row_basis.sort_order,
               row_basis.default_profile_key,
               row_basis.target_profile_key,
               row_basis.target_quality,
               row_basis.target_stability_class,
               row_basis.target_updated_at,
               pic.state,
               pic.stability_class,
               pic.quality_current,
               pic.basis_fingerprint,
               pic.selected_artifact_id,
               pic.updated_at,
               artifacts.media_type,
               inline_payload.payload,
               work_candidates.state,
               work_candidates.blocked_reason,
               work_candidates.failure_kind,
               work_candidates.error_detail
        FROM row_basis
        LEFT JOIN LibraryAssetCapabilities pic
          ON pic.library_asset_id = ?1
         AND pic.capability_kind = row_basis.capability_kind
         AND pic.profile_key = row_basis.selected_profile_key
        LEFT JOIN Artifacts artifacts
          ON artifacts.artifact_id = pic.selected_artifact_id
        LEFT JOIN ArtifactInlinePayloads inline_payload
          ON inline_payload.artifact_id = artifacts.artifact_id
        LEFT JOIN work_candidates
          ON work_candidates.capability_kind = row_basis.capability_kind
         AND work_candidates.target_profile_key = row_basis.selected_profile_key
         AND work_candidates.work_rank = 1
        ORDER BY row_basis.sort_order ASC";

    connection
        .prepare(query)?
        .query_map(params![library_asset_id], preparation_detail_row_from_sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn preparation_detail_row_from_sql(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<PreparationDetailRowRecord> {
    Ok(PreparationDetailRowRecord {
        capability_kind: row.get(0)?,
        label: row.get(1)?,
        outcome_kind: row.get(2)?,
        sort_order: row.get(3)?,
        default_profile_key: row.get(4)?,
        target_profile_key: row.get(5)?,
        target_quality: row.get(6)?,
        target_stability_class: row.get(7)?,
        target_updated_at: row.get(8)?,
        capability_state: row.get(9)?,
        stability_class: row.get(10)?,
        quality_current: row.get(11)?,
        basis_fingerprint: row.get(12)?,
        selected_artifact_id: row.get(13)?,
        capability_updated_at: row.get(14)?,
        media_type: row.get(15)?,
        inline_payload: row.get(16)?,
        work_item_state: row.get(17)?,
        blocked_reason: row.get(18)?,
        failure_kind: row.get(19)?,
        error_detail: row.get(20)?,
    })
}

fn compose_preparation_detail_row(
    record: PreparationDetailRowRecord,
) -> LibrarySqliteResult<StoreLibraryAssetPreparationDetailRow> {
    validate_capability_kind(&record.capability_kind)?;
    validate_outcome_kind(&record.outcome_kind)?;

    let requirement_class = if record.target_profile_key.is_some() {
        "required"
    } else {
        "on_demand"
    };
    let work_state = compose_work_state(&record, requirement_class);
    let outcome_state = compose_outcome_state(&record);
    let satisfaction_state = compose_satisfaction_state(&record, requirement_class);
    let artifact_coverage_state = compose_artifact_coverage_state(&record, &outcome_state);
    let display_value_summary =
        compose_display_value_summary(&record, &outcome_state, artifact_coverage_state.as_deref())?;
    let target_summary = compose_target_summary(&record);
    let explanation_summary =
        compose_explanation_summary(&record, requirement_class, &work_state, &outcome_state);

    Ok(StoreLibraryAssetPreparationDetailRow {
        capability_kind: record.capability_kind,
        label: record.label,
        requirement_class: requirement_class.to_string(),
        outcome_kind: record.outcome_kind,
        work_state,
        outcome_state,
        satisfaction_state,
        display_value_summary,
        target_summary,
        explanation_summary,
        artifact_coverage_state,
        progress: None,
        sort_order: record.sort_order,
    })
}

fn validate_capability_kind(capability_kind: &str) -> LibrarySqliteResult<()> {
    match capability_kind {
        PREPARATION_DETAIL_CAPABILITY_TEMPO
        | PREPARATION_DETAIL_CAPABILITY_MUSICAL_KEY
        | PREPARATION_DETAIL_CAPABILITY_BEATGRID
        | PREPARATION_DETAIL_CAPABILITY_WAVEFORM
        | PREPARATION_DETAIL_CAPABILITY_STEMS => Ok(()),
        other => Err(LibrarySqliteError::MalformedSchemaState(format!(
            "preparation detail selected unsupported capability {other:?}"
        ))),
    }
}

fn validate_outcome_kind(outcome_kind: &str) -> LibrarySqliteResult<()> {
    match outcome_kind {
        "fact" | "structure" | "artifact" => Ok(()),
        other => Err(LibrarySqliteError::MalformedSchemaState(format!(
            "preparation detail selected unsupported outcome kind {other:?}"
        ))),
    }
}

fn compose_work_state(record: &PreparationDetailRowRecord, requirement_class: &str) -> String {
    if let Some(work_item_state) = record.work_item_state.as_deref() {
        return match work_item_state {
            "queued" => "queued",
            "leased" => "active",
            "blocked" => "blocked",
            "failed" => "failed",
            _ => "none",
        }
        .to_string();
    }

    match record.capability_state.as_deref() {
        Some("queued") => "queued".to_string(),
        Some("leased") => "active".to_string(),
        Some("blocked") => "blocked".to_string(),
        Some("failed") => "failed".to_string(),
        _ if requirement_class == "on_demand" => "not_requested".to_string(),
        _ => "none".to_string(),
    }
}

fn compose_outcome_state(record: &PreparationDetailRowRecord) -> String {
    match record.capability_state.as_deref() {
        Some("ready") if required_stable_but_capability_is_provisional(record) => {
            "provisional".to_string()
        }
        Some("ready") => "ready".to_string(),
        Some("stale") => "stale".to_string(),
        _ => "missing".to_string(),
    }
}

fn compose_satisfaction_state(
    record: &PreparationDetailRowRecord,
    requirement_class: &str,
) -> String {
    if requirement_class == "on_demand" {
        return if matches!(record.capability_state.as_deref(), Some("ready")) {
            "satisfied"
        } else {
            "unsatisfied"
        }
        .to_string();
    }

    if required_target_is_satisfied(record) {
        "satisfied".to_string()
    } else {
        "unsatisfied".to_string()
    }
}

fn required_target_is_satisfied(record: &PreparationDetailRowRecord) -> bool {
    if record.capability_state.as_deref() != Some("ready") {
        return false;
    }
    if record.quality_current.unwrap_or(0) < record.target_quality.unwrap_or(0) {
        return false;
    }
    match record.target_stability_class.as_deref() {
        Some("stable") => record.stability_class.as_deref() == Some("stable"),
        Some("provisional") => matches!(
            record.stability_class.as_deref(),
            Some("provisional" | "stable")
        ),
        _ => false,
    }
}

fn required_stable_but_capability_is_provisional(record: &PreparationDetailRowRecord) -> bool {
    record.target_stability_class.as_deref() == Some("stable")
        && record.stability_class.as_deref() == Some("provisional")
}

fn compose_artifact_coverage_state(
    record: &PreparationDetailRowRecord,
    outcome_state: &str,
) -> Option<String> {
    if record.capability_kind != PREPARATION_DETAIL_CAPABILITY_WAVEFORM {
        return None;
    }

    Some(
        match outcome_state {
            "stale" => "stale",
            "ready" | "provisional" => {
                if record.media_type.as_deref() == Some(WAVEFORM_OVERVIEW_MEDIA_TYPE) {
                    "overview_ready"
                } else if record.selected_artifact_id.is_some() {
                    "refinement_available"
                } else {
                    "none"
                }
            }
            _ => "none",
        }
        .to_string(),
    )
}

fn compose_display_value_summary(
    record: &PreparationDetailRowRecord,
    outcome_state: &str,
    artifact_coverage_state: Option<&str>,
) -> LibrarySqliteResult<Option<String>> {
    if outcome_state == "missing" {
        return Ok(None);
    }

    match record.capability_kind.as_str() {
        PREPARATION_DETAIL_CAPABILITY_TEMPO => read_tempo_summary(record),
        PREPARATION_DETAIL_CAPABILITY_MUSICAL_KEY => read_musical_key_summary(record),
        PREPARATION_DETAIL_CAPABILITY_BEATGRID => Ok(Some("Ready".to_string())),
        PREPARATION_DETAIL_CAPABILITY_WAVEFORM => Ok(match artifact_coverage_state {
            Some("overview_ready") => Some("Overview ready".to_string()),
            Some("refinement_available") => Some("Refinement available".to_string()),
            Some("stale") => Some("Stale".to_string()),
            _ => Some("Ready".to_string()),
        }),
        PREPARATION_DETAIL_CAPABILITY_STEMS => Ok(Some("Ready".to_string())),
        _ => Ok(None),
    }
}

fn read_tempo_summary(record: &PreparationDetailRowRecord) -> LibrarySqliteResult<Option<String>> {
    let Some(value) = read_inline_json_value(record)? else {
        return Ok(None);
    };
    let tempo = value
        .get("tempo_bpm")
        .or_else(|| value.get("tempo"))
        .and_then(serde_json::Value::as_f64);
    Ok(tempo.map(|tempo| format!("{tempo:.2}")))
}

fn read_musical_key_summary(
    record: &PreparationDetailRowRecord,
) -> LibrarySqliteResult<Option<String>> {
    let Some(value) = read_inline_json_value(record)? else {
        return Ok(None);
    };
    Ok(value
        .get("musical_key")
        .or_else(|| value.get("key"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string))
}

fn read_inline_json_value(
    record: &PreparationDetailRowRecord,
) -> LibrarySqliteResult<Option<serde_json::Value>> {
    let Some(payload) = record.inline_payload.as_deref() else {
        return Ok(None);
    };
    let value = serde_json::from_slice(payload).map_err(|error| {
        LibrarySqliteError::MalformedSchemaState(format!(
            "preparation detail capability {} inline payload is not valid JSON: {error}",
            record.capability_kind
        ))
    })?;
    Ok(Some(value))
}

fn compose_target_summary(record: &PreparationDetailRowRecord) -> Option<String> {
    let profile = record.target_profile_key.as_deref()?;
    Some(format!(
        "profile {profile}, quality {}, {}",
        record.target_quality.unwrap_or(0),
        record
            .target_stability_class
            .as_deref()
            .unwrap_or("provisional")
    ))
}

fn compose_explanation_summary(
    record: &PreparationDetailRowRecord,
    requirement_class: &str,
    work_state: &str,
    outcome_state: &str,
) -> Option<String> {
    if let Some(detail) = record
        .error_detail
        .as_deref()
        .or(record.blocked_reason.as_deref())
        .or(record.failure_kind.as_deref())
    {
        return Some(detail.to_string());
    }

    match (requirement_class, work_state, outcome_state) {
        ("on_demand", "not_requested", "missing") => {
            Some("available on demand; no current obligation".to_string())
        }
        ("required", "queued", _) => Some("queued in background".to_string()),
        ("required", "active", _) => Some("active in background".to_string()),
        ("required", "blocked", _) => Some("blocked before completion".to_string()),
        ("required", "failed", _) => Some("latest attempt failed".to_string()),
        ("required", _, "missing") => Some(format!("missing required {}", record.label)),
        ("required", _, "stale") => Some("existing result is stale".to_string()),
        ("required", _, "provisional") => Some("existing result is provisional".to_string()),
        ("required", _, _) if required_target_is_satisfied(record) => {
            Some("required target is satisfied".to_string())
        }
        ("required", _, _) => {
            Some("existing result does not satisfy the selected target".to_string())
        }
        _ => None,
    }
}

fn aggregate_readiness_summary(rows: &[StoreLibraryAssetPreparationDetailRow]) -> String {
    let required_rows = rows
        .iter()
        .filter(|row| row.requirement_class == "required")
        .collect::<Vec<_>>();
    if required_rows.is_empty() {
        return "not_required".to_string();
    }
    if required_rows.iter().any(|row| row.work_state == "failed") {
        return "failed".to_string();
    }
    if required_rows.iter().any(|row| row.work_state == "blocked") {
        return "blocked".to_string();
    }
    if required_rows
        .iter()
        .all(|row| row.satisfaction_state == "satisfied")
    {
        return "ready".to_string();
    }
    if required_rows
        .iter()
        .filter(|row| row.satisfaction_state == "unsatisfied")
        .all(|row| matches!(row.work_state.as_str(), "queued" | "active"))
    {
        return "preparing".to_string();
    }
    "underprepared".to_string()
}

fn group_preparation_detail_rows(
    rows: Vec<StoreLibraryAssetPreparationDetailRow>,
) -> Vec<StoreLibraryAssetPreparationDetailGroup> {
    let mut required_facts = Vec::new();
    let mut required_structures = Vec::new();
    let mut required_artifacts = Vec::new();
    let mut on_demand = Vec::new();

    for row in rows {
        if row.requirement_class == "on_demand" {
            on_demand.push(row);
        } else {
            match row.outcome_kind.as_str() {
                "fact" => required_facts.push(row),
                "structure" => required_structures.push(row),
                "artifact" => required_artifacts.push(row),
                _ => {}
            }
        }
    }

    vec![
        StoreLibraryAssetPreparationDetailGroup {
            group_key: "required_facts".to_string(),
            rows: required_facts,
        },
        StoreLibraryAssetPreparationDetailGroup {
            group_key: "required_structures".to_string(),
            rows: required_structures,
        },
        StoreLibraryAssetPreparationDetailGroup {
            group_key: "required_artifacts".to_string(),
            rows: required_artifacts,
        },
        StoreLibraryAssetPreparationDetailGroup {
            group_key: "on_demand_capabilities".to_string(),
            rows: on_demand,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::{PREPARATION_DETAIL_CAPABILITY_WAVEFORM, read_library_asset_preparation_detail};
    use crate::schema;
    use rusqlite::{Connection, params};

    fn connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("open in-memory sqlite");
        schema::install_baseline_schema_for_test(&mut connection).expect("install baseline");
        connection
    }

    fn insert_asset(connection: &Connection, library_asset_id: i64) {
        connection
            .execute(
                "INSERT INTO LibraryAssets (
                     library_asset_id,
                     equivalence_fingerprint,
                     retention_policy,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, 'keep_metadata', 1, 1)",
                params![
                    library_asset_id,
                    format!("eq:prep-detail:{library_asset_id}")
                ],
            )
            .expect("insert library asset");
    }

    fn insert_policy(connection: &Connection, prep_policy_id: i64) {
        connection
            .execute(
                "INSERT INTO PrepPolicies (
                     prep_policy_id,
                     policy_name,
                     is_system_policy,
                     is_user_editable,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, 1, 0, 1, 1)",
                params![prep_policy_id, format!("Policy {prep_policy_id}")],
            )
            .expect("insert prep policy");
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_target(
        connection: &Connection,
        library_asset_id: i64,
        capability_kind: &str,
        profile: &str,
        quality: i64,
        stability: &str,
        priority: &str,
        policy_id: i64,
    ) {
        connection
            .execute(
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
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 10)",
                params![
                    library_asset_id,
                    capability_kind,
                    profile,
                    quality,
                    stability,
                    priority,
                    policy_id
                ],
            )
            .expect("insert resolved target");
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_capability(
        connection: &Connection,
        library_asset_id: i64,
        capability_kind: &str,
        profile: &str,
        state: &str,
        stability: Option<&str>,
        quality: Option<i64>,
        selected_artifact_id: Option<i64>,
    ) {
        connection
            .execute(
                "INSERT INTO LibraryAssetCapabilities (
                     library_asset_id,
                     capability_kind,
                     profile_key,
                     state,
                     stability_class,
                     quality_current,
                     basis_fingerprint,
                     selected_artifact_id,
                     updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 20)",
                params![
                    library_asset_id,
                    capability_kind,
                    profile,
                    state,
                    stability,
                    quality,
                    stability.map(|_| "basis:test"),
                    selected_artifact_id
                ],
            )
            .expect("insert capability");
    }

    fn insert_work_item(
        connection: &Connection,
        library_asset_id: i64,
        capability_kind: &str,
        profile: &str,
        quality: i64,
        state: &str,
    ) {
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
                     created_at,
                     updated_at
                 )
                 VALUES ('library_asset', ?1, 'compute_capability', ?2, ?3, ?4,
                         'background', ?5, ?6, ?7, 0, 1, 2)",
                params![
                    library_asset_id.to_string(),
                    capability_kind,
                    profile,
                    quality,
                    format!("basis:{library_asset_id}:{capability_kind}:{profile}:{quality}"),
                    state,
                    if state == "leased" {
                        Some(100_i64)
                    } else {
                        None
                    }
                ],
            )
            .expect("insert work item");
    }

    #[test]
    fn missing_asset_returns_no_preparation_detail() {
        let connection = connection();
        let detail =
            read_library_asset_preparation_detail(&connection, 404).expect("read preparation");

        assert!(detail.is_none());
    }

    #[test]
    fn selected_asset_preparation_detail_groups_initial_rows_in_lawful_order() {
        let connection = connection();
        insert_asset(&connection, 1);
        insert_policy(&connection, 1);
        insert_target(
            &connection,
            1,
            "tempo",
            "default",
            0,
            "stable",
            "background",
            1,
        );
        insert_target(
            &connection,
            1,
            "beatgrid",
            "default",
            0,
            "stable",
            "background",
            1,
        );
        insert_target(
            &connection,
            1,
            "waveform",
            "default",
            90,
            "stable",
            "background",
            1,
        );

        let detail = read_library_asset_preparation_detail(&connection, 1)
            .expect("read preparation")
            .expect("asset exists");

        assert_eq!(detail.library_asset_id, 1);
        assert_eq!(
            detail
                .groups
                .iter()
                .map(|group| group.group_key.as_str())
                .collect::<Vec<_>>(),
            vec![
                "required_facts",
                "required_structures",
                "required_artifacts",
                "on_demand_capabilities"
            ]
        );
        assert_eq!(
            detail
                .groups
                .iter()
                .flat_map(|group| group.rows.iter())
                .map(|row| row.capability_kind.as_str())
                .collect::<Vec<_>>(),
            vec!["tempo", "beatgrid", "waveform", "musical_key", "stems"]
        );

        let required_fact = &detail.groups[0].rows[0];
        assert_eq!(required_fact.outcome_kind, "fact");
        assert_eq!(required_fact.requirement_class, "required");

        let required_structure = &detail.groups[1].rows[0];
        assert_eq!(required_structure.capability_kind, "beatgrid");
        assert_eq!(required_structure.outcome_kind, "structure");

        let waveform = &detail.groups[2].rows[0];
        assert_eq!(
            waveform.capability_kind,
            PREPARATION_DETAIL_CAPABILITY_WAVEFORM
        );
        assert_eq!(waveform.outcome_kind, "artifact");
        assert_eq!(waveform.artifact_coverage_state.as_deref(), Some("none"));
        assert!(waveform.progress.is_none());

        assert_eq!(
            detail.groups[3]
                .rows
                .iter()
                .map(|row| (row.capability_kind.as_str(), row.requirement_class.as_str()))
                .collect::<Vec<_>>(),
            vec![("musical_key", "on_demand"), ("stems", "on_demand")]
        );
    }

    #[test]
    fn deterministic_target_collapse_prefers_quality_without_scheduler_priority_authority() {
        let connection = connection();
        insert_asset(&connection, 1);
        insert_policy(&connection, 1);
        insert_policy(&connection, 2);
        insert_target(
            &connection,
            1,
            "waveform",
            "hi-quality-background",
            100,
            "stable",
            "background",
            1,
        );
        insert_target(
            &connection,
            1,
            "waveform",
            "urgent-lower-quality",
            80,
            "stable",
            "urgent",
            2,
        );

        let detail = read_library_asset_preparation_detail(&connection, 1)
            .expect("read preparation")
            .expect("asset exists");
        let waveform = detail.groups[2]
            .rows
            .iter()
            .find(|row| row.capability_kind == "waveform")
            .expect("waveform row");

        assert_eq!(
            waveform.target_summary.as_deref(),
            Some("profile hi-quality-background, quality 100, stable")
        );
    }

    #[test]
    fn deterministic_target_collapse_prefers_stable_when_quality_ties() {
        let connection = connection();
        insert_asset(&connection, 1);
        insert_policy(&connection, 1);
        insert_policy(&connection, 2);
        insert_target(
            &connection,
            1,
            "waveform",
            "provisional-urgent",
            90,
            "provisional",
            "urgent",
            1,
        );
        insert_target(
            &connection,
            1,
            "waveform",
            "stable-background",
            90,
            "stable",
            "background",
            2,
        );

        let detail = read_library_asset_preparation_detail(&connection, 1)
            .expect("read preparation")
            .expect("asset exists");
        let waveform = detail.groups[2]
            .rows
            .iter()
            .find(|row| row.capability_kind == "waveform")
            .expect("waveform row");

        assert_eq!(
            waveform.target_summary.as_deref(),
            Some("profile stable-background, quality 90, stable")
        );
    }

    #[test]
    fn deterministic_target_collapse_prefers_default_profile_when_quality_and_stability_tie() {
        let connection = connection();
        insert_asset(&connection, 1);
        insert_policy(&connection, 1);
        insert_policy(&connection, 2);
        insert_target(
            &connection,
            1,
            "waveform",
            "aaa-non-default",
            90,
            "stable",
            "urgent",
            1,
        );
        insert_target(
            &connection,
            1,
            "waveform",
            "default",
            90,
            "stable",
            "background",
            2,
        );

        let detail = read_library_asset_preparation_detail(&connection, 1)
            .expect("read preparation")
            .expect("asset exists");
        let waveform = detail.groups[2]
            .rows
            .iter()
            .find(|row| row.capability_kind == "waveform")
            .expect("waveform row");

        assert_eq!(
            waveform.target_summary.as_deref(),
            Some("profile default, quality 90, stable")
        );
    }

    #[test]
    fn deterministic_target_collapse_uses_lexical_profile_key_as_final_tie_break() {
        let connection = connection();
        insert_asset(&connection, 1);
        insert_policy(&connection, 1);
        insert_policy(&connection, 2);
        insert_target(
            &connection,
            1,
            "waveform",
            "zeta",
            90,
            "stable",
            "urgent",
            1,
        );
        insert_target(
            &connection,
            1,
            "waveform",
            "alpha",
            90,
            "stable",
            "background",
            2,
        );

        let detail = read_library_asset_preparation_detail(&connection, 1)
            .expect("read preparation")
            .expect("asset exists");
        let waveform = detail.groups[2]
            .rows
            .iter()
            .find(|row| row.capability_kind == "waveform")
            .expect("waveform row");

        assert_eq!(
            waveform.target_summary.as_deref(),
            Some("profile alpha, quality 90, stable")
        );
    }

    #[test]
    fn aggregate_readiness_uses_existing_browser_vocabulary() {
        let connection = connection();
        insert_asset(&connection, 1);
        insert_policy(&connection, 1);
        insert_target(
            &connection,
            1,
            "tempo",
            "default",
            0,
            "stable",
            "background",
            1,
        );
        insert_work_item(&connection, 1, "tempo", "default", 0, "queued");

        let detail = read_library_asset_preparation_detail(&connection, 1)
            .expect("read preparation")
            .expect("asset exists");

        assert_eq!(detail.aggregate_readiness_summary, "preparing");
    }

    #[test]
    fn on_demand_is_explicit_and_not_collapsed_into_missing() {
        let connection = connection();
        insert_asset(&connection, 1);

        let detail = read_library_asset_preparation_detail(&connection, 1)
            .expect("read preparation")
            .expect("asset exists");

        assert_eq!(detail.aggregate_readiness_summary, "not_required");
        assert!(detail.groups[..3].iter().all(|group| group.rows.is_empty()));
        assert_eq!(detail.groups[3].rows.len(), 5);
        assert!(detail.groups[3].rows.iter().all(|row| {
            row.requirement_class == "on_demand"
                && row.work_state == "not_requested"
                && row.outcome_state == "missing"
        }));
    }

    #[test]
    fn required_ready_capability_satisfies_selected_target() {
        let connection = connection();
        insert_asset(&connection, 1);
        insert_policy(&connection, 1);
        insert_target(
            &connection,
            1,
            "stems",
            "default",
            90,
            "stable",
            "background",
            1,
        );
        insert_capability(
            &connection,
            1,
            "stems",
            "default",
            "ready",
            Some("stable"),
            Some(100),
            None,
        );

        let detail = read_library_asset_preparation_detail(&connection, 1)
            .expect("read preparation")
            .expect("asset exists");
        let stems = detail.groups[2]
            .rows
            .iter()
            .find(|row| row.capability_kind == "stems")
            .expect("stems row");

        assert_eq!(stems.outcome_kind, "artifact");
        assert_eq!(stems.outcome_state, "ready");
        assert_eq!(stems.satisfaction_state, "satisfied");
        assert_eq!(detail.aggregate_readiness_summary, "ready");
    }
}
