use std::fmt::Write as _;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::authority::write_lane::AdmittedWrite;
use crate::source_media::{
    open_source_media_file_for_read, SourceMediaOperation, SourceMediaWritePolicy,
};
use crate::work_control::{validate_mount_epoch_stamp, MountEpochStamp, SourceAdmissionToken};
use crate::{LibrarySqliteError, LibrarySqliteResult};

const INSPECTION_HEADER_BYTES: usize = 4 * 1024;
const INSPECTION_READ_BUFFER_BYTES: usize = 8 * 1024;
const MEDIA_INSPECTION_LEASE_MS: i64 = 30_000;
const MEDIA_INSPECTION_SOURCE_MEDIA_WRITE_POLICY: SourceMediaWritePolicy =
    SourceMediaWritePolicy::read_only(SourceMediaOperation::MediaInspection);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaInspectionCommitRequest {
    pub file_id: i64,
    pub stamp: MountEpochStamp,
    pub started_at_ms: i64,
    pub finished_at_ms: i64,
    pub outcome: MediaInspectionOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaInspectionOutcome {
    Succeeded(MediaInspectionSuccess),
    Failed { error_detail: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaInspectionSuccess {
    pub media_kind: String,
    pub mime_type: String,
    pub width_px: Option<i64>,
    pub height_px: Option<i64>,
    pub duration_ms: Option<i64>,
    pub orientation_degrees: Option<i64>,
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaInspectionCommitResult {
    pub media_run_id: i64,
    pub file_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EligibleVisualMediaType {
    Png,
    Jpeg,
    Gif,
    Mp4,
    Mov,
    M4v,
    Webm,
}

type MediaHeaderInspection = Result<MediaHeaderFacts, &'static str>;
type MediaHeaderFacts = (Option<i64>, Option<i64>, Option<i64>, Option<i64>);

impl EligibleVisualMediaType {
    fn from_path(path: &str) -> Option<Self> {
        let extension = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();

        match extension.as_str() {
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "gif" => Some(Self::Gif),
            "mp4" => Some(Self::Mp4),
            "mov" => Some(Self::Mov),
            "m4v" => Some(Self::M4v),
            "webm" => Some(Self::Webm),
            _ => None,
        }
    }

    const fn media_kind(self) -> &'static str {
        match self {
            Self::Png | Self::Jpeg | Self::Gif => "image",
            Self::Mp4 | Self::Mov | Self::M4v | Self::Webm => "video",
        }
    }

    const fn mime_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::Mp4 => "video/mp4",
            Self::Mov => "video/quicktime",
            Self::M4v => "video/x-m4v",
            Self::Webm => "video/webm",
        }
    }

    fn inspect_header(self, header: &[u8]) -> MediaHeaderInspection {
        match self {
            Self::Png => inspect_png_header(header),
            Self::Jpeg => inspect_jpeg_header(header),
            Self::Gif => inspect_gif_header(header),
            Self::Mp4 | Self::Mov | Self::M4v => inspect_iso_bmff_header(header),
            Self::Webm => inspect_webm_header(header),
        }
    }
}

pub(crate) fn is_eligible_visual_media_path(path: &str) -> bool {
    EligibleVisualMediaType::from_path(path).is_some()
}

pub(crate) fn build_filesystem_media_inspection_request(
    file_id: i64,
    file_path: &Path,
    canonical_path: &str,
) -> MediaInspectionCommitRequest {
    build_filesystem_media_inspection_request_with_context(
        file_id,
        file_path,
        canonical_path,
        MountEpochStamp {
            root_id: 0,
            mount_epoch: 0,
        },
        None,
    )
    .expect("default media inspection builder must not be cancelled")
}

pub(crate) fn build_filesystem_media_inspection_request_with_context(
    file_id: i64,
    file_path: &Path,
    canonical_path: &str,
    stamp: MountEpochStamp,
    cancellation: Option<&SourceAdmissionToken>,
) -> Result<MediaInspectionCommitRequest, LibrarySqliteError> {
    let started_at_ms = unix_time_ms_now();
    let outcome = filesystem_media_inspection_outcome(
        file_path,
        canonical_path,
        stamp.root_id,
        cancellation,
    )?;
    let finished_at_ms = unix_time_ms_now().max(started_at_ms);

    Ok(MediaInspectionCommitRequest {
        file_id,
        stamp,
        started_at_ms,
        finished_at_ms,
        outcome,
    })
}

pub(crate) struct MediaInspectTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> MediaInspectTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub(crate) fn inspect_file_from_library(
        &mut self,
        file_id: i64,
        stamp: MountEpochStamp,
        cancellation: Option<&SourceAdmissionToken>,
    ) -> LibrarySqliteResult<MediaInspectionCommitResult> {
        let started_at_ms = crate::time::unix_time_ms()?;
        self.mark_media_state_leased(file_id, started_at_ms)?;
        let (root_id, root_path, relative_path) = self.require_file_storage_path(file_id)?;
        if cancellation.is_some_and(SourceAdmissionToken::is_cancelled) {
            return Err(LibrarySqliteError::RootWorkCancelled { root_id });
        }
        let file_path = root_path.join(Path::new(&relative_path));
        let request = match build_filesystem_media_inspection_request_with_context(
            file_id,
            &file_path,
            &relative_path,
            stamp,
            cancellation,
        ) {
            Ok(request) => request,
            Err(error @ LibrarySqliteError::RootWorkCancelled { .. })
            | Err(error @ LibrarySqliteError::StaleMountEpochStamp { .. }) => return Err(error),
            Err(other) => return Err(other),
        };
        match self.commit_media_inspection(&request) {
            Ok(result) => Ok(result),
            Err(LibrarySqliteError::RootWorkCancelled { root_id }) => {
                Err(LibrarySqliteError::RootWorkCancelled { root_id })
            }
            Err(error @ LibrarySqliteError::StaleMountEpochStamp { .. }) => Err(error),
            Err(LibrarySqliteError::MissingFileRootPath { .. }) => {
                Err(LibrarySqliteError::MissingFileRootPath { file_id, root_id })
            }
            Err(other) => Err(other),
        }
    }

    pub(crate) fn commit_media_inspection(
        &mut self,
        request: &MediaInspectionCommitRequest,
    ) -> LibrarySqliteResult<MediaInspectionCommitResult> {
        self.require_file_exists(request.file_id)?;
        validate_mount_epoch_stamp(self.tx(), request.stamp)?;
        let media_run_id = self.insert_media_run(request)?;

        match &request.outcome {
            MediaInspectionOutcome::Failed { error_detail } => {
                self.upsert_file_media_state(
                    request.file_id,
                    "failed",
                    media_run_id,
                    error_detail.as_deref(),
                    request.finished_at_ms,
                )?;
            }
            MediaInspectionOutcome::Succeeded(media) => {
                self.upsert_file_media_latest(
                    request.file_id,
                    media_run_id,
                    media,
                    request.finished_at_ms,
                )?;
                self.upsert_file_media_state(
                    request.file_id,
                    "succeeded",
                    media_run_id,
                    None,
                    request.finished_at_ms,
                )?;
            }
        }

        Ok(MediaInspectionCommitResult {
            media_run_id,
            file_id: request.file_id,
        })
    }

    fn tx(&self) -> &AdmittedWrite<'conn> {
        self.tx
    }

    fn require_file_exists(&self, file_id: i64) -> LibrarySqliteResult<()> {
        let exists = self.tx().query_row(
            "SELECT EXISTS(
                     SELECT 1
                     FROM source_files
                     WHERE source_file_id = ?1
                 )",
            [file_id],
            |row| row.get::<_, i64>(0),
        )?;
        if exists == 0 {
            return Err(LibrarySqliteError::MissingFile(file_id));
        }
        Ok(())
    }

    fn require_file_storage_path(
        &self,
        file_id: i64,
    ) -> LibrarySqliteResult<(i64, PathBuf, String)> {
        let file_storage = self
            .tx()
            .query_row(
                "SELECT sf.source_id,
                        COALESCE(lss.effective_path, sl.absolute_path),
                        sf.relative_path
                 FROM source_files sf
                 LEFT JOIN source_state lss
                   ON lss.source_id = sf.source_id
                 LEFT JOIN source_locators sl
                   ON sl.source_id = sf.source_id
                 WHERE sf.source_file_id = ?1",
                [file_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;
        let Some((root_id, root_path, relative_path)) = file_storage else {
            return Err(LibrarySqliteError::MissingFile(file_id));
        };
        let Some(root_path) = root_path.filter(|value| !value.is_empty()) else {
            return Err(LibrarySqliteError::MissingFileRootPath { file_id, root_id });
        };

        Ok((root_id, PathBuf::from(root_path), relative_path))
    }

    fn insert_media_run(&self, request: &MediaInspectionCommitRequest) -> LibrarySqliteResult<i64> {
        let (outcome, error_detail) = match &request.outcome {
            MediaInspectionOutcome::Succeeded(_) => ("success", None),
            MediaInspectionOutcome::Failed { error_detail } => ("failure", error_detail.as_deref()),
        };

        self.tx().execute(
            "INSERT INTO media_runs (
                 file_id,
                 started_at,
                 finished_at,
                 outcome,
                 error_detail
             )
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                request.file_id,
                request.started_at_ms,
                request.finished_at_ms,
                outcome,
                error_detail,
            ],
        )?;

        Ok(self.tx().last_insert_rowid())
    }

    fn upsert_file_media_state(
        &self,
        file_id: i64,
        state: &str,
        media_run_id: i64,
        error_detail: Option<&str>,
        updated_at_ms: i64,
    ) -> LibrarySqliteResult<()> {
        self.tx().execute(
            "INSERT INTO file_media_state (
                 file_id,
                 state,
                 leased_until,
                 attempt_count,
                 requested_at,
                 last_media_run_id,
                 last_error_detail,
                 updated_at
             )
             VALUES (?1, ?2, NULL, 1, ?3, ?4, ?5, ?3)
             ON CONFLICT(file_id) DO UPDATE
             SET state = excluded.state,
                 leased_until = NULL,
                 attempt_count = file_media_state.attempt_count + 1,
                 last_media_run_id = excluded.last_media_run_id,
                 last_error_detail = excluded.last_error_detail,
                 updated_at = excluded.updated_at",
            params![file_id, state, updated_at_ms, media_run_id, error_detail],
        )?;

        Ok(())
    }

    fn mark_media_state_leased(&self, file_id: i64, leased_at_ms: i64) -> LibrarySqliteResult<()> {
        self.tx().execute(
            "INSERT INTO file_media_state (
                 file_id,
                 state,
                 leased_until,
                 attempt_count,
                 requested_at,
                 last_media_run_id,
                 last_error_detail,
                 updated_at
             )
             VALUES (?1, 'leased', ?2, 0, ?3, NULL, NULL, ?3)
             ON CONFLICT(file_id) DO UPDATE
             SET state = 'leased',
                 leased_until = excluded.leased_until,
                 updated_at = excluded.updated_at
             WHERE file_media_state.state IN ('pending', 'failed', 'leased')",
            params![
                file_id,
                leased_at_ms + MEDIA_INSPECTION_LEASE_MS,
                leased_at_ms,
            ],
        )?;
        Ok(())
    }

    fn upsert_file_media_latest(
        &self,
        file_id: i64,
        media_run_id: i64,
        media: &MediaInspectionSuccess,
        inspected_at_ms: i64,
    ) -> LibrarySqliteResult<()> {
        self.tx().execute(
            "INSERT INTO file_media_latest (
                 file_id,
                 media_run_id,
                 media_kind,
                 mime_type,
                 width_px,
                 height_px,
                 duration_ms,
                 orientation_degrees,
                 content_hash,
                 inspected_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(file_id) DO UPDATE
             SET media_run_id = excluded.media_run_id,
                 media_kind = excluded.media_kind,
                 mime_type = excluded.mime_type,
                 width_px = excluded.width_px,
                 height_px = excluded.height_px,
                 duration_ms = excluded.duration_ms,
                 orientation_degrees = excluded.orientation_degrees,
                 content_hash = excluded.content_hash,
                 inspected_at = excluded.inspected_at",
            params![
                file_id,
                media_run_id,
                media.media_kind.as_str(),
                media.mime_type.as_str(),
                media.width_px,
                media.height_px,
                media.duration_ms,
                media.orientation_degrees,
                media.content_hash.as_deref(),
                inspected_at_ms,
            ],
        )?;

        Ok(())
    }
}

fn filesystem_media_inspection_outcome(
    file_path: &Path,
    canonical_path: &str,
    root_id: i64,
    cancellation: Option<&SourceAdmissionToken>,
) -> Result<MediaInspectionOutcome, LibrarySqliteError> {
    let Some(media_type) = EligibleVisualMediaType::from_path(canonical_path) else {
        return Ok(MediaInspectionOutcome::Failed {
            error_detail: Some(format!(
                "media inspection failed for {canonical_path}: path is not an eligible visual media candidate"
            )),
        });
    };

    match inspect_visual_media_from_filesystem_with_context(
        file_path,
        canonical_path,
        media_type,
        cancellation,
    ) {
        Ok(success) => Ok(MediaInspectionOutcome::Succeeded(success)),
        Err(MediaFilesystemInspection::Cancelled) => {
            Err(LibrarySqliteError::RootWorkCancelled { root_id })
        }
        Err(MediaFilesystemInspection::Failed(error_detail)) => {
            Ok(MediaInspectionOutcome::Failed {
                error_detail: Some(error_detail),
            })
        }
    }
}

enum MediaFilesystemInspection {
    Cancelled,
    Failed(String),
}

fn inspect_visual_media_from_filesystem_with_context(
    file_path: &Path,
    canonical_path: &str,
    media_type: EligibleVisualMediaType,
    cancellation: Option<&SourceAdmissionToken>,
) -> Result<MediaInspectionSuccess, MediaFilesystemInspection> {
    let (content_hash, header) =
        read_content_hash_and_header(file_path, cancellation).map_err(|error| match error {
            ReadMediaFilesystemError::Cancelled => MediaFilesystemInspection::Cancelled,
            ReadMediaFilesystemError::Io(io_error) => MediaFilesystemInspection::Failed(format!(
                "media inspection failed for {canonical_path}: {io_error}"
            )),
        })?;
    let (width_px, height_px, duration_ms, orientation_degrees) =
        media_type.inspect_header(&header).map_err(|detail| {
            MediaFilesystemInspection::Failed(format!(
                "media inspection failed for {canonical_path}: {detail}"
            ))
        })?;

    Ok(MediaInspectionSuccess {
        media_kind: media_type.media_kind().to_string(),
        mime_type: media_type.mime_type().to_string(),
        width_px,
        height_px,
        duration_ms,
        orientation_degrees,
        content_hash: Some(format!("sha256:{content_hash}")),
    })
}

enum ReadMediaFilesystemError {
    Cancelled,
    Io(std::io::Error),
}

fn read_content_hash_and_header(
    file_path: &Path,
    cancellation: Option<&SourceAdmissionToken>,
) -> Result<(String, Vec<u8>), ReadMediaFilesystemError> {
    let file =
        open_source_media_file_for_read(MEDIA_INSPECTION_SOURCE_MEDIA_WRITE_POLICY, file_path)
            .map_err(ReadMediaFilesystemError::Io)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut header = Vec::with_capacity(INSPECTION_HEADER_BYTES);
    let mut buffer = [0_u8; INSPECTION_READ_BUFFER_BYTES];

    loop {
        if cancellation.is_some_and(SourceAdmissionToken::is_cancelled) {
            return Err(ReadMediaFilesystemError::Cancelled);
        }
        let bytes_read = reader
            .read(&mut buffer)
            .map_err(ReadMediaFilesystemError::Io)?;
        if bytes_read == 0 {
            break;
        }

        if header.len() < INSPECTION_HEADER_BYTES {
            let header_bytes = (INSPECTION_HEADER_BYTES - header.len()).min(bytes_read);
            header.extend_from_slice(&buffer[..header_bytes]);
        }

        hasher.update(&buffer[..bytes_read]);
    }

    Ok((hex_encode(&hasher.finalize()), header))
}

fn inspect_png_header(header: &[u8]) -> MediaHeaderInspection {
    const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
    if header.len() < 24 || !header.starts_with(PNG_SIGNATURE) || &header[12..16] != b"IHDR" {
        return Err("missing PNG signature or IHDR chunk");
    }

    let width_px = u32::from_be_bytes([header[16], header[17], header[18], header[19]]) as i64;
    let height_px = u32::from_be_bytes([header[20], header[21], header[22], header[23]]) as i64;
    Ok((Some(width_px), Some(height_px), None, None))
}

fn inspect_gif_header(header: &[u8]) -> MediaHeaderInspection {
    if header.len() < 10 || !(header.starts_with(b"GIF87a") || header.starts_with(b"GIF89a")) {
        return Err("missing GIF signature");
    }

    let width_px = u16::from_le_bytes([header[6], header[7]]) as i64;
    let height_px = u16::from_le_bytes([header[8], header[9]]) as i64;
    Ok((Some(width_px), Some(height_px), None, None))
}

fn inspect_jpeg_header(header: &[u8]) -> MediaHeaderInspection {
    if header.len() < 4 || header[0] != 0xff || header[1] != 0xd8 {
        return Err("missing JPEG SOI marker");
    }

    let mut index = 2;
    while index + 8 < header.len() {
        if header[index] != 0xff {
            index += 1;
            continue;
        }

        while index < header.len() && header[index] == 0xff {
            index += 1;
        }
        if index >= header.len() {
            break;
        }

        let marker = header[index];
        index += 1;

        if matches!(marker, 0xd8 | 0xd9) {
            continue;
        }
        if index + 1 >= header.len() {
            break;
        }
        let segment_length = u16::from_be_bytes([header[index], header[index + 1]]) as usize;
        if segment_length < 2 || index + segment_length > header.len() {
            break;
        }

        if matches!(
            marker,
            0xc0 | 0xc1
                | 0xc2
                | 0xc3
                | 0xc5
                | 0xc6
                | 0xc7
                | 0xc9
                | 0xca
                | 0xcb
                | 0xcd
                | 0xce
                | 0xcf
        ) {
            if segment_length < 7 {
                return Err("malformed JPEG SOF segment");
            }
            let height_index = index + 3;
            let width_index = index + 5;
            let height_px =
                u16::from_be_bytes([header[height_index], header[height_index + 1]]) as i64;
            let width_px =
                u16::from_be_bytes([header[width_index], header[width_index + 1]]) as i64;
            return Ok((Some(width_px), Some(height_px), None, None));
        }

        index += segment_length;
    }

    Err("missing JPEG size-bearing SOF segment in header window")
}

fn inspect_iso_bmff_header(header: &[u8]) -> MediaHeaderInspection {
    if header.len() < 12 || &header[4..8] != b"ftyp" {
        return Err("missing ISO BMFF ftyp box");
    }

    Ok((None, None, None, None))
}

fn inspect_webm_header(header: &[u8]) -> MediaHeaderInspection {
    if header.len() < 4 || header[0..4] != [0x1a, 0x45, 0xdf, 0xa3] {
        return Err("missing WebM EBML header");
    }

    Ok((None, None, None, None))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    encoded
}

fn unix_time_ms_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{
        build_filesystem_media_inspection_request, is_eligible_visual_media_path,
        MediaInspectionOutcome,
    };

    fn minimal_png_bytes() -> Vec<u8> {
        vec![
            0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, b'I', b'H',
            b'D', b'R', 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00,
        ]
    }

    #[test]
    fn eligible_visual_media_path_is_case_insensitive_and_non_audio() {
        assert!(is_eligible_visual_media_path("art/COVER.PNG"));
        assert!(is_eligible_visual_media_path("video/promo.mp4"));
        assert!(is_eligible_visual_media_path("gallery/page.jpeg"));
        assert!(!is_eligible_visual_media_path("artist/alpha.flac"));
        assert!(!is_eligible_visual_media_path("notes/schema.sql"));
    }

    #[test]
    fn filesystem_media_inspection_request_succeeds_for_valid_png_media() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let file_path = tempdir.path().join("cover.png");
        std::fs::write(&file_path, minimal_png_bytes()).expect("write png");

        let request = build_filesystem_media_inspection_request(7, &file_path, "covers/cover.png");

        match request.outcome {
            MediaInspectionOutcome::Succeeded(success) => {
                assert_eq!(success.media_kind, "image");
                assert_eq!(success.mime_type, "image/png");
                assert_eq!(success.width_px, Some(2));
                assert_eq!(success.height_px, Some(3));
                assert!(success
                    .content_hash
                    .as_deref()
                    .is_some_and(|value| value.starts_with("sha256:")));
            }
            other => panic!("expected successful media inspection, got {other:?}"),
        }
    }

    #[test]
    fn filesystem_media_inspection_request_persists_invalid_media_as_failure() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let file_path = tempdir.path().join("broken.png");
        std::fs::write(&file_path, b"not a png").expect("write invalid png");

        let request = build_filesystem_media_inspection_request(9, &file_path, "covers/broken.png");

        match request.outcome {
            MediaInspectionOutcome::Failed { error_detail } => {
                assert!(error_detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("PNG")));
            }
            other => panic!("expected failed media inspection, got {other:?}"),
        }
    }
}
