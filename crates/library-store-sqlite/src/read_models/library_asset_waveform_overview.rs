use std::fs;

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use crate::authority::work::ArtifactFileStoreRoot;
use crate::authority::work::file_store::ArtifactFileStoreRelativePath;
use crate::read_models::waveform_profile_selection::RELEVANT_WAVEFORM_TARGET_ORDER_SQL;
use crate::source_media::AppOwnedStorageKind;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::ArtifactStorageKind;

const MAX_OVERVIEW_BUCKETS: usize = 4_096;
const MAX_OVERVIEW_PAYLOAD_BYTES: usize = 1_048_576;
pub(crate) const LIBRARY_ASSET_WAVEFORM_OVERVIEW_MEDIA_TYPE: &str =
    "application/vnd.music-library.library-asset-waveform-overview+json";
const LIBRARY_ASSET_WAVEFORM_OVERVIEW_SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreLibraryAssetWaveformOverviewCapabilityState {
    Ready,
    Stale,
}

impl StoreLibraryAssetWaveformOverviewCapabilityState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Stale => "stale",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "ready" => Some(Self::Ready),
            "stale" => Some(Self::Stale),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreLibraryAssetWaveformOverviewAmplitudeScale {
    SignedI16,
}

impl StoreLibraryAssetWaveformOverviewAmplitudeScale {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SignedI16 => "signed_i16",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreLibraryAssetWaveformOverviewBucket {
    pub min_amplitude_i16: i16,
    pub max_amplitude_i16: i16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoreLibraryAssetWaveformOverview {
    pub source_profile_key: String,
    pub source_quality_current: Option<i64>,
    pub capability_state: StoreLibraryAssetWaveformOverviewCapabilityState,
    pub amplitude_scale: StoreLibraryAssetWaveformOverviewAmplitudeScale,
    pub bucket_count: usize,
    pub duration_ms: Option<i64>,
    pub source_sample_count: Option<i64>,
    pub samples_per_bucket: Option<i64>,
    pub buckets: Vec<StoreLibraryAssetWaveformOverviewBucket>,
    pub accepted_artifact_id: i64,
    pub basis_fingerprint: String,
    pub capability_updated_at: i64,
    pub artifact_created_at: i64,
}

#[derive(Debug, Clone, PartialEq)]
struct AcceptedWaveformArtifactRow {
    profile_key: String,
    capability_state: StoreLibraryAssetWaveformOverviewCapabilityState,
    quality_current: Option<i64>,
    basis_fingerprint: String,
    selected_artifact_id: i64,
    capability_updated_at: i64,
    artifact_created_at: i64,
    media_type: String,
    storage_kind: ArtifactStorageKind,
    inline_payload: Option<Vec<u8>>,
    file_store_root_kind: Option<String>,
    file_store_relative_path: Option<String>,
    file_store_payload_bytes: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
struct DecodedLibraryAssetWaveformOverview {
    schema_version: i64,
    amplitude_scale: StoreLibraryAssetWaveformOverviewAmplitudeScale,
    bucket_count: usize,
    duration_ms: Option<i64>,
    source_sample_count: Option<i64>,
    samples_per_bucket: Option<i64>,
    buckets: Vec<StoreLibraryAssetWaveformOverviewBucket>,
}

pub fn read_library_asset_waveform_overview(
    connection: &Connection,
    file_store_root: &ArtifactFileStoreRoot,
    library_asset_id: i64,
) -> LibrarySqliteResult<Option<StoreLibraryAssetWaveformOverview>> {
    let query = format!(
        "WITH capability_defaults AS (
             SELECT MAX(CASE WHEN capability_kind = 'waveform' THEN default_profile_key END) AS waveform_default_profile_key
             FROM CapabilitySpecs
         ),
         waveform_target_candidates AS (
             SELECT rpt.library_asset_id,
                    rpt.target_profile_key,
                    rpt.target_quality,
                    ROW_NUMBER() OVER (
                        PARTITION BY rpt.library_asset_id
                        ORDER BY {RELEVANT_WAVEFORM_TARGET_ORDER_SQL}
                    ) AS target_rank
             FROM ResolvedLibraryAssetPrepTargets rpt
             CROSS JOIN capability_defaults
             WHERE rpt.capability_kind = 'waveform'
               AND rpt.library_asset_id = ?1
         ),
         waveform_targets AS (
             SELECT library_asset_id,
                    target_profile_key
             FROM waveform_target_candidates
             WHERE target_rank = 1
         ),
         waveform_profile AS (
             SELECT pi.library_asset_id,
                    COALESCE(
                        waveform_targets.target_profile_key,
                        capability_defaults.waveform_default_profile_key
                    ) AS profile_key
             FROM LibraryAssets pi
             CROSS JOIN capability_defaults
             LEFT JOIN waveform_targets
               ON waveform_targets.library_asset_id = pi.library_asset_id
             WHERE pi.library_asset_id = ?1
         )
         SELECT pic.profile_key,
                pic.state,
                pic.quality_current,
                pic.basis_fingerprint,
                pic.selected_artifact_id,
                pic.updated_at,
                artifacts.created_at,
                artifacts.media_type,
                artifacts.storage_kind,
                inline_payload.payload,
                file_entry.root_kind,
                file_entry.relative_path,
                file_entry.payload_bytes
         FROM waveform_profile
         JOIN LibraryAssetCapabilities pic
           ON pic.library_asset_id = waveform_profile.library_asset_id
          AND pic.capability_kind = 'waveform'
          AND pic.profile_key = waveform_profile.profile_key
          AND pic.state IN ('ready', 'stale')
         JOIN Artifacts artifacts
           ON artifacts.artifact_id = pic.selected_artifact_id
          AND artifacts.subject_kind = 'library_asset'
          AND artifacts.subject_id = CAST(pic.library_asset_id AS TEXT)
          AND artifacts.capability_kind = 'waveform'
          AND artifacts.profile_key = pic.profile_key
          AND artifacts.artifact_kind = 'capability_result'
          AND artifacts.artifact_role = 'primary_result'
          AND artifacts.basis_fingerprint = pic.basis_fingerprint
         LEFT JOIN ArtifactInlinePayloads inline_payload
           ON inline_payload.artifact_id = artifacts.artifact_id
         LEFT JOIN ArtifactFileStoreEntries file_entry
           ON file_entry.artifact_id = artifacts.artifact_id"
    );

    let Some(accepted_artifact) = connection
        .query_row(
            &query,
            params![library_asset_id],
            accepted_artifact_from_row,
        )
        .optional()?
    else {
        return Ok(None);
    };

    if !is_json_overview_media_type(&accepted_artifact.media_type) {
        return Ok(None);
    }

    let Some(payload) = load_artifact_payload(file_store_root, &accepted_artifact)? else {
        return Ok(None);
    };
    let decoded = decode_library_asset_waveform_overview_payload(
        accepted_artifact.selected_artifact_id,
        &payload,
    )?;
    debug_assert_eq!(
        decoded.schema_version,
        LIBRARY_ASSET_WAVEFORM_OVERVIEW_SCHEMA_VERSION
    );

    Ok(Some(StoreLibraryAssetWaveformOverview {
        source_profile_key: accepted_artifact.profile_key,
        source_quality_current: accepted_artifact.quality_current,
        capability_state: accepted_artifact.capability_state,
        amplitude_scale: decoded.amplitude_scale,
        bucket_count: decoded.bucket_count,
        duration_ms: decoded.duration_ms,
        source_sample_count: decoded.source_sample_count,
        samples_per_bucket: decoded.samples_per_bucket,
        buckets: decoded.buckets,
        accepted_artifact_id: accepted_artifact.selected_artifact_id,
        basis_fingerprint: accepted_artifact.basis_fingerprint,
        capability_updated_at: accepted_artifact.capability_updated_at,
        artifact_created_at: accepted_artifact.artifact_created_at,
    }))
}

fn accepted_artifact_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<AcceptedWaveformArtifactRow> {
    let state: String = row.get(1)?;
    let storage_kind: String = row.get(8)?;
    let capability_state = StoreLibraryAssetWaveformOverviewCapabilityState::parse(&state)
        .ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                1,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unsupported waveform capability state {state:?}"),
                )),
            )
        })?;
    let storage_kind = ArtifactStorageKind::parse(&storage_kind).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            8,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unsupported artifact storage kind {storage_kind:?}"),
            )),
        )
    })?;

    Ok(AcceptedWaveformArtifactRow {
        profile_key: row.get(0)?,
        capability_state,
        quality_current: row.get(2)?,
        basis_fingerprint: row.get(3)?,
        selected_artifact_id: row.get(4)?,
        capability_updated_at: row.get(5)?,
        artifact_created_at: row.get(6)?,
        media_type: row.get(7)?,
        storage_kind,
        inline_payload: row.get(9)?,
        file_store_root_kind: row.get(10)?,
        file_store_relative_path: row.get(11)?,
        file_store_payload_bytes: row.get(12)?,
    })
}

fn is_json_overview_media_type(media_type: &str) -> bool {
    media_type == LIBRARY_ASSET_WAVEFORM_OVERVIEW_MEDIA_TYPE
}

fn load_artifact_payload(
    file_store_root: &ArtifactFileStoreRoot,
    artifact: &AcceptedWaveformArtifactRow,
) -> LibrarySqliteResult<Option<Vec<u8>>> {
    match artifact.storage_kind {
        ArtifactStorageKind::InlinePayload => {
            let payload = artifact.inline_payload.clone().ok_or_else(|| {
                LibrarySqliteError::MalformedSchemaState(format!(
                    "inline waveform artifact {} is missing ArtifactInlinePayloads",
                    artifact.selected_artifact_id
                ))
            })?;
            if payload.len() > MAX_OVERVIEW_PAYLOAD_BYTES {
                return Ok(None);
            }
            Ok(Some(payload))
        }
        ArtifactStorageKind::FileStore => {
            let root_kind = artifact.file_store_root_kind.as_deref().ok_or_else(|| {
                LibrarySqliteError::MalformedSchemaState(format!(
                    "file-backed waveform artifact {} is missing root_kind",
                    artifact.selected_artifact_id
                ))
            })?;
            let root_kind = AppOwnedStorageKind::from_db_value(root_kind).ok_or_else(|| {
                LibrarySqliteError::MalformedSchemaState(format!(
                    "waveform artifact {} uses unsupported artifact file-store root kind {root_kind:?}",
                    artifact.selected_artifact_id
                ))
            })?;
            if root_kind != file_store_root.kind() {
                return Err(LibrarySqliteError::MalformedSchemaState(format!(
                    "waveform artifact {} uses unsupported artifact file-store root kind {}",
                    artifact.selected_artifact_id,
                    root_kind.as_str()
                )));
            }
            let relative_path = artifact.file_store_relative_path.clone().ok_or_else(|| {
                LibrarySqliteError::MalformedSchemaState(format!(
                    "file-backed waveform artifact {} is missing relative_path",
                    artifact.selected_artifact_id
                ))
            })?;
            let payload_bytes = artifact.file_store_payload_bytes.ok_or_else(|| {
                LibrarySqliteError::MalformedSchemaState(format!(
                    "file-backed waveform artifact {} is missing payload_bytes",
                    artifact.selected_artifact_id
                ))
            })?;
            if usize::try_from(payload_bytes)
                .map(|bytes| bytes > MAX_OVERVIEW_PAYLOAD_BYTES)
                .unwrap_or(true)
            {
                return Ok(None);
            }
            let relative_path = ArtifactFileStoreRelativePath::from_db_value(
                relative_path,
                format!("waveform artifact {}", artifact.selected_artifact_id),
            )?;
            let absolute_path = file_store_root.resolve_relative_path(&relative_path);
            let payload = fs::read(&absolute_path).map_err(|source| {
                LibrarySqliteError::ArtifactFileStoreIo {
                    action: "read waveform artifact payload",
                    path: absolute_path,
                    source,
                }
            })?;
            if payload.len() > MAX_OVERVIEW_PAYLOAD_BYTES {
                return Ok(None);
            }
            Ok(Some(payload))
        }
    }
}

fn decode_library_asset_waveform_overview_payload(
    artifact_id: i64,
    payload: &[u8],
) -> LibrarySqliteResult<DecodedLibraryAssetWaveformOverview> {
    let value: Value = serde_json::from_slice(payload).map_err(|error| {
        LibrarySqliteError::MalformedSchemaState(format!(
            "waveform artifact {artifact_id} overview payload is not valid JSON: {error}"
        ))
    })?;

    if !value.is_object() {
        return Err(malformed_overview(
            artifact_id,
            "top-level payload must be a JSON object",
        ));
    }
    if value.get("library_asset_waveform_overview").is_some() {
        return Err(malformed_overview(
            artifact_id,
            "library_asset_waveform_overview wrapper is not supported; expected a flat top-level overview object",
        ));
    }

    let schema_version = required_schema_version(artifact_id, &value)?;
    let overview = &value;
    let bucket_count = required_usize(artifact_id, overview, "bucket_count")?;
    if bucket_count == 0 || bucket_count > MAX_OVERVIEW_BUCKETS {
        return Err(malformed_overview(
            artifact_id,
            format!(
                "bucket_count must be between 1 and {MAX_OVERVIEW_BUCKETS}, found {bucket_count}"
            ),
        ));
    }
    let amplitude_scale = match required_str(artifact_id, overview, "amplitude_scale")? {
        "signed_i16" => StoreLibraryAssetWaveformOverviewAmplitudeScale::SignedI16,
        other => {
            return Err(malformed_overview(
                artifact_id,
                format!("unsupported amplitude_scale {other:?}"),
            ));
        }
    };
    let duration_ms = optional_nonnegative_i64(artifact_id, overview, "duration_ms")?;
    let source_sample_count =
        optional_nonnegative_i64(artifact_id, overview, "source_sample_count")?;
    let samples_per_bucket = optional_nonnegative_i64(artifact_id, overview, "samples_per_bucket")?;
    let buckets = required_buckets(artifact_id, overview)?;
    if bucket_count != buckets.len() {
        return Err(malformed_overview(
            artifact_id,
            format!(
                "bucket_count {bucket_count} does not match buckets length {}",
                buckets.len()
            ),
        ));
    }

    Ok(DecodedLibraryAssetWaveformOverview {
        schema_version,
        amplitude_scale,
        bucket_count,
        duration_ms,
        source_sample_count,
        samples_per_bucket,
        buckets,
    })
}

fn required_schema_version(artifact_id: i64, overview: &Value) -> LibrarySqliteResult<i64> {
    let schema_version = required_nonnegative_i64(artifact_id, overview, "schema_version")?;
    if schema_version != LIBRARY_ASSET_WAVEFORM_OVERVIEW_SCHEMA_VERSION {
        return Err(malformed_overview(
            artifact_id,
            format!(
                "unsupported schema_version {schema_version}; supported schema_version is {LIBRARY_ASSET_WAVEFORM_OVERVIEW_SCHEMA_VERSION}"
            ),
        ));
    }
    Ok(schema_version)
}

fn required_buckets(
    artifact_id: i64,
    overview: &Value,
) -> LibrarySqliteResult<Vec<StoreLibraryAssetWaveformOverviewBucket>> {
    let buckets = overview
        .get("buckets")
        .and_then(Value::as_array)
        .ok_or_else(|| malformed_overview(artifact_id, "buckets must be an array"))?;
    if buckets.len() > MAX_OVERVIEW_BUCKETS {
        return Err(malformed_overview(
            artifact_id,
            format!(
                "buckets length {} exceeds maximum {MAX_OVERVIEW_BUCKETS}",
                buckets.len()
            ),
        ));
    }

    buckets
        .iter()
        .enumerate()
        .map(|(index, bucket)| {
            let min_amplitude_i16 =
                required_i16(artifact_id, bucket, "min_amplitude_i16", index)?;
            let max_amplitude_i16 =
                required_i16(artifact_id, bucket, "max_amplitude_i16", index)?;
            if min_amplitude_i16 > max_amplitude_i16 {
                return Err(malformed_overview(
                    artifact_id,
                    format!(
                        "bucket {index} min_amplitude_i16 {min_amplitude_i16} exceeds max_amplitude_i16 {max_amplitude_i16}"
                    ),
                ));
            }
            Ok(StoreLibraryAssetWaveformOverviewBucket {
                min_amplitude_i16,
                max_amplitude_i16,
            })
        })
        .collect()
}

fn required_usize(artifact_id: i64, overview: &Value, field: &str) -> LibrarySqliteResult<usize> {
    let value = required_nonnegative_i64(artifact_id, overview, field)?;
    usize::try_from(value).map_err(|_| {
        malformed_overview(
            artifact_id,
            format!("{field} {value} exceeds supported integer range"),
        )
    })
}

fn required_nonnegative_i64(
    artifact_id: i64,
    overview: &Value,
    field: &str,
) -> LibrarySqliteResult<i64> {
    let Some(value) = overview.get(field) else {
        return Err(malformed_overview(
            artifact_id,
            format!("{field} is required"),
        ));
    };
    let Some(value) = value.as_i64() else {
        return Err(malformed_overview(
            artifact_id,
            format!("{field} must be an integer"),
        ));
    };
    if value < 0 {
        return Err(malformed_overview(
            artifact_id,
            format!("{field} must be nonnegative, found {value}"),
        ));
    }
    Ok(value)
}

fn optional_nonnegative_i64(
    artifact_id: i64,
    overview: &Value,
    field: &str,
) -> LibrarySqliteResult<Option<i64>> {
    let Some(value) = overview.get(field) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let Some(value) = value.as_i64() else {
        return Err(malformed_overview(
            artifact_id,
            format!("{field} must be an integer"),
        ));
    };
    if value < 0 {
        return Err(malformed_overview(
            artifact_id,
            format!("{field} must be nonnegative, found {value}"),
        ));
    }
    Ok(Some(value))
}

fn required_i16(
    artifact_id: i64,
    bucket: &Value,
    field: &str,
    bucket_index: usize,
) -> LibrarySqliteResult<i16> {
    let Some(value) = bucket.get(field).and_then(Value::as_i64) else {
        return Err(malformed_overview(
            artifact_id,
            format!("bucket {bucket_index} {field} must be an integer"),
        ));
    };
    i16::try_from(value).map_err(|_| {
        malformed_overview(
            artifact_id,
            format!("bucket {bucket_index} {field} {value} exceeds i16 range"),
        )
    })
}

fn required_str<'a>(
    artifact_id: i64,
    overview: &'a Value,
    field: &str,
) -> LibrarySqliteResult<&'a str> {
    overview
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| malformed_overview(artifact_id, format!("{field} must be a string")))
}

fn malformed_overview(artifact_id: i64, detail: impl Into<String>) -> LibrarySqliteError {
    LibrarySqliteError::MalformedSchemaState(format!(
        "waveform artifact {artifact_id} overview payload is malformed: {}",
        detail.into()
    ))
}
