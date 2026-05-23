use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::source_media::{SourceMediaOperation, SourceMediaWritePolicy, source_media_metadata};
use walkdir::WalkDir;

use super::{DiscoveredFileInput, DiscoveredLocationInput, DiscoveredLocationKind};

const FOLLOW_FILESYSTEM_SYMLINKS: bool = false;
const FILESYSTEM_DISCOVERY_SOURCE_MEDIA_WRITE_POLICY: SourceMediaWritePolicy =
    SourceMediaWritePolicy::read_only(SourceMediaOperation::FilesystemDiscovery);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemWalkEntry {
    pub canonical_path: String,
    pub display_name: String,
    pub kind: DiscoveredLocationKind,
    pub file_size_bytes: Option<i64>,
    pub modified_at_ns: Option<i64>,
    pub observed_at_ms: i64,
}

impl FilesystemWalkEntry {
    fn into_location_input(self) -> DiscoveredLocationInput {
        DiscoveredLocationInput {
            canonical_path: self.canonical_path,
            display_name: self.display_name,
            kind: self.kind,
            file_size_bytes: self.file_size_bytes,
            modified_at_ns: self.modified_at_ns,
            observed_at_ms: self.observed_at_ms,
        }
    }
}

pub struct FilesystemWalkEntries {
    root_path: PathBuf,
    source_media_write_policy: SourceMediaWritePolicy,
    iter: walkdir::IntoIter,
}

impl FilesystemWalkEntries {
    pub fn new(root: &Path) -> Self {
        Self::from_source_media_root(root, FILESYSTEM_DISCOVERY_SOURCE_MEDIA_WRITE_POLICY)
    }

    pub(crate) fn from_source_media_root(
        root: &Path,
        source_media_write_policy: SourceMediaWritePolicy,
    ) -> Self {
        let iter = WalkDir::new(root)
            .follow_links(FOLLOW_FILESYSTEM_SYMLINKS)
            .sort_by_file_name()
            .into_iter();
        Self {
            root_path: root.to_path_buf(),
            source_media_write_policy,
            iter,
        }
    }
}

impl fmt::Debug for FilesystemWalkEntries {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilesystemWalkEntries")
            .finish_non_exhaustive()
    }
}

impl Iterator for FilesystemWalkEntries {
    type Item = Result<DiscoveredLocationInput, std::io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entry = self.iter.next()?;
            match entry {
                Ok(entry) => {
                    let file_type = entry.file_type();
                    if !file_type.is_dir() && !file_type.is_file() {
                        continue;
                    }

                    let entry_path = entry.path();
                    let canonical_path = match normalize_relative_path(&self.root_path, entry_path)
                    {
                        Ok(path) => path,
                        Err(error) => return Some(Err(error)),
                    };
                    let display_name = match display_name_for_entry(&self.root_path, entry_path) {
                        Ok(name) => name,
                        Err(error) => return Some(Err(error)),
                    };

                    let modified_at_ns = if file_type.is_file() {
                        match source_media_metadata(self.source_media_write_policy, entry_path) {
                            Ok(metadata) => modified_at_ns_from_metadata(&metadata),
                            Err(error) => return Some(Err(error)),
                        }
                    } else {
                        entry
                            .metadata()
                            .ok()
                            .and_then(|metadata| modified_at_ns_from_metadata(&metadata))
                    };
                    let file_size_bytes = if file_type.is_file() {
                        match source_media_metadata(self.source_media_write_policy, entry_path) {
                            Ok(metadata) => i64::try_from(metadata.len()).ok(),
                            Err(error) => return Some(Err(error)),
                        }
                    } else {
                        None
                    };

                    return Some(Ok(FilesystemWalkEntry {
                        canonical_path,
                        display_name,
                        kind: if file_type.is_dir() {
                            DiscoveredLocationKind::Folder
                        } else {
                            DiscoveredLocationKind::File
                        },
                        file_size_bytes,
                        modified_at_ns,
                        observed_at_ms: unix_time_ms_now(),
                    }
                    .into_location_input()));
                }
                Err(error) => return Some(Err(std::io::Error::other(error))),
            }
        }
    }
}

pub fn collect_discovered_file_inputs(
    root_path: &Path,
) -> Result<Vec<DiscoveredFileInput>, std::io::Error> {
    let mut discovered = Vec::new();

    for entry in FilesystemWalkEntries::new(root_path) {
        let entry = entry?;
        if entry.kind != DiscoveredLocationKind::File {
            continue;
        }

        discovered.push(DiscoveredFileInput {
            canonical_path: entry.canonical_path,
            file_size_bytes: entry.file_size_bytes,
            modified_at_ns: entry.modified_at_ns,
            observed_at_ms: entry.observed_at_ms,
        });
    }

    Ok(discovered)
}

fn display_name_for_entry(root_path: &Path, entry_path: &Path) -> Result<String, std::io::Error> {
    if entry_path == root_path {
        return Ok(path_leaf_name(root_path));
    }

    let leaf = entry_path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("filesystem entry {:?} is missing a file name", entry_path),
        )
    })?;
    let display_name = leaf.to_str().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("non-UTF-8 path segment under {:?}", entry_path),
        )
    })?;

    Ok(display_name.to_string())
}

fn path_leaf_name(path: &Path) -> String {
    path.file_name()
        .and_then(|segment| segment.to_str())
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn normalize_relative_path(root_path: &Path, entry_path: &Path) -> Result<String, std::io::Error> {
    let relative_path = entry_path.strip_prefix(root_path).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "file path {:?} must stay within root path {:?}",
                entry_path, root_path
            ),
        )
    })?;

    let mut segments = Vec::new();
    for component in relative_path.components() {
        match component {
            Component::Normal(segment) => segments.push(
                segment
                    .to_str()
                    .ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("non-UTF-8 path segment under {:?}", entry_path),
                        )
                    })?
                    .to_string(),
            ),
            Component::CurDir => {}
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "relative scan path {:?} contained a non-normal component",
                        relative_path
                    ),
                ));
            }
        }
    }

    Ok(segments.join("/"))
}

fn unix_time_ms_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn modified_at_ns_from_metadata(metadata: &std::fs::Metadata) -> Option<i64> {
    metadata
        .modified()
        .ok()
        .and_then(|timestamp| timestamp.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_nanos()).ok())
}

#[cfg(test)]
mod tests {
    use super::{FilesystemWalkEntries, collect_discovered_file_inputs, normalize_relative_path};
    use crate::authority::ingest::DiscoveredLocationKind;

    #[test]
    fn filesystem_walk_entries_yield_root_folders_and_files_in_stable_order() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let root_path = tempdir.path().join("music");
        std::fs::create_dir_all(root_path.join("artists").join("alpha"))
            .expect("create nested root directory");
        std::fs::create_dir_all(root_path.join("artists").join("bravo"))
            .expect("create sibling directory");
        std::fs::write(
            root_path.join("artists").join("alpha").join("track.mp3"),
            b"mp3",
        )
        .expect("write nested file");

        let walked = FilesystemWalkEntries::new(&root_path)
            .collect::<Result<Vec<_>, _>>()
            .expect("walk filesystem");

        assert_eq!(
            walked
                .iter()
                .map(|entry| (entry.canonical_path.clone(), entry.kind.clone()))
                .collect::<Vec<_>>(),
            vec![
                ("".to_string(), DiscoveredLocationKind::Folder),
                ("artists".to_string(), DiscoveredLocationKind::Folder),
                ("artists/alpha".to_string(), DiscoveredLocationKind::Folder),
                (
                    "artists/alpha/track.mp3".to_string(),
                    DiscoveredLocationKind::File
                ),
                ("artists/bravo".to_string(), DiscoveredLocationKind::Folder),
            ]
        );
    }

    #[test]
    fn collect_discovered_file_inputs_walks_files_recursively_and_normalizes_relative_paths() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let root_path = tempdir.path().join("music");
        std::fs::create_dir_all(root_path.join("nested")).expect("create nested root directory");
        std::fs::write(root_path.join("loose.flac"), b"flac").expect("write loose file");
        std::fs::write(root_path.join("nested").join("track.mp3"), b"mp3")
            .expect("write nested file");

        let mut discovered = collect_discovered_file_inputs(&root_path).expect("collect files");
        discovered.sort_by(|left, right| left.canonical_path.cmp(&right.canonical_path));

        assert_eq!(
            discovered
                .iter()
                .map(|file| file.canonical_path.clone())
                .collect::<Vec<_>>(),
            vec!["loose.flac".to_string(), "nested/track.mp3".to_string()]
        );
    }

    #[test]
    fn normalize_relative_path_rejects_paths_outside_root() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let root_path = tempdir.path().join("music");
        let outside_path = tempdir.path().join("elsewhere").join("track.mp3");

        let error = normalize_relative_path(&root_path, &outside_path)
            .expect_err("outside-root path must be rejected");

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }
}
