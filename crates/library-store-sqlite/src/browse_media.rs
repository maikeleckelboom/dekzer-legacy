// Source-file observation owns provisional path-based file classification.
// Maintained read models should consume the stored source_files.file_kind and file_class.
#![allow(dead_code)]

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum SourceFileClass {
    Audio,
    Video,
    Image,
    Unsupported,
}

impl SourceFileClass {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            SourceFileClass::Audio => "audio",
            SourceFileClass::Video => "video",
            SourceFileClass::Image => "image",
            SourceFileClass::Unsupported => "unsupported",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceFileClassFilter {
    NavigationOnly,
    PlayableMedia,
    PlayableMediaAndImages,
}

pub(crate) fn is_playable_media_file_class(file_class: &str) -> bool {
    matches!(file_class, "audio" | "video")
}

pub(crate) fn is_image_file_class(file_class: &str) -> bool {
    file_class == "image"
}

pub(crate) fn source_file_class_filter_predicate_sql(
    source_file_class_filter: SourceFileClassFilter,
) -> &'static str {
    match source_file_class_filter {
        SourceFileClassFilter::NavigationOnly => "0 = 1",
        SourceFileClassFilter::PlayableMedia => "file_class IN ('audio', 'video')",
        SourceFileClassFilter::PlayableMediaAndImages => {
            "file_class IN ('audio', 'video', 'image')"
        }
    }
}

pub(crate) fn source_file_class_filter_predicate_sql_for_column(
    source_file_class_filter: SourceFileClassFilter,
    column_sql: &str,
) -> String {
    match source_file_class_filter {
        SourceFileClassFilter::NavigationOnly => "0 = 1".to_string(),
        SourceFileClassFilter::PlayableMedia => {
            format!("{column_sql} IN ('audio', 'video')")
        }
        SourceFileClassFilter::PlayableMediaAndImages => {
            format!("{column_sql} IN ('audio', 'video', 'image')")
        }
    }
}

pub(crate) fn canonical_file_class_from_file_kind(
    file_kind: Option<&str>,
) -> Option<SourceFileClass> {
    match file_kind? {
        "audio" => Some(SourceFileClass::Audio),
        "video" => Some(SourceFileClass::Video),
        "image" => Some(SourceFileClass::Image),
        "cue_sheet" | "log_doc" | "text_doc" | "archive" | "other" => {
            Some(SourceFileClass::Unsupported)
        }
        "unknown" => None,
        _ => None,
    }
}

pub(crate) fn canonical_file_class_from_media_kind(
    media_kind: Option<&str>,
) -> Option<SourceFileClass> {
    match media_kind? {
        "audio" => Some(SourceFileClass::Audio),
        "video" => Some(SourceFileClass::Video),
        "image" => Some(SourceFileClass::Image),
        _ => Some(SourceFileClass::Unsupported),
    }
}

pub(crate) fn provisional_file_class_from_path(path: &str) -> Option<SourceFileClass> {
    canonical_file_class_from_file_kind(Some(classify_relative_path_file_kind(path)))
}

pub(crate) fn file_class_str_from_path(relative_path: &str) -> &'static str {
    match provisional_file_class_from_path(relative_path) {
        Some(file_class) => file_class.as_str(),
        None => "none",
    }
}

pub(crate) fn file_kind_str_from_path(relative_path: &str) -> &'static str {
    classify_relative_path_file_kind(relative_path)
}

pub(crate) fn is_media_relevant_unsupported_file_kind(file_kind: &str) -> bool {
    file_kind == "cue_sheet"
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
        SourceFileClass, canonical_file_class_from_file_kind, canonical_file_class_from_media_kind,
        file_class_str_from_path, file_kind_str_from_path, is_media_relevant_unsupported_file_kind,
        provisional_file_class_from_path,
    };

    #[test]
    fn file_kind_mapping_classifies_audio_video_image_and_unsupported_classes() {
        assert_eq!(
            canonical_file_class_from_file_kind(Some("audio")),
            Some(SourceFileClass::Audio)
        );
        assert_eq!(
            canonical_file_class_from_file_kind(Some("video")),
            Some(SourceFileClass::Video)
        );
        assert_eq!(
            canonical_file_class_from_file_kind(Some("image")),
            Some(SourceFileClass::Image)
        );
        assert_eq!(
            canonical_file_class_from_file_kind(Some("cue_sheet")),
            Some(SourceFileClass::Unsupported)
        );
        assert_eq!(canonical_file_class_from_file_kind(Some("unknown")), None);
        assert_eq!(canonical_file_class_from_file_kind(None), None);
    }

    #[test]
    fn media_kind_mapping_preserves_video_and_image_and_downgrades_others_to_unsupported() {
        assert_eq!(
            canonical_file_class_from_media_kind(Some("video")),
            Some(SourceFileClass::Video)
        );
        assert_eq!(
            canonical_file_class_from_media_kind(Some("image")),
            Some(SourceFileClass::Image)
        );
        assert_eq!(
            canonical_file_class_from_media_kind(Some("other")),
            Some(SourceFileClass::Unsupported)
        );
    }

    #[test]
    fn provisional_path_classification_reuses_backend_extension_mapping() {
        assert_eq!(
            provisional_file_class_from_path("crate/track.mp3"),
            Some(SourceFileClass::Audio)
        );
        assert_eq!(
            provisional_file_class_from_path("crate/clip.mp4"),
            Some(SourceFileClass::Video)
        );
        assert_eq!(
            provisional_file_class_from_path("crate/cover.png"),
            Some(SourceFileClass::Image)
        );
        assert_eq!(
            provisional_file_class_from_path("crate/notes.txt"),
            Some(SourceFileClass::Unsupported)
        );
        assert_eq!(provisional_file_class_from_path("crate/mystery"), None);
    }

    #[test]
    fn file_class_str_from_path_classifies_wma_and_alac_as_audio() {
        assert_eq!(file_class_str_from_path("lib/track.wma"), "audio");
        assert_eq!(file_class_str_from_path("lib/track.alac"), "audio");
        assert_eq!(file_class_str_from_path("lib/track.mp3"), "audio");
        assert_eq!(file_class_str_from_path("lib/clip.mkv"), "video");
        assert_eq!(file_class_str_from_path("lib/cover.png"), "image");
        assert_eq!(file_class_str_from_path("lib/readme"), "none");
    }

    #[test]
    fn file_kind_str_from_path_preserves_cue_without_broadening_unsupported() {
        assert_eq!(file_kind_str_from_path("lib/album.cue"), "cue_sheet");
        assert_eq!(file_kind_str_from_path("lib/readme.txt"), "text_doc");
        assert_eq!(file_kind_str_from_path("lib/archive.zip"), "archive");
        assert_eq!(file_kind_str_from_path("lib/data.bin"), "other");
        assert_eq!(file_kind_str_from_path("lib/mystery"), "unknown");
        assert!(is_media_relevant_unsupported_file_kind("cue_sheet"));
        assert!(!is_media_relevant_unsupported_file_kind("text_doc"));
        assert!(!is_media_relevant_unsupported_file_kind("archive"));
        assert!(!is_media_relevant_unsupported_file_kind("other"));
        assert!(!is_media_relevant_unsupported_file_kind("unknown"));
    }

    #[test]
    fn browse_file_class_as_str_round_trips_through_database_domain() {
        assert_eq!(SourceFileClass::Audio.as_str(), "audio");
        assert_eq!(SourceFileClass::Video.as_str(), "video");
        assert_eq!(SourceFileClass::Unsupported.as_str(), "unsupported");
    }
}
