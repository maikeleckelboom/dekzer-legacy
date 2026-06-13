use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use library_boundary_protocol as protocol;

use crate::local_browse_entry_points::{
    local_browse_entry_point_platform, normalize_local_browse_path_key,
};

const LOCAL_BROWSE_ITEM_LIMIT_MAX: usize = 200;

pub(crate) trait LocalBrowseItemReader: Send + Sync {
    fn read_items(
        &self,
        request: protocol::ReadLocalBrowseItemsRequest,
        admitted_source_path_keys: &HashSet<String>,
    ) -> protocol::ProtocolResult<protocol::ReadLocalBrowseItemsReply>;
}

pub(crate) fn production_local_browse_item_reader() -> Arc<dyn LocalBrowseItemReader> {
    Arc::new(PlatformLocalBrowseItemReader)
}

struct PlatformLocalBrowseItemReader;

impl LocalBrowseItemReader for PlatformLocalBrowseItemReader {
    fn read_items(
        &self,
        request: protocol::ReadLocalBrowseItemsRequest,
        admitted_source_path_keys: &HashSet<String>,
    ) -> protocol::ProtocolResult<protocol::ReadLocalBrowseItemsReply> {
        validate_common_request(&request)?;

        #[cfg(not(windows))]
        {
            let _ = admitted_source_path_keys;
            Ok(failure_reply(
                &request,
                protocol::LocalBrowseItemsReadStatus::UnsupportedPlatform,
                protocol::LocalBrowseItemFailureCode::UnsupportedPlatform,
                "local browse item reads are Windows-only in V0",
            ))
        }

        #[cfg(windows)]
        {
            read_windows_items(request, admitted_source_path_keys)
        }
    }
}

fn validate_common_request(
    request: &protocol::ReadLocalBrowseItemsRequest,
) -> protocol::ProtocolResult<()> {
    if request.root_canonical_path.trim().is_empty() {
        return Err(protocol::ProtocolError::InvalidRequest {
            detail: "readLocalBrowseItems rootCanonicalPath must not be empty".to_string(),
        });
    }
    if request.parent_canonical_path.trim().is_empty() {
        return Err(protocol::ProtocolError::InvalidRequest {
            detail: "readLocalBrowseItems parentCanonicalPath must not be empty".to_string(),
        });
    }
    if request.limit == 0 || request.limit > LOCAL_BROWSE_ITEM_LIMIT_MAX {
        return Err(protocol::ProtocolError::InvalidRequest {
            detail: format!(
                "readLocalBrowseItems limit must be between 1 and {LOCAL_BROWSE_ITEM_LIMIT_MAX}"
            ),
        });
    }
    Ok(())
}

#[cfg(windows)]
fn read_windows_items(
    request: protocol::ReadLocalBrowseItemsRequest,
    admitted_source_path_keys: &HashSet<String>,
) -> protocol::ProtocolResult<protocol::ReadLocalBrowseItemsReply> {
    let root_path = PathBuf::from(&request.root_canonical_path);
    let parent_path = PathBuf::from(&request.parent_canonical_path);
    validate_windows_path(&root_path, "rootCanonicalPath")?;
    validate_windows_path(&parent_path, "parentCanonicalPath")?;

    let root_key = normalize_local_browse_path_key(&root_path);
    let parent_key = normalize_local_browse_path_key(&parent_path);
    if !path_key_is_within_or_equal(&parent_key, &root_key) {
        return Ok(failure_reply(
            &request,
            protocol::LocalBrowseItemsReadStatus::Failed,
            protocol::LocalBrowseItemFailureCode::ParentOutsideRoot,
            "parentCanonicalPath is outside rootCanonicalPath",
        ));
    }

    let root_metadata = match fs::symlink_metadata(&root_path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return Ok(path_failure_reply(
                &request,
                error,
                protocol::LocalBrowseItemFailureCode::RootPathUnavailable,
                protocol::LocalBrowseItemFailureCode::RootPathUnavailable,
            ));
        }
    };
    if is_reparse_point(&root_metadata) {
        return Ok(failure_reply(
            &request,
            protocol::LocalBrowseItemsReadStatus::Unavailable,
            protocol::LocalBrowseItemFailureCode::ReparsePointSkipped,
            "rootCanonicalPath is a reparse point and was not followed",
        ));
    }
    if !root_metadata.is_dir() {
        return Ok(failure_reply(
            &request,
            protocol::LocalBrowseItemsReadStatus::Unavailable,
            protocol::LocalBrowseItemFailureCode::RootPathUnavailable,
            "rootCanonicalPath is not a directory",
        ));
    }

    let parent_metadata = match fs::symlink_metadata(&parent_path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return Ok(path_failure_reply(
                &request,
                error,
                protocol::LocalBrowseItemFailureCode::ParentMissing,
                protocol::LocalBrowseItemFailureCode::ParentPathUnavailable,
            ));
        }
    };
    if is_reparse_point(&parent_metadata) {
        return Ok(failure_reply(
            &request,
            protocol::LocalBrowseItemsReadStatus::Unavailable,
            protocol::LocalBrowseItemFailureCode::ReparsePointSkipped,
            "parentCanonicalPath is a reparse point and was not followed",
        ));
    }
    if !parent_metadata.is_dir() {
        return Ok(failure_reply(
            &request,
            protocol::LocalBrowseItemsReadStatus::Failed,
            protocol::LocalBrowseItemFailureCode::ParentNotDirectory,
            "parentCanonicalPath is not a directory",
        ));
    }

    let directory = match fs::read_dir(&parent_path) {
        Ok(directory) => directory,
        Err(error) => {
            return Ok(path_failure_reply(
                &request,
                error,
                protocol::LocalBrowseItemFailureCode::ParentPathUnavailable,
                protocol::LocalBrowseItemFailureCode::EnumerationUnavailable,
            ));
        }
    };

    let mut item_keys = Vec::new();
    let mut enumeration_failure = false;
    for entry_result in directory {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(_) => {
                enumeration_failure = true;
                continue;
            }
        };
        item_keys.push(item_key_for_directory_entry(
            &request,
            &root_key,
            &parent_key,
            entry,
        ));
    }

    sort_item_keys(&mut item_keys);

    let total_items = item_keys.len();
    let key_failure = item_keys.iter().any(|key| {
        key.item_kind == protocol::LocalBrowseItemKind::Inaccessible && key.failure.is_some()
    });
    let items = item_keys
        .into_iter()
        .skip(request.offset)
        .take(request.limit)
        .map(|key| item_for_key(&request, key, admitted_source_path_keys))
        .collect::<Vec<_>>();

    let status = if enumeration_failure || key_failure {
        protocol::LocalBrowseItemsReadStatus::PartialFailure
    } else {
        protocol::LocalBrowseItemsReadStatus::Complete
    };
    let failure = if enumeration_failure || key_failure {
        Some(protocol::LocalBrowseItemFailure {
            code: protocol::LocalBrowseItemFailureCode::EnumerationUnavailable,
            detail: "one or more local browse items could not be fully resolved".to_string(),
        })
    } else {
        None
    };

    Ok(protocol::ReadLocalBrowseItemsReply {
        status,
        window_identity: window_identity(&request),
        offset: request.offset,
        limit: request.limit,
        total_items,
        items,
        failure,
    })
}

#[cfg(windows)]
fn validate_windows_path(path: &Path, field: &str) -> protocol::ProtocolResult<()> {
    if !path.is_absolute() {
        return Err(protocol::ProtocolError::InvalidRequest {
            detail: format!("readLocalBrowseItems {field} must be absolute"),
        });
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(protocol::ProtocolError::InvalidRequest {
            detail: format!("readLocalBrowseItems {field} must not contain parent segments"),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalBrowseItemKey {
    path: PathBuf,
    path_key: String,
    display_name: String,
    item_kind: protocol::LocalBrowseItemKind,
    status: protocol::LocalBrowseItemStatus,
    file_kind: Option<protocol::ContentsFileKind>,
    media_relevance: Option<protocol::LocalBrowseItemMediaRelevance>,
    parent_admission_available: bool,
    failure: Option<protocol::LocalBrowseItemFailure>,
}

#[cfg(windows)]
fn item_key_for_directory_entry(
    request: &protocol::ReadLocalBrowseItemsRequest,
    root_key: &str,
    parent_key: &str,
    entry: fs::DirEntry,
) -> LocalBrowseItemKey {
    let display_name = entry.file_name().to_string_lossy().into_owned();
    let path = entry.path();
    let path_key = normalize_local_browse_path_key(&path);
    if !path_key_is_within_or_equal(&path_key, root_key) {
        return inaccessible_key(
            path,
            path_key,
            display_name,
            protocol::LocalBrowseItemStatus::Rejected,
            protocol::LocalBrowseItemFailureCode::ParentOutsideRoot,
            "child item resolved outside rootCanonicalPath",
        );
    }

    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) => {
            let (status, code) = item_status_from_error(error.kind());
            return inaccessible_key(
                path,
                path_key,
                display_name,
                status,
                code,
                format!("failed to read item metadata: {error}"),
            );
        }
    };

    if is_reparse_point(&metadata) {
        return inaccessible_key(
            path,
            path_key,
            display_name,
            protocol::LocalBrowseItemStatus::Rejected,
            protocol::LocalBrowseItemFailureCode::ReparsePointSkipped,
            "item is a reparse point and was not followed",
        );
    }

    if metadata.is_dir() {
        let item_kind = if is_rejected_system_drive_item(
            request.entry_point_kind,
            parent_key,
            root_key,
            &display_name,
        ) {
            protocol::LocalBrowseItemKind::RejectedRoot
        } else {
            protocol::LocalBrowseItemKind::Directory
        };
        return LocalBrowseItemKey {
            path,
            path_key,
            display_name,
            item_kind,
            status: if item_kind == protocol::LocalBrowseItemKind::RejectedRoot {
                protocol::LocalBrowseItemStatus::Rejected
            } else {
                protocol::LocalBrowseItemStatus::Available
            },
            file_kind: None,
            media_relevance: None,
            parent_admission_available: false,
            failure: (item_kind == protocol::LocalBrowseItemKind::RejectedRoot).then(|| {
                protocol::LocalBrowseItemFailure {
                    code: protocol::LocalBrowseItemFailureCode::RejectedRoot,
                    detail: "system-owned directory is not directly admissible as a source root"
                        .to_string(),
                }
            }),
        };
    }

    if metadata.is_file() {
        let classification = classify_item_file(&path);
        return LocalBrowseItemKey {
            path,
            path_key,
            display_name,
            item_kind: classification.item_kind,
            status: classification.status,
            file_kind: Some(classification.file_kind),
            media_relevance: Some(classification.media_relevance),
            parent_admission_available: classification.parent_admission_available,
            failure: None,
        };
    }

    LocalBrowseItemKey {
        path,
        path_key,
        display_name,
        item_kind: protocol::LocalBrowseItemKind::Unknown,
        status: protocol::LocalBrowseItemStatus::Unknown,
        file_kind: Some(protocol::ContentsFileKind::Unknown),
        media_relevance: Some(protocol::LocalBrowseItemMediaRelevance::Unknown),
        parent_admission_available: false,
        failure: None,
    }
}

fn inaccessible_key(
    path: PathBuf,
    path_key: String,
    display_name: String,
    status: protocol::LocalBrowseItemStatus,
    failure_code: protocol::LocalBrowseItemFailureCode,
    detail: impl Into<String>,
) -> LocalBrowseItemKey {
    LocalBrowseItemKey {
        path,
        path_key,
        display_name,
        item_kind: protocol::LocalBrowseItemKind::Inaccessible,
        status,
        file_kind: None,
        media_relevance: None,
        parent_admission_available: false,
        failure: Some(protocol::LocalBrowseItemFailure {
            code: failure_code,
            detail: detail.into(),
        }),
    }
}

fn item_for_key(
    request: &protocol::ReadLocalBrowseItemsRequest,
    key: LocalBrowseItemKey,
    admitted_source_path_keys: &HashSet<String>,
) -> protocol::LocalBrowseItem {
    let duplicate = key.item_kind != protocol::LocalBrowseItemKind::RejectedRoot
        && admitted_source_path_keys.contains(&key.path_key);
    let status = if duplicate {
        protocol::LocalBrowseItemStatus::DuplicateOfAdmittedSource
    } else {
        key.status
    };
    let admission_action = admission_action_for_item(&key, status);
    let available_actions = available_actions_for_item(&key, status);

    protocol::LocalBrowseItem {
        identity: item_identity(request, &key.path),
        item_kind: key.item_kind,
        display_name: key.display_name,
        status,
        platform: local_browse_entry_point_platform(),
        file_kind: key.file_kind,
        media_relevance: key.media_relevance,
        admission_action,
        available_actions,
        failure: key.failure,
    }
}

fn admission_action_for_item(
    key: &LocalBrowseItemKey,
    status: protocol::LocalBrowseItemStatus,
) -> Option<protocol::LocalBrowseAdmissionAction> {
    if status != protocol::LocalBrowseItemStatus::Available {
        return None;
    }

    match key.item_kind {
        protocol::LocalBrowseItemKind::Directory => {
            Some(protocol::LocalBrowseAdmissionAction::RequestAdmission)
        }
        protocol::LocalBrowseItemKind::MediaFile if key.parent_admission_available => {
            Some(protocol::LocalBrowseAdmissionAction::RequestParentAdmission)
        }
        _ => None,
    }
}

fn available_actions_for_item(
    key: &LocalBrowseItemKey,
    status: protocol::LocalBrowseItemStatus,
) -> protocol::LocalBrowseAvailableActions {
    let can_browse = matches!(
        key.item_kind,
        protocol::LocalBrowseItemKind::Directory
            if status == protocol::LocalBrowseItemStatus::Available
                || status == protocol::LocalBrowseItemStatus::DuplicateOfAdmittedSource
    );
    let can_request_admission = key.item_kind == protocol::LocalBrowseItemKind::Directory
        && status == protocol::LocalBrowseItemStatus::Available;
    let can_request_parent_admission = key.item_kind == protocol::LocalBrowseItemKind::MediaFile
        && status == protocol::LocalBrowseItemStatus::Available
        && key.parent_admission_available;

    protocol::LocalBrowseAvailableActions {
        can_browse,
        can_request_admission,
        can_choose_descendant: can_browse,
        can_request_parent_admission,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ItemFileClassification {
    item_kind: protocol::LocalBrowseItemKind,
    status: protocol::LocalBrowseItemStatus,
    file_kind: protocol::ContentsFileKind,
    media_relevance: protocol::LocalBrowseItemMediaRelevance,
    parent_admission_available: bool,
}

fn classify_item_file(path: &Path) -> ItemFileClassification {
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
        Some("cue") => ItemFileClassification {
            item_kind: protocol::LocalBrowseItemKind::MediaFile,
            status: protocol::LocalBrowseItemStatus::Available,
            file_kind: protocol::ContentsFileKind::CueSheet,
            media_relevance: protocol::LocalBrowseItemMediaRelevance::CompanionMetadata,
            parent_admission_available: true,
        },
        Some("log") => unsupported_file(protocol::ContentsFileKind::LogDoc),
        Some("txt" | "nfo" | "md") => unsupported_file(protocol::ContentsFileKind::TextDoc),
        Some("zip" | "rar" | "7z" | "tar" | "gz") => {
            unsupported_file(protocol::ContentsFileKind::Archive)
        }
        Some(_) => unsupported_file(protocol::ContentsFileKind::Other),
        None => ItemFileClassification {
            item_kind: protocol::LocalBrowseItemKind::Unknown,
            status: protocol::LocalBrowseItemStatus::Unknown,
            file_kind: protocol::ContentsFileKind::Unknown,
            media_relevance: protocol::LocalBrowseItemMediaRelevance::Unknown,
            parent_admission_available: false,
        },
    }
}

fn media_file(file_kind: protocol::ContentsFileKind) -> ItemFileClassification {
    ItemFileClassification {
        item_kind: protocol::LocalBrowseItemKind::MediaFile,
        status: protocol::LocalBrowseItemStatus::Available,
        file_kind,
        media_relevance: protocol::LocalBrowseItemMediaRelevance::MediaRelevant,
        parent_admission_available: true,
    }
}

fn unsupported_file(file_kind: protocol::ContentsFileKind) -> ItemFileClassification {
    ItemFileClassification {
        item_kind: protocol::LocalBrowseItemKind::UnsupportedFile,
        status: protocol::LocalBrowseItemStatus::Available,
        file_kind,
        media_relevance: protocol::LocalBrowseItemMediaRelevance::Unsupported,
        parent_admission_available: false,
    }
}

fn item_status_from_error(
    error_kind: io::ErrorKind,
) -> (
    protocol::LocalBrowseItemStatus,
    protocol::LocalBrowseItemFailureCode,
) {
    match error_kind {
        io::ErrorKind::NotFound => (
            protocol::LocalBrowseItemStatus::Missing,
            protocol::LocalBrowseItemFailureCode::MetadataUnavailable,
        ),
        io::ErrorKind::PermissionDenied => (
            protocol::LocalBrowseItemStatus::PermissionBlocked,
            protocol::LocalBrowseItemFailureCode::PermissionDenied,
        ),
        _ => (
            protocol::LocalBrowseItemStatus::Unavailable,
            protocol::LocalBrowseItemFailureCode::MetadataUnavailable,
        ),
    }
}

fn path_failure_reply(
    request: &protocol::ReadLocalBrowseItemsRequest,
    error: io::Error,
    missing_code: protocol::LocalBrowseItemFailureCode,
    unavailable_code: protocol::LocalBrowseItemFailureCode,
) -> protocol::ReadLocalBrowseItemsReply {
    let (status, code) = match error.kind() {
        io::ErrorKind::NotFound => (protocol::LocalBrowseItemsReadStatus::Missing, missing_code),
        io::ErrorKind::PermissionDenied => (
            protocol::LocalBrowseItemsReadStatus::PermissionBlocked,
            protocol::LocalBrowseItemFailureCode::PermissionDenied,
        ),
        _ => (
            protocol::LocalBrowseItemsReadStatus::Unavailable,
            unavailable_code,
        ),
    };
    failure_reply(request, status, code, error.to_string())
}

fn failure_reply(
    request: &protocol::ReadLocalBrowseItemsRequest,
    status: protocol::LocalBrowseItemsReadStatus,
    code: protocol::LocalBrowseItemFailureCode,
    detail: impl Into<String>,
) -> protocol::ReadLocalBrowseItemsReply {
    protocol::ReadLocalBrowseItemsReply {
        status,
        window_identity: window_identity(request),
        offset: request.offset,
        limit: request.limit,
        total_items: 0,
        items: Vec::new(),
        failure: Some(protocol::LocalBrowseItemFailure {
            code,
            detail: detail.into(),
        }),
    }
}

fn item_identity(
    request: &protocol::ReadLocalBrowseItemsRequest,
    item_path: &Path,
) -> protocol::LocalBrowseItemIdentity {
    protocol::LocalBrowseItemIdentity {
        entry_point_kind: request.entry_point_kind,
        root_canonical_path: request.root_canonical_path.clone(),
        item_canonical_path: item_path.to_string_lossy().into_owned(),
    }
}

fn window_identity(
    request: &protocol::ReadLocalBrowseItemsRequest,
) -> protocol::LocalBrowseWindowIdentity {
    protocol::LocalBrowseWindowIdentity {
        entry_point_kind: request.entry_point_kind,
        root_canonical_path: request.root_canonical_path.clone(),
        parent_canonical_path: request.parent_canonical_path.clone(),
    }
}

fn path_key_is_within_or_equal(path_key: &str, root_key: &str) -> bool {
    path_key == root_key
        || path_key.starts_with(&format!("{root_key}\\"))
        || (root_key.ends_with(':') && path_key.starts_with(&format!("{root_key}\\")))
}

fn sort_item_keys(item_keys: &mut [LocalBrowseItemKey]) {
    item_keys.sort_by(|left, right| {
        item_sort_group(left.item_kind)
            .cmp(&item_sort_group(right.item_kind))
            .then_with(|| {
                normalized_sort_text(&left.display_name)
                    .cmp(&normalized_sort_text(&right.display_name))
            })
            .then_with(|| left.path_key.cmp(&right.path_key))
    });
}

fn item_sort_group(item_kind: protocol::LocalBrowseItemKind) -> u8 {
    match item_kind {
        protocol::LocalBrowseItemKind::Directory | protocol::LocalBrowseItemKind::RejectedRoot => 0,
        protocol::LocalBrowseItemKind::MediaFile => 1,
        protocol::LocalBrowseItemKind::UnsupportedFile => 2,
        protocol::LocalBrowseItemKind::Inaccessible => 3,
        protocol::LocalBrowseItemKind::Unknown => 4,
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

fn is_rejected_system_drive_item(
    entry_point_kind: protocol::LocalBrowseEntryPointKind,
    parent_key: &str,
    root_key: &str,
    display_name: &str,
) -> bool {
    entry_point_kind == protocol::LocalBrowseEntryPointKind::SystemDriveRoot
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
pub(crate) fn classify_item_file_for_test(
    path: &Path,
) -> (
    protocol::LocalBrowseItemKind,
    protocol::ContentsFileKind,
    protocol::LocalBrowseItemMediaRelevance,
    Option<protocol::LocalBrowseAdmissionAction>,
) {
    let classification = classify_item_file(path);
    let key = LocalBrowseItemKey {
        path: path.to_path_buf(),
        path_key: normalize_local_browse_path_key(path),
        display_name: path.to_string_lossy().into_owned(),
        item_kind: classification.item_kind,
        status: classification.status,
        file_kind: Some(classification.file_kind),
        media_relevance: Some(classification.media_relevance),
        parent_admission_available: classification.parent_admission_available,
        failure: None,
    };
    (
        classification.item_kind,
        classification.file_kind,
        classification.media_relevance,
        admission_action_for_item(&key, classification.status),
    )
}

#[cfg(test)]
pub(crate) fn window_item_names_for_test(
    mut names: Vec<(&str, protocol::LocalBrowseItemKind)>,
    offset: usize,
    limit: usize,
) -> (usize, Vec<String>) {
    let mut keys = names
        .drain(..)
        .map(|(name, item_kind)| LocalBrowseItemKey {
            path: PathBuf::from(name),
            path_key: normalize_local_browse_path_key(Path::new(name)),
            display_name: name.to_string(),
            item_kind,
            status: protocol::LocalBrowseItemStatus::Available,
            file_kind: None,
            media_relevance: None,
            parent_admission_available: false,
            failure: None,
        })
        .collect::<Vec<_>>();
    sort_item_keys(&mut keys);
    let total = keys.len();
    let window = keys
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|key| key.display_name)
        .collect();
    (total, window)
}
