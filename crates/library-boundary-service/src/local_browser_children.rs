use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use library_boundary_protocol as protocol;

use crate::local_browser_entry_points::{
    local_browser_entry_point_platform, normalize_local_browser_path_key,
};

const LOCAL_BROWSER_CHILD_LIMIT_MAX: usize = 200;

pub(crate) trait LocalBrowserChildReader: Send + Sync {
    fn read_children(
        &self,
        request: protocol::ReadLocalBrowserChildrenRequest,
        admitted_source_path_keys: &HashSet<String>,
    ) -> protocol::ProtocolResult<protocol::ReadLocalBrowserChildrenReply>;
}

pub(crate) fn production_local_browser_child_reader() -> Arc<dyn LocalBrowserChildReader> {
    Arc::new(PlatformLocalBrowserChildReader)
}

struct PlatformLocalBrowserChildReader;

impl LocalBrowserChildReader for PlatformLocalBrowserChildReader {
    fn read_children(
        &self,
        request: protocol::ReadLocalBrowserChildrenRequest,
        admitted_source_path_keys: &HashSet<String>,
    ) -> protocol::ProtocolResult<protocol::ReadLocalBrowserChildrenReply> {
        validate_common_request(&request)?;

        #[cfg(not(windows))]
        {
            let _ = admitted_source_path_keys;
            Ok(failure_reply(
                &request,
                protocol::LocalBrowserChildrenReadStatus::UnsupportedPlatform,
                protocol::LocalBrowserChildFailureCode::UnsupportedPlatform,
                "local browser child reads are Windows-only in V0",
            ))
        }

        #[cfg(windows)]
        {
            read_windows_children(request, admitted_source_path_keys)
        }
    }
}

fn validate_common_request(
    request: &protocol::ReadLocalBrowserChildrenRequest,
) -> protocol::ProtocolResult<()> {
    if request.root_canonical_path.trim().is_empty() {
        return Err(protocol::ProtocolError::InvalidRequest {
            detail: "readLocalBrowserChildren rootCanonicalPath must not be empty".to_string(),
        });
    }
    if request.parent_canonical_path.trim().is_empty() {
        return Err(protocol::ProtocolError::InvalidRequest {
            detail: "readLocalBrowserChildren parentCanonicalPath must not be empty".to_string(),
        });
    }
    if request.limit == 0 || request.limit > LOCAL_BROWSER_CHILD_LIMIT_MAX {
        return Err(protocol::ProtocolError::InvalidRequest {
            detail: format!(
                "readLocalBrowserChildren limit must be between 1 and {LOCAL_BROWSER_CHILD_LIMIT_MAX}"
            ),
        });
    }
    Ok(())
}

#[cfg(windows)]
fn read_windows_children(
    request: protocol::ReadLocalBrowserChildrenRequest,
    admitted_source_path_keys: &HashSet<String>,
) -> protocol::ProtocolResult<protocol::ReadLocalBrowserChildrenReply> {
    let root_path = PathBuf::from(&request.root_canonical_path);
    let parent_path = PathBuf::from(&request.parent_canonical_path);
    validate_windows_path(&root_path, "rootCanonicalPath")?;
    validate_windows_path(&parent_path, "parentCanonicalPath")?;

    let root_key = normalize_local_browser_path_key(&root_path);
    let parent_key = normalize_local_browser_path_key(&parent_path);
    if !path_key_is_within_or_equal(&parent_key, &root_key) {
        return Ok(failure_reply(
            &request,
            protocol::LocalBrowserChildrenReadStatus::Failed,
            protocol::LocalBrowserChildFailureCode::ParentOutsideRoot,
            "parentCanonicalPath is outside rootCanonicalPath",
        ));
    }

    let root_metadata = match fs::symlink_metadata(&root_path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return Ok(path_failure_reply(
                &request,
                error,
                protocol::LocalBrowserChildFailureCode::RootPathUnavailable,
                protocol::LocalBrowserChildFailureCode::RootPathUnavailable,
            ));
        }
    };
    if is_reparse_point(&root_metadata) {
        return Ok(failure_reply(
            &request,
            protocol::LocalBrowserChildrenReadStatus::Unavailable,
            protocol::LocalBrowserChildFailureCode::ReparsePointSkipped,
            "rootCanonicalPath is a reparse point and was not followed",
        ));
    }
    if !root_metadata.is_dir() {
        return Ok(failure_reply(
            &request,
            protocol::LocalBrowserChildrenReadStatus::Unavailable,
            protocol::LocalBrowserChildFailureCode::RootPathUnavailable,
            "rootCanonicalPath is not a directory",
        ));
    }

    let parent_metadata = match fs::symlink_metadata(&parent_path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return Ok(path_failure_reply(
                &request,
                error,
                protocol::LocalBrowserChildFailureCode::ParentMissing,
                protocol::LocalBrowserChildFailureCode::ParentPathUnavailable,
            ));
        }
    };
    if is_reparse_point(&parent_metadata) {
        return Ok(failure_reply(
            &request,
            protocol::LocalBrowserChildrenReadStatus::Unavailable,
            protocol::LocalBrowserChildFailureCode::ReparsePointSkipped,
            "parentCanonicalPath is a reparse point and was not followed",
        ));
    }
    if !parent_metadata.is_dir() {
        return Ok(failure_reply(
            &request,
            protocol::LocalBrowserChildrenReadStatus::Failed,
            protocol::LocalBrowserChildFailureCode::ParentNotDirectory,
            "parentCanonicalPath is not a directory",
        ));
    }

    let directory = match fs::read_dir(&parent_path) {
        Ok(directory) => directory,
        Err(error) => {
            return Ok(path_failure_reply(
                &request,
                error,
                protocol::LocalBrowserChildFailureCode::ParentPathUnavailable,
                protocol::LocalBrowserChildFailureCode::EnumerationUnavailable,
            ));
        }
    };

    let mut rows = Vec::new();
    let mut enumeration_failure = None;
    for entry_result in directory {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(error) => {
                enumeration_failure = Some(error);
                continue;
            }
        };
        rows.push(row_for_directory_entry(
            &request,
            &root_key,
            &parent_key,
            entry,
            admitted_source_path_keys,
        ));
    }

    rows.sort_by(|left, right| {
        row_sort_group(left.row_kind)
            .cmp(&row_sort_group(right.row_kind))
            .then_with(|| {
                normalized_sort_text(&left.display_name)
                    .cmp(&normalized_sort_text(&right.display_name))
            })
            .then_with(|| {
                normalize_local_browser_path_key(Path::new(&left.identity.candidate_canonical_path))
                    .cmp(&normalize_local_browser_path_key(Path::new(
                        &right.identity.candidate_canonical_path,
                    )))
            })
    });

    let total_rows = rows.len();
    let window_rows = rows
        .into_iter()
        .skip(request.offset)
        .take(request.limit)
        .collect::<Vec<_>>();
    let row_failure = window_rows.iter().any(|row| {
        row.row_kind == protocol::LocalBrowserCandidateRowKind::InaccessibleCandidate
            && row.failure.is_some()
    });
    let status = if enumeration_failure.is_some() || row_failure {
        protocol::LocalBrowserChildrenReadStatus::PartialFailure
    } else {
        protocol::LocalBrowserChildrenReadStatus::Complete
    };
    let failure = if enumeration_failure.is_some() || row_failure {
        Some(protocol::LocalBrowserChildFailure {
            code: protocol::LocalBrowserChildFailureCode::EnumerationUnavailable,
            detail: "one or more local browser child rows could not be fully resolved".to_string(),
        })
    } else {
        None
    };

    Ok(protocol::ReadLocalBrowserChildrenReply {
        status,
        window_identity: window_identity(&request),
        offset: request.offset,
        limit: request.limit,
        total_rows,
        rows: window_rows,
        failure,
    })
}

#[cfg(windows)]
fn validate_windows_path(path: &Path, field: &str) -> protocol::ProtocolResult<()> {
    if !path.is_absolute() {
        return Err(protocol::ProtocolError::InvalidRequest {
            detail: format!("readLocalBrowserChildren {field} must be absolute"),
        });
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(protocol::ProtocolError::InvalidRequest {
            detail: format!("readLocalBrowserChildren {field} must not contain parent segments"),
        });
    }
    Ok(())
}

#[cfg(windows)]
fn row_for_directory_entry(
    request: &protocol::ReadLocalBrowserChildrenRequest,
    root_key: &str,
    parent_key: &str,
    entry: fs::DirEntry,
    admitted_source_path_keys: &HashSet<String>,
) -> protocol::LocalBrowserChildRow {
    let display_name = entry.file_name().to_string_lossy().into_owned();
    let path = entry.path();
    let candidate_key = normalize_local_browser_path_key(&path);
    if !path_key_is_within_or_equal(&candidate_key, root_key) {
        return inaccessible_row(
            request,
            path,
            display_name,
            protocol::LocalBrowserCandidateStatus::Rejected,
            protocol::LocalBrowserChildFailureCode::ParentOutsideRoot,
            "child candidate resolved outside rootCanonicalPath",
        );
    }

    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) => {
            let (status, code) = row_status_from_error(error.kind());
            return inaccessible_row(
                request,
                path,
                display_name,
                status,
                code,
                format!("failed to read child metadata: {error}"),
            );
        }
    };

    if is_reparse_point(&metadata) {
        return inaccessible_row(
            request,
            path,
            display_name,
            protocol::LocalBrowserCandidateStatus::Rejected,
            protocol::LocalBrowserChildFailureCode::ReparsePointSkipped,
            "child candidate is a reparse point and was not followed",
        );
    }

    if metadata.is_dir() {
        let row_kind = if is_rejected_system_drive_child(
            request.entry_point_kind,
            parent_key,
            root_key,
            &display_name,
        ) {
            protocol::LocalBrowserCandidateRowKind::RejectedRootCandidate
        } else {
            protocol::LocalBrowserCandidateRowKind::DirectoryCandidate
        };
        return directory_row(
            request,
            path,
            display_name,
            row_kind,
            admitted_source_path_keys,
        );
    }

    if metadata.is_file() {
        return file_row(request, path, display_name, admitted_source_path_keys);
    }

    protocol::LocalBrowserChildRow {
        identity: candidate_identity(request, &path),
        row_kind: protocol::LocalBrowserCandidateRowKind::UnknownCandidate,
        display_name,
        status: protocol::LocalBrowserCandidateStatus::Unknown,
        platform: local_browser_entry_point_platform(),
        file_kind: Some(protocol::ContentsFileKind::Unknown),
        media_relevance: Some(protocol::LocalBrowserCandidateMediaRelevance::Unknown),
        admission_hint: protocol::LocalBrowserCandidateAdmissionHint::NotDirectlyAdmissible,
        affordances: unavailable_affordances(),
        failure: None,
    }
}

fn directory_row(
    request: &protocol::ReadLocalBrowserChildrenRequest,
    path: PathBuf,
    display_name: String,
    row_kind: protocol::LocalBrowserCandidateRowKind,
    admitted_source_path_keys: &HashSet<String>,
) -> protocol::LocalBrowserChildRow {
    let rejected = row_kind == protocol::LocalBrowserCandidateRowKind::RejectedRootCandidate;
    let duplicate =
        !rejected && admitted_source_path_keys.contains(&normalize_local_browser_path_key(&path));
    let status = if duplicate {
        protocol::LocalBrowserCandidateStatus::DuplicateOfAdmittedSource
    } else if rejected {
        protocol::LocalBrowserCandidateStatus::Rejected
    } else {
        protocol::LocalBrowserCandidateStatus::Available
    };
    let admission_hint = if duplicate {
        protocol::LocalBrowserCandidateAdmissionHint::DuplicateOfAdmittedSource
    } else if rejected {
        protocol::LocalBrowserCandidateAdmissionHint::Rejected
    } else if request.entry_point_kind == protocol::LocalBrowserEntryPointKind::Music {
        protocol::LocalBrowserCandidateAdmissionHint::CanRequestAdmission
    } else {
        protocol::LocalBrowserCandidateAdmissionHint::RequiresConfirmation
    };
    let can_browse = !rejected;
    let can_request_admission =
        status == protocol::LocalBrowserCandidateStatus::Available && !rejected;
    let requires_confirmation = can_request_admission
        && request.entry_point_kind != protocol::LocalBrowserEntryPointKind::Music;

    protocol::LocalBrowserChildRow {
        identity: candidate_identity(request, &path),
        row_kind,
        display_name,
        status,
        platform: local_browser_entry_point_platform(),
        file_kind: None,
        media_relevance: None,
        admission_hint,
        affordances: protocol::LocalBrowserCandidateAffordances {
            can_browse,
            can_request_admission,
            can_choose_descendant: can_browse,
            can_request_parent_admission: false,
            requires_confirmation,
        },
        failure: rejected.then(|| protocol::LocalBrowserChildFailure {
            code: protocol::LocalBrowserChildFailureCode::RejectedRoot,
            detail: "system-owned directory is not directly admissible as a source root"
                .to_string(),
        }),
    }
}

fn file_row(
    request: &protocol::ReadLocalBrowserChildrenRequest,
    path: PathBuf,
    display_name: String,
    admitted_source_path_keys: &HashSet<String>,
) -> protocol::LocalBrowserChildRow {
    let classification = classify_candidate_file(&path);
    let duplicate = admitted_source_path_keys.contains(&normalize_local_browser_path_key(&path));
    let status = if duplicate {
        protocol::LocalBrowserCandidateStatus::DuplicateOfAdmittedSource
    } else {
        classification.status
    };
    let admission_hint = if duplicate {
        protocol::LocalBrowserCandidateAdmissionHint::DuplicateOfAdmittedSource
    } else {
        classification.admission_hint
    };

    protocol::LocalBrowserChildRow {
        identity: candidate_identity(request, &path),
        row_kind: classification.row_kind,
        display_name,
        status,
        platform: local_browser_entry_point_platform(),
        file_kind: Some(classification.file_kind),
        media_relevance: Some(classification.media_relevance),
        admission_hint,
        affordances: protocol::LocalBrowserCandidateAffordances {
            can_browse: false,
            can_request_admission: false,
            can_choose_descendant: false,
            can_request_parent_admission: !duplicate && classification.can_request_parent_admission,
            requires_confirmation: false,
        },
        failure: None,
    }
}

fn inaccessible_row(
    request: &protocol::ReadLocalBrowserChildrenRequest,
    path: PathBuf,
    display_name: String,
    status: protocol::LocalBrowserCandidateStatus,
    failure_code: protocol::LocalBrowserChildFailureCode,
    detail: impl Into<String>,
) -> protocol::LocalBrowserChildRow {
    protocol::LocalBrowserChildRow {
        identity: candidate_identity(request, &path),
        row_kind: protocol::LocalBrowserCandidateRowKind::InaccessibleCandidate,
        display_name,
        status,
        platform: local_browser_entry_point_platform(),
        file_kind: None,
        media_relevance: None,
        admission_hint: protocol::LocalBrowserCandidateAdmissionHint::Unavailable,
        affordances: unavailable_affordances(),
        failure: Some(protocol::LocalBrowserChildFailure {
            code: failure_code,
            detail: detail.into(),
        }),
    }
}

struct CandidateFileClassification {
    row_kind: protocol::LocalBrowserCandidateRowKind,
    status: protocol::LocalBrowserCandidateStatus,
    file_kind: protocol::ContentsFileKind,
    media_relevance: protocol::LocalBrowserCandidateMediaRelevance,
    admission_hint: protocol::LocalBrowserCandidateAdmissionHint,
    can_request_parent_admission: bool,
}

fn classify_candidate_file(path: &Path) -> CandidateFileClassification {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase());

    match extension.as_deref() {
        Some(
            "mp3" | "flac" | "wav" | "aiff" | "aif" | "aifc" | "ogg" | "oga" | "m4a" | "aac"
            | "opus" | "wv",
        ) => media_file(protocol::ContentsFileKind::Audio),
        Some("mp4" | "m4v" | "mkv" | "avi" | "mov" | "wmv" | "webm") => {
            media_file(protocol::ContentsFileKind::Video)
        }
        Some("jpg" | "jpeg" | "png" | "webp") => media_file(protocol::ContentsFileKind::Image),
        Some("cue") => CandidateFileClassification {
            row_kind: protocol::LocalBrowserCandidateRowKind::MediaFileCandidate,
            status: protocol::LocalBrowserCandidateStatus::Available,
            file_kind: protocol::ContentsFileKind::CueSheet,
            media_relevance: protocol::LocalBrowserCandidateMediaRelevance::CompanionMetadata,
            admission_hint: protocol::LocalBrowserCandidateAdmissionHint::ChooseParentDirectory,
            can_request_parent_admission: true,
        },
        Some("log") => unsupported_file(protocol::ContentsFileKind::LogDoc),
        Some("txt" | "nfo" | "md") => unsupported_file(protocol::ContentsFileKind::TextDoc),
        Some("zip" | "rar" | "7z" | "tar" | "gz") => {
            unsupported_file(protocol::ContentsFileKind::Archive)
        }
        Some(_) => unsupported_file(protocol::ContentsFileKind::Other),
        None => CandidateFileClassification {
            row_kind: protocol::LocalBrowserCandidateRowKind::UnknownCandidate,
            status: protocol::LocalBrowserCandidateStatus::Unknown,
            file_kind: protocol::ContentsFileKind::Unknown,
            media_relevance: protocol::LocalBrowserCandidateMediaRelevance::Unknown,
            admission_hint: protocol::LocalBrowserCandidateAdmissionHint::NotDirectlyAdmissible,
            can_request_parent_admission: false,
        },
    }
}

fn media_file(file_kind: protocol::ContentsFileKind) -> CandidateFileClassification {
    CandidateFileClassification {
        row_kind: protocol::LocalBrowserCandidateRowKind::MediaFileCandidate,
        status: protocol::LocalBrowserCandidateStatus::Available,
        file_kind,
        media_relevance: protocol::LocalBrowserCandidateMediaRelevance::MediaRelevant,
        admission_hint: protocol::LocalBrowserCandidateAdmissionHint::ChooseParentDirectory,
        can_request_parent_admission: true,
    }
}

fn unsupported_file(file_kind: protocol::ContentsFileKind) -> CandidateFileClassification {
    CandidateFileClassification {
        row_kind: protocol::LocalBrowserCandidateRowKind::UnsupportedFileCandidate,
        status: protocol::LocalBrowserCandidateStatus::Available,
        file_kind,
        media_relevance: protocol::LocalBrowserCandidateMediaRelevance::Unsupported,
        admission_hint: protocol::LocalBrowserCandidateAdmissionHint::NotDirectlyAdmissible,
        can_request_parent_admission: false,
    }
}

fn row_status_from_error(
    error_kind: io::ErrorKind,
) -> (
    protocol::LocalBrowserCandidateStatus,
    protocol::LocalBrowserChildFailureCode,
) {
    match error_kind {
        io::ErrorKind::NotFound => (
            protocol::LocalBrowserCandidateStatus::Missing,
            protocol::LocalBrowserChildFailureCode::MetadataUnavailable,
        ),
        io::ErrorKind::PermissionDenied => (
            protocol::LocalBrowserCandidateStatus::PermissionBlocked,
            protocol::LocalBrowserChildFailureCode::PermissionDenied,
        ),
        _ => (
            protocol::LocalBrowserCandidateStatus::Unavailable,
            protocol::LocalBrowserChildFailureCode::MetadataUnavailable,
        ),
    }
}

fn path_failure_reply(
    request: &protocol::ReadLocalBrowserChildrenRequest,
    error: io::Error,
    missing_code: protocol::LocalBrowserChildFailureCode,
    unavailable_code: protocol::LocalBrowserChildFailureCode,
) -> protocol::ReadLocalBrowserChildrenReply {
    let (status, code) = match error.kind() {
        io::ErrorKind::NotFound => (
            protocol::LocalBrowserChildrenReadStatus::Missing,
            missing_code,
        ),
        io::ErrorKind::PermissionDenied => (
            protocol::LocalBrowserChildrenReadStatus::PermissionBlocked,
            protocol::LocalBrowserChildFailureCode::PermissionDenied,
        ),
        _ => (
            protocol::LocalBrowserChildrenReadStatus::Unavailable,
            unavailable_code,
        ),
    };
    failure_reply(request, status, code, error.to_string())
}

fn failure_reply(
    request: &protocol::ReadLocalBrowserChildrenRequest,
    status: protocol::LocalBrowserChildrenReadStatus,
    code: protocol::LocalBrowserChildFailureCode,
    detail: impl Into<String>,
) -> protocol::ReadLocalBrowserChildrenReply {
    protocol::ReadLocalBrowserChildrenReply {
        status,
        window_identity: window_identity(request),
        offset: request.offset,
        limit: request.limit,
        total_rows: 0,
        rows: Vec::new(),
        failure: Some(protocol::LocalBrowserChildFailure {
            code,
            detail: detail.into(),
        }),
    }
}

fn candidate_identity(
    request: &protocol::ReadLocalBrowserChildrenRequest,
    candidate_path: &Path,
) -> protocol::LocalBrowserCandidateIdentity {
    protocol::LocalBrowserCandidateIdentity {
        entry_point_kind: request.entry_point_kind,
        root_canonical_path: request.root_canonical_path.clone(),
        candidate_canonical_path: candidate_path.to_string_lossy().into_owned(),
    }
}

fn window_identity(
    request: &protocol::ReadLocalBrowserChildrenRequest,
) -> protocol::LocalBrowserChildWindowIdentity {
    protocol::LocalBrowserChildWindowIdentity {
        entry_point_kind: request.entry_point_kind,
        root_canonical_path: request.root_canonical_path.clone(),
        parent_canonical_path: request.parent_canonical_path.clone(),
    }
}

fn unavailable_affordances() -> protocol::LocalBrowserCandidateAffordances {
    protocol::LocalBrowserCandidateAffordances {
        can_browse: false,
        can_request_admission: false,
        can_choose_descendant: false,
        can_request_parent_admission: false,
        requires_confirmation: false,
    }
}

fn path_key_is_within_or_equal(candidate_key: &str, root_key: &str) -> bool {
    candidate_key == root_key
        || candidate_key.starts_with(&format!("{root_key}\\"))
        || (root_key.ends_with(':') && candidate_key.starts_with(&format!("{root_key}\\")))
}

fn row_sort_group(row_kind: protocol::LocalBrowserCandidateRowKind) -> u8 {
    match row_kind {
        protocol::LocalBrowserCandidateRowKind::DirectoryCandidate
        | protocol::LocalBrowserCandidateRowKind::RejectedRootCandidate => 0,
        protocol::LocalBrowserCandidateRowKind::MediaFileCandidate => 1,
        protocol::LocalBrowserCandidateRowKind::UnsupportedFileCandidate => 2,
        protocol::LocalBrowserCandidateRowKind::InaccessibleCandidate => 3,
        protocol::LocalBrowserCandidateRowKind::UnknownCandidate => 4,
    }
}

fn normalized_sort_text(value: &str) -> String {
    value.replace('/', "\\").to_ascii_lowercase()
}

#[cfg(windows)]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    metadata.file_attributes()
        & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
        != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

fn is_rejected_system_drive_child(
    entry_point_kind: protocol::LocalBrowserEntryPointKind,
    parent_key: &str,
    root_key: &str,
    display_name: &str,
) -> bool {
    entry_point_kind == protocol::LocalBrowserEntryPointKind::SystemDriveRoot
        && parent_key == root_key
        && matches!(
            display_name.to_ascii_lowercase().as_str(),
            "$recycle.bin"
                | "$sysreset"
                | "program files"
                | "program files (x86)"
                | "programdata"
                | "recovery"
                | "system volume information"
                | "windows"
        )
}

#[cfg(test)]
pub(crate) fn classify_candidate_file_for_test(
    path: &Path,
) -> (
    protocol::LocalBrowserCandidateRowKind,
    protocol::ContentsFileKind,
    protocol::LocalBrowserCandidateMediaRelevance,
    protocol::LocalBrowserCandidateAdmissionHint,
) {
    let classification = classify_candidate_file(path);
    (
        classification.row_kind,
        classification.file_kind,
        classification.media_relevance,
        classification.admission_hint,
    )
}
