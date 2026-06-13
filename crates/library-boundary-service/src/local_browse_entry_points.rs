use std::path::{Path, PathBuf};
use std::sync::Arc;

use library_boundary_protocol as protocol;

#[cfg(windows)]
use std::os::windows::ffi::{OsStrExt, OsStringExt};

pub(crate) trait LocalBrowseEntryPointResolver: Send + Sync {
    fn resolve_entry_points(
        &self,
    ) -> Result<LocalBrowseEntryPointResolution, LocalBrowseEntryPointResolveFailure>;
}

pub(crate) trait LocalBrowsePathStatusResolver: Send + Sync {
    fn resolve_status(&self, path: &Path) -> LocalBrowsePathStatusProbe;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalBrowsePathStatusProbe {
    Directory,
    NonDirectory,
    NotFound,
    PermissionBlocked,
    Unavailable,
}

impl LocalBrowsePathStatusProbe {
    fn status(self) -> protocol::LocalBrowseEntryPointStatus {
        match self {
            Self::Directory => protocol::LocalBrowseEntryPointStatus::Available,
            Self::NonDirectory | Self::Unavailable => {
                protocol::LocalBrowseEntryPointStatus::Unavailable
            }
            Self::NotFound => protocol::LocalBrowseEntryPointStatus::Missing,
            Self::PermissionBlocked => protocol::LocalBrowseEntryPointStatus::PermissionBlocked,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalBrowseEntryPointResolution {
    pub(crate) entries: Vec<ResolvedLocalBrowseEntryPoint>,
    pub(crate) failure: Option<LocalBrowseEntryPointResolveFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedLocalBrowseEntryPoint {
    pub(crate) entry_point_kind: protocol::LocalBrowseEntryPointKind,
    pub(crate) resolved_path: Option<PathBuf>,
    pub(crate) display_name: String,
    pub(crate) status: protocol::LocalBrowseEntryPointStatus,
    pub(crate) platform: protocol::LocalBrowsePlatform,
    pub(crate) failure: Option<LocalBrowseEntryPointResolveFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalBrowseEntryPointResolveFailure {
    pub(crate) code: protocol::LocalBrowseEntryPointFailureCode,
    pub(crate) detail: String,
}

pub(crate) fn production_local_browse_entry_point_resolver()
-> Arc<dyn LocalBrowseEntryPointResolver> {
    Arc::new(PlatformLocalBrowseEntryPointResolver)
}

pub(crate) struct PlatformLocalBrowsePathStatusResolver;

impl LocalBrowsePathStatusResolver for PlatformLocalBrowsePathStatusResolver {
    fn resolve_status(&self, path: &Path) -> LocalBrowsePathStatusProbe {
        resolve_platform_path_status(path)
    }
}

pub(crate) fn local_browse_entry_point_platform() -> protocol::LocalBrowsePlatform {
    #[cfg(windows)]
    {
        protocol::LocalBrowsePlatform::Windows
    }
    #[cfg(target_os = "macos")]
    {
        protocol::LocalBrowsePlatform::Macos
    }
    #[cfg(target_os = "linux")]
    {
        protocol::LocalBrowsePlatform::Linux
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        protocol::LocalBrowsePlatform::Unsupported
    }
}

struct PlatformLocalBrowseEntryPointResolver;

impl LocalBrowseEntryPointResolver for PlatformLocalBrowseEntryPointResolver {
    fn resolve_entry_points(
        &self,
    ) -> Result<LocalBrowseEntryPointResolution, LocalBrowseEntryPointResolveFailure> {
        resolve_platform_entry_points()
    }
}

fn entry_failure(
    code: protocol::LocalBrowseEntryPointFailureCode,
    detail: impl Into<String>,
) -> LocalBrowseEntryPointResolveFailure {
    LocalBrowseEntryPointResolveFailure {
        code,
        detail: detail.into(),
    }
}

fn entry(
    entry_point_kind: protocol::LocalBrowseEntryPointKind,
    resolved_path: Option<PathBuf>,
    display_name: impl Into<String>,
    status: protocol::LocalBrowseEntryPointStatus,
    failure: Option<LocalBrowseEntryPointResolveFailure>,
) -> ResolvedLocalBrowseEntryPoint {
    ResolvedLocalBrowseEntryPoint {
        entry_point_kind,
        resolved_path,
        display_name: display_name.into(),
        status,
        platform: local_browse_entry_point_platform(),
        failure,
    }
}

pub(crate) fn status_for_path(path: &Path) -> protocol::LocalBrowseEntryPointStatus {
    status_for_path_with_resolver(&PlatformLocalBrowsePathStatusResolver, path)
}

pub(crate) fn status_for_path_with_resolver(
    resolver: &dyn LocalBrowsePathStatusResolver,
    path: &Path,
) -> protocol::LocalBrowseEntryPointStatus {
    resolver.resolve_status(path).status()
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn resolve_platform_path_status(path: &Path) -> LocalBrowsePathStatusProbe {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, GetFileAttributesW, INVALID_FILE_ATTRIBUTES,
    };

    let attributes = unsafe { GetFileAttributesW(nul_terminated_wide(path).as_ptr()) };
    if attributes != INVALID_FILE_ATTRIBUTES {
        if attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
            return LocalBrowsePathStatusProbe::Directory;
        }
        return LocalBrowsePathStatusProbe::NonDirectory;
    }

    windows_path_status_probe_from_error(unsafe { GetLastError() })
}

#[cfg(windows)]
pub(crate) fn windows_path_status_probe_from_error(error: u32) -> LocalBrowsePathStatusProbe {
    use windows_sys::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_ACCESS_DENIED_APPDATA, ERROR_BAD_NETPATH,
        ERROR_CLOUD_FILE_ACCESS_DENIED, ERROR_CLOUD_FILE_AUTHENTICATION_FAILED,
        ERROR_CLOUD_FILE_NETWORK_UNAVAILABLE, ERROR_CLOUD_FILE_PROVIDER_NOT_RUNNING,
        ERROR_CLOUD_FILE_PROVIDER_TERMINATED, ERROR_CLOUD_FILE_REQUEST_TIMEOUT,
        ERROR_CLOUD_FILE_UNSUCCESSFUL, ERROR_FILE_NOT_FOUND, ERROR_NETWORK_UNREACHABLE,
        ERROR_NOT_READY, ERROR_PATH_NOT_FOUND,
    };

    match error {
        ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => LocalBrowsePathStatusProbe::NotFound,
        ERROR_ACCESS_DENIED
        | ERROR_ACCESS_DENIED_APPDATA
        | ERROR_CLOUD_FILE_ACCESS_DENIED
        | ERROR_CLOUD_FILE_AUTHENTICATION_FAILED => LocalBrowsePathStatusProbe::PermissionBlocked,
        ERROR_NOT_READY
        | ERROR_BAD_NETPATH
        | ERROR_NETWORK_UNREACHABLE
        | ERROR_CLOUD_FILE_NETWORK_UNAVAILABLE
        | ERROR_CLOUD_FILE_PROVIDER_NOT_RUNNING
        | ERROR_CLOUD_FILE_PROVIDER_TERMINATED
        | ERROR_CLOUD_FILE_REQUEST_TIMEOUT
        | ERROR_CLOUD_FILE_UNSUCCESSFUL => LocalBrowsePathStatusProbe::Unavailable,
        _ => LocalBrowsePathStatusProbe::Unavailable,
    }
}

#[cfg(not(windows))]
fn resolve_platform_path_status(path: &Path) -> LocalBrowsePathStatusProbe {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => LocalBrowsePathStatusProbe::Directory,
        Ok(_) => LocalBrowsePathStatusProbe::NonDirectory,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            LocalBrowsePathStatusProbe::NotFound
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            LocalBrowsePathStatusProbe::PermissionBlocked
        }
        Err(_) => LocalBrowsePathStatusProbe::Unavailable,
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn resolve_platform_entry_points()
-> Result<LocalBrowseEntryPointResolution, LocalBrowseEntryPointResolveFailure> {
    use std::ffi::OsString;
    use std::ptr;

    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{
        GetDriveTypeW, GetLogicalDrives, GetVolumePathNameW,
    };
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::System::SystemInformation::GetSystemWindowsDirectoryW;
    use windows_sys::Win32::System::WindowsProgramming::{DRIVE_FIXED, DRIVE_REMOVABLE};
    use windows_sys::Win32::UI::Shell::{
        FOLDERID_Desktop, FOLDERID_Downloads, FOLDERID_Music, FOLDERID_Profile,
        SHGetKnownFolderPath,
    };
    use windows_sys::core::{GUID, PCWSTR, PWSTR};

    const MAX_WINDOWS_PATH_BUFFER: usize = 32_768;

    let mut entries = Vec::new();
    let mut resolution_failure = None;

    let system_drive_root = match resolve_system_drive_root() {
        Ok(path) => {
            let status = status_for_path(&path);
            entries.push(entry(
                protocol::LocalBrowseEntryPointKind::SystemDriveRoot,
                Some(path.clone()),
                path_to_display_name(&path),
                status,
                None,
            ));
            Some(path)
        }
        Err(failure) => {
            entries.push(entry(
                protocol::LocalBrowseEntryPointKind::SystemDriveRoot,
                None,
                "System Drive",
                protocol::LocalBrowseEntryPointStatus::Unavailable,
                Some(failure),
            ));
            None
        }
    };

    match resolve_windows_volume_roots() {
        Ok(volume_roots) => {
            let system_key = system_drive_root
                .as_deref()
                .map(crate::local_browse_entry_points::normalize_local_browse_path_key);
            for volume in volume_roots {
                if volume.drive_type == DRIVE_FIXED {
                    if system_key.as_deref()
                        == Some(normalize_local_browse_path_key(&volume.path).as_str())
                    {
                        continue;
                    }
                    entries.push(entry(
                        protocol::LocalBrowseEntryPointKind::LocalDataVolumeRoot,
                        Some(volume.path.clone()),
                        path_to_display_name(&volume.path),
                        status_for_path(&volume.path),
                        None,
                    ));
                } else if volume.drive_type == DRIVE_REMOVABLE {
                    entries.push(entry(
                        protocol::LocalBrowseEntryPointKind::RemovableVolumeRoot,
                        Some(volume.path.clone()),
                        path_to_display_name(&volume.path),
                        status_for_path(&volume.path),
                        None,
                    ));
                }
            }
        }
        Err(failure) => {
            resolution_failure = Some(failure);
        }
    }

    entries.push(resolve_known_folder_entry(
        protocol::LocalBrowseEntryPointKind::UserHome,
        &FOLDERID_Profile,
        "Home",
    ));
    entries.push(resolve_known_folder_entry(
        protocol::LocalBrowseEntryPointKind::Desktop,
        &FOLDERID_Desktop,
        "Desktop",
    ));
    entries.push(resolve_known_folder_entry(
        protocol::LocalBrowseEntryPointKind::Downloads,
        &FOLDERID_Downloads,
        "Downloads",
    ));
    entries.push(resolve_known_folder_entry(
        protocol::LocalBrowseEntryPointKind::Music,
        &FOLDERID_Music,
        "Music",
    ));

    return Ok(LocalBrowseEntryPointResolution {
        entries,
        failure: resolution_failure,
    });

    struct WindowsVolumeRoot {
        path: PathBuf,
        drive_type: u32,
    }

    fn resolve_known_folder_entry(
        entry_point_kind: protocol::LocalBrowseEntryPointKind,
        folder_id: &GUID,
        display_name: &'static str,
    ) -> ResolvedLocalBrowseEntryPoint {
        match known_folder_path(folder_id) {
            Ok(path) => entry(
                entry_point_kind,
                Some(path.clone()),
                display_name,
                status_for_path(&path),
                None,
            ),
            Err(failure) => entry(
                entry_point_kind,
                None,
                display_name,
                protocol::LocalBrowseEntryPointStatus::Unavailable,
                Some(failure),
            ),
        }
    }

    fn known_folder_path(folder_id: &GUID) -> Result<PathBuf, LocalBrowseEntryPointResolveFailure> {
        let mut raw_path: PWSTR = ptr::null_mut();
        let result = unsafe {
            SHGetKnownFolderPath(
                folder_id as *const GUID,
                0,
                0 as HANDLE,
                &mut raw_path as *mut PWSTR,
            )
        };
        if result < 0 || raw_path.is_null() {
            return Err(entry_failure(
                protocol::LocalBrowseEntryPointFailureCode::KnownFolderUnavailable,
                format!("SHGetKnownFolderPath failed with HRESULT {result:#x}"),
            ));
        }

        let path = unsafe { wide_ptr_to_path_buf(raw_path as PCWSTR) };
        unsafe {
            CoTaskMemFree(raw_path.cast());
        }
        path.ok_or_else(|| {
            entry_failure(
                protocol::LocalBrowseEntryPointFailureCode::KnownFolderUnavailable,
                "SHGetKnownFolderPath returned an empty path",
            )
        })
    }

    fn resolve_system_drive_root() -> Result<PathBuf, LocalBrowseEntryPointResolveFailure> {
        let windows_directory = windows_directory_path()?;
        volume_path_for(&windows_directory)
    }

    fn windows_directory_path() -> Result<PathBuf, LocalBrowseEntryPointResolveFailure> {
        let mut buffer = vec![0_u16; MAX_WINDOWS_PATH_BUFFER];
        let length = unsafe {
            GetSystemWindowsDirectoryW(
                buffer.as_mut_ptr(),
                u32::try_from(buffer.len()).expect("Windows path buffer fits u32"),
            )
        };
        if length == 0 || usize::try_from(length).map_or(true, |len| len >= buffer.len()) {
            return Err(entry_failure(
                protocol::LocalBrowseEntryPointFailureCode::SystemDriveUnavailable,
                "GetSystemWindowsDirectoryW failed to resolve the system Windows directory",
            ));
        }
        buffer.truncate(usize::try_from(length).expect("Windows path length fits usize"));
        Ok(PathBuf::from(OsString::from_wide(&buffer)))
    }

    fn volume_path_for(path: &Path) -> Result<PathBuf, LocalBrowseEntryPointResolveFailure> {
        let path_wide = nul_terminated_wide(path);
        let mut buffer = vec![0_u16; MAX_WINDOWS_PATH_BUFFER];
        let success = unsafe {
            GetVolumePathNameW(
                path_wide.as_ptr(),
                buffer.as_mut_ptr(),
                u32::try_from(buffer.len()).expect("Windows path buffer fits u32"),
            )
        };
        if success == 0 {
            return Err(entry_failure(
                protocol::LocalBrowseEntryPointFailureCode::SystemDriveUnavailable,
                "GetVolumePathNameW failed to resolve the system volume root",
            ));
        }
        wide_buffer_to_path_buf(&buffer).ok_or_else(|| {
            entry_failure(
                protocol::LocalBrowseEntryPointFailureCode::SystemDriveUnavailable,
                "GetVolumePathNameW returned an empty system volume root",
            )
        })
    }

    fn resolve_windows_volume_roots()
    -> Result<Vec<WindowsVolumeRoot>, LocalBrowseEntryPointResolveFailure> {
        let logical_drives = unsafe { GetLogicalDrives() };
        if logical_drives == 0 {
            return Err(entry_failure(
                protocol::LocalBrowseEntryPointFailureCode::VolumeEnumerationUnavailable,
                "GetLogicalDrives failed while resolving local browse entry points",
            ));
        }

        let mut roots = Vec::new();
        for index in 0_u8..26 {
            let mask = 1_u32 << index;
            if logical_drives & mask == 0 {
                continue;
            }
            let letter = char::from(b'A' + index);
            let path = PathBuf::from(format!("{letter}:\\"));
            let path_wide = nul_terminated_wide(&path);
            let drive_type = unsafe { GetDriveTypeW(path_wide.as_ptr()) };
            if drive_type == DRIVE_FIXED || drive_type == DRIVE_REMOVABLE {
                roots.push(WindowsVolumeRoot { path, drive_type });
            }
        }
        Ok(roots)
    }

    unsafe fn wide_ptr_to_path_buf(ptr: PCWSTR) -> Option<PathBuf> {
        if ptr.is_null() {
            return None;
        }
        let mut length = 0_usize;
        while unsafe { *ptr.add(length) } != 0 {
            length += 1;
        }
        if length == 0 {
            return None;
        }
        let slice = unsafe { std::slice::from_raw_parts(ptr, length) };
        Some(PathBuf::from(OsString::from_wide(slice)))
    }

    fn wide_buffer_to_path_buf(buffer: &[u16]) -> Option<PathBuf> {
        let length = buffer.iter().position(|value| *value == 0)?;
        if length == 0 {
            return None;
        }
        Some(PathBuf::from(OsString::from_wide(&buffer[..length])))
    }
}

#[cfg(windows)]
fn nul_terminated_wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

#[cfg(not(windows))]
fn resolve_platform_entry_points()
-> Result<LocalBrowseEntryPointResolution, LocalBrowseEntryPointResolveFailure> {
    let platform = local_browse_entry_point_platform();
    let entries = [
        protocol::LocalBrowseEntryPointKind::SystemDriveRoot,
        protocol::LocalBrowseEntryPointKind::LocalDataVolumeRoot,
        protocol::LocalBrowseEntryPointKind::RemovableVolumeRoot,
        protocol::LocalBrowseEntryPointKind::UserHome,
        protocol::LocalBrowseEntryPointKind::Desktop,
        protocol::LocalBrowseEntryPointKind::Downloads,
        protocol::LocalBrowseEntryPointKind::Music,
    ]
    .into_iter()
    .map(|kind| ResolvedLocalBrowseEntryPoint {
        entry_point_kind: kind,
        resolved_path: None,
        display_name: default_display_name(kind).to_string(),
        status: protocol::LocalBrowseEntryPointStatus::UnsupportedPlatform,
        platform,
        failure: Some(entry_failure(
            protocol::LocalBrowseEntryPointFailureCode::UnsupportedPlatform,
            "local browse entry point resolution is Windows-only in V0",
        )),
    })
    .collect();

    Ok(LocalBrowseEntryPointResolution {
        entries,
        failure: None,
    })
}

pub(crate) fn normalize_local_browse_path_key(path: &Path) -> String {
    let mut text = path.to_string_lossy().replace('/', "\\");
    if let Some(stripped) = text.strip_prefix("\\\\?\\UNC\\") {
        text = format!("\\\\{stripped}");
    } else if let Some(stripped) = text.strip_prefix("\\\\?\\") {
        text = stripped.to_string();
    }

    while text.len() > 1 && (text.ends_with('\\') || text.ends_with('/')) {
        text.pop();
    }

    text.to_ascii_lowercase()
}

fn path_to_display_name(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(not(windows))]
fn default_display_name(kind: protocol::LocalBrowseEntryPointKind) -> &'static str {
    match kind {
        protocol::LocalBrowseEntryPointKind::SystemDriveRoot => "System Drive",
        protocol::LocalBrowseEntryPointKind::LocalDataVolumeRoot => "Local Data Volume",
        protocol::LocalBrowseEntryPointKind::RemovableVolumeRoot => "Removable Volume",
        protocol::LocalBrowseEntryPointKind::UserHome => "Home",
        protocol::LocalBrowseEntryPointKind::Desktop => "Desktop",
        protocol::LocalBrowseEntryPointKind::Downloads => "Downloads",
        protocol::LocalBrowseEntryPointKind::Music => "Music",
    }
}
