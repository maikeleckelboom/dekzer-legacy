// Media-class helpers are used by parked filesystem/path annotation flows, not
// by the maintained snapshot-read center.
#![allow(dead_code)]

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum BrowseMediaClass {
    Audio,
    Video,
    Unsupported,
}

pub(crate) fn canonical_media_class_from_file_kind(
    file_kind: Option<&str>,
) -> Option<BrowseMediaClass> {
    match file_kind? {
        "audio" => Some(BrowseMediaClass::Audio),
        "video" => Some(BrowseMediaClass::Video),
        "cue_sheet" | "image" | "log_doc" | "text_doc" | "archive" | "other" => {
            Some(BrowseMediaClass::Unsupported)
        }
        "unknown" => None,
        _ => None,
    }
}

pub(crate) fn canonical_media_class_from_media_kind(
    media_kind: Option<&str>,
) -> Option<BrowseMediaClass> {
    match media_kind? {
        "audio" => Some(BrowseMediaClass::Audio),
        "video" => Some(BrowseMediaClass::Video),
        _ => Some(BrowseMediaClass::Unsupported),
    }
}

pub(crate) fn provisional_media_class_from_path(path: &str) -> Option<BrowseMediaClass> {
    canonical_media_class_from_file_kind(Some(classify_relative_path_file_kind(path)))
}

pub(crate) fn classify_relative_path_file_kind(relative_path: &str) -> &'static str {
    let Some(extension) = Path::new(relative_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
    else {
        return "unknown";
    };

    match extension.as_str() {
        "mp3" | "wav" | "flac" | "aiff" | "aif" | "aifc" | "m4a" | "aac" | "ogg" | "oga"
        | "opus" | "wma" | "alac" => "audio",
        "cue" => "cue_sheet",
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tif" | "tiff" => "image",
        "mp4" | "mov" | "m4v" | "webm" | "mkv" | "avi" => "video",
        "log" | "pdf" | "rtf" => "log_doc",
        "psd" => "image",
        "txt" | "md" | "nfo" | "json" | "csv" => "text_doc",
        "zip" | "rar" | "7z" | "tar" | "gz" => "archive",
        "bin" | "dat" => "other",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BrowseMediaClass, canonical_media_class_from_file_kind,
        canonical_media_class_from_media_kind, provisional_media_class_from_path,
    };

    #[test]
    fn file_kind_mapping_classifies_audio_video_and_unsupported_classes() {
        assert_eq!(
            canonical_media_class_from_file_kind(Some("audio")),
            Some(BrowseMediaClass::Audio)
        );
        assert_eq!(
            canonical_media_class_from_file_kind(Some("video")),
            Some(BrowseMediaClass::Video)
        );
        assert_eq!(
            canonical_media_class_from_file_kind(Some("image")),
            Some(BrowseMediaClass::Unsupported)
        );
        assert_eq!(canonical_media_class_from_file_kind(Some("unknown")), None);
        assert_eq!(canonical_media_class_from_file_kind(None), None);
    }

    #[test]
    fn media_kind_mapping_preserves_video_and_downgrades_non_dj_visuals_to_unsupported() {
        assert_eq!(
            canonical_media_class_from_media_kind(Some("video")),
            Some(BrowseMediaClass::Video)
        );
        assert_eq!(
            canonical_media_class_from_media_kind(Some("image")),
            Some(BrowseMediaClass::Unsupported)
        );
    }

    #[test]
    fn provisional_path_classification_reuses_backend_extension_mapping() {
        assert_eq!(
            provisional_media_class_from_path("crate/track.mp3"),
            Some(BrowseMediaClass::Audio)
        );
        assert_eq!(
            provisional_media_class_from_path("crate/clip.mp4"),
            Some(BrowseMediaClass::Video)
        );
        assert_eq!(
            provisional_media_class_from_path("crate/notes.txt"),
            Some(BrowseMediaClass::Unsupported)
        );
        assert_eq!(provisional_media_class_from_path("crate/mystery"), None);
    }
}
