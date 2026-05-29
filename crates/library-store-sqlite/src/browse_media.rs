// Source-file observation owns provisional path-based media classification.
// Maintained read models should consume the stored source_files.media_class.
#![allow(dead_code)]

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum BrowseMediaClass {
    Audio,
    Video,
    Image,
    Unsupported,
}

impl BrowseMediaClass {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            BrowseMediaClass::Audio => "audio",
            BrowseMediaClass::Video => "video",
            BrowseMediaClass::Image => "image",
            BrowseMediaClass::Unsupported => "unsupported",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LibraryTreeRowAdmission {
    Performance,
    PerformanceAndImages,
}

pub(crate) fn is_primary_media_class(media_class: &str) -> bool {
    matches!(media_class, "audio" | "video")
}

pub(crate) fn is_image_media_class(media_class: &str) -> bool {
    media_class == "image"
}

pub(crate) fn library_tree_row_admission_predicate_sql(
    library_tree_row_admission: LibraryTreeRowAdmission,
) -> &'static str {
    match library_tree_row_admission {
        LibraryTreeRowAdmission::Performance => "media_class IN ('audio', 'video')",
        LibraryTreeRowAdmission::PerformanceAndImages => "media_class IN ('audio', 'video', 'image')",
    }
}

pub(crate) fn library_tree_row_admission_predicate_sql_for_column(
    library_tree_row_admission: LibraryTreeRowAdmission,
    column_sql: &str,
) -> String {
    match library_tree_row_admission {
        LibraryTreeRowAdmission::Performance => {
            format!("{column_sql} IN ('audio', 'video')")
        }
        LibraryTreeRowAdmission::PerformanceAndImages => {
            format!("{column_sql} IN ('audio', 'video', 'image')")
        }
    }
}

pub(crate) fn canonical_media_class_from_file_kind(
    file_kind: Option<&str>,
) -> Option<BrowseMediaClass> {
    match file_kind? {
        "audio" => Some(BrowseMediaClass::Audio),
        "video" => Some(BrowseMediaClass::Video),
        "image" => Some(BrowseMediaClass::Image),
        "cue_sheet" | "log_doc" | "text_doc" | "archive" | "other" => {
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
        "image" => Some(BrowseMediaClass::Image),
        _ => Some(BrowseMediaClass::Unsupported),
    }
}

pub(crate) fn provisional_media_class_from_path(path: &str) -> Option<BrowseMediaClass> {
    canonical_media_class_from_file_kind(Some(classify_relative_path_file_kind(path)))
}

pub(crate) fn media_class_str_from_path(relative_path: &str) -> &'static str {
    match provisional_media_class_from_path(relative_path) {
        Some(media_class) => media_class.as_str(),
        None => "none",
    }
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
        canonical_media_class_from_media_kind, media_class_str_from_path,
        provisional_media_class_from_path,
    };

    #[test]
    fn file_kind_mapping_classifies_audio_video_image_and_unsupported_classes() {
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
            Some(BrowseMediaClass::Image)
        );
        assert_eq!(
            canonical_media_class_from_file_kind(Some("cue_sheet")),
            Some(BrowseMediaClass::Unsupported)
        );
        assert_eq!(canonical_media_class_from_file_kind(Some("unknown")), None);
        assert_eq!(canonical_media_class_from_file_kind(None), None);
    }

    #[test]
    fn media_kind_mapping_preserves_video_and_image_and_downgrades_others_to_unsupported() {
        assert_eq!(
            canonical_media_class_from_media_kind(Some("video")),
            Some(BrowseMediaClass::Video)
        );
        assert_eq!(
            canonical_media_class_from_media_kind(Some("image")),
            Some(BrowseMediaClass::Image)
        );
        assert_eq!(
            canonical_media_class_from_media_kind(Some("other")),
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
            provisional_media_class_from_path("crate/cover.png"),
            Some(BrowseMediaClass::Image)
        );
        assert_eq!(
            provisional_media_class_from_path("crate/notes.txt"),
            Some(BrowseMediaClass::Unsupported)
        );
        assert_eq!(provisional_media_class_from_path("crate/mystery"), None);
    }

    #[test]
    fn media_class_str_from_path_classifies_wma_and_alac_as_audio() {
        assert_eq!(media_class_str_from_path("lib/track.wma"), "audio");
        assert_eq!(media_class_str_from_path("lib/track.alac"), "audio");
        assert_eq!(media_class_str_from_path("lib/track.mp3"), "audio");
        assert_eq!(media_class_str_from_path("lib/clip.mkv"), "video");
        assert_eq!(media_class_str_from_path("lib/cover.png"), "image");
        assert_eq!(media_class_str_from_path("lib/readme"), "none");
    }

    #[test]
    fn browse_media_class_as_str_round_trips_through_database_domain() {
        assert_eq!(BrowseMediaClass::Audio.as_str(), "audio");
        assert_eq!(BrowseMediaClass::Video.as_str(), "video");
        assert_eq!(BrowseMediaClass::Unsupported.as_str(), "unsupported");
    }
}
