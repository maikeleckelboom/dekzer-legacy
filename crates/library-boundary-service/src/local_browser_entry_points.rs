use std::path::{Path, PathBuf};
use std::sync::Arc;

use library_boundary_protocol as protocol;

#[cfg(windows)]
use std::os::windows::ffi::{OsStrExt, OsStringExt};

pub(crate) trait LocalBrowserEntryPointResolver: Send + Sync {
    fn resolve_entry_points(
        &self,
    ) -> Result<LocalBrowserEntryPointResolution, LocalBrowserEntryPointResolveFailure>;
}

pub(crate) trait LocalBrowserPathStatusResolver: Send + Sync {
    fn resolve_status(&self, path: &Path) -> LocalBrowserPathStatusProbe;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalBrowserPathStatusProbe {
    Directory,
    NonDirectory,
    NotFound,
    PermissionBlocked,
    Unavailable,
}

impl LocalBrowserPathStatusProbe {
    fn status(self) -> protocol::LocalBrowserEntryPointStatus {
        match self {
            Self::Directory => protocol::LocalBrowserEntryPointStatus::Available,
            Self::NonDirectory | Self::Unavailable => {
                protocol::LocalBrowserEntryPointStatus::Unavailable
            }
            Self::NotFound => protocol::LocalBrowserEntryPointStatus::Missing,
            Self::PermissionBlocked => protocol::LocalBrowserEntryPointStatus::PermissionBlocked,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalBrowserEntryPointResolution {
    pub(crate) entries: Vec<ResolvedLocalBrowserEntryPoint>,
    pub(crate) failure: Option<LocalBrowserEntryPointResolveFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedLocalBrowserEntryPoint {
    pub(crate) entry_point_kind: protocol::LocalBrowserEntryPointKind,
    pub(crate) canonical_path: Option<PathBuf>,
    pub(crate) display_name: String,
    pub(crate) status: protocol::LocalBrowserEntryPointStatus,
    pub(crate) platform: protocol::LocalBrowserEntryPointPlatform,
    pub(crate) failure: Option<LocalBrowserEntryPointResolveFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalBrowserEntryPointResolveFailure {
    pub(crate) code: protocol::LocalBrowserEntryPointFailureCode,
    pub(crate) detail: String,
}

pub(crate) fn production_local_browser_entry_point_resolver()
-> Arc<dyn LocalBrowserEntryPointResolver> {
    Arc::new(PlatformLocalBrowserEntryPointResolver)
}

pub(crate) struct PlatformLocalBrowserPathStatusResolver;

impl LocalBrowserPathStatusResolver for PlatformLocalBrowserPathStatusResolver {
    fn resolve_status(&self, path: &Path) -> LocalBrowserPathStatusProbe {
        resolve_platform_path_status(path)
    }
}

pub(crate) fn local_browser_entry_point_platform() -> protocol::LocalBrowserEntryPointPlatform {
    #[cfg(windows)]
    {
        protocol::LocalBrowserEntryPointPlatform::Windows
    }
    #[cfg(target_os = "macos")]
    {
        protocol::LocalBrowserEntryPointPlatform::Macos
    }
    #[cfg(target_os = "linux")]
    {
        protocol::LocalBrowserEntryPointPlatform::Linux
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        protocol::LocalBrowserEntryPointPlatform::Unsupported
    }
}

struct PlatformLocalBrowserEntryPointResolver;

impl LocalBrowserEntryPointResolver for PlatformLocalBrowserEntryPointResolver {
    fn resolve_entry_points(
        &self,
    ) -> Result<LocalBrowserEntryPointResolution, LocalBrowserEntryPointResolveFailure> {
        resolve_platform_entry_points()
    }
}

fn entry_failure(
    code: protocol::LocalBrowserEntryPointFailureCode,
    detail: impl Into<String>,
) -> LocalBrowserEntryPointResolveFailure {
    LocalBrowserEntryPointResolveFailure {
        code,
        detail: detail.into(),
    }
}

fn entry(
    entry_point_kind: protocol::LocalBrowserEntryPointKind,
    canonical_path: Option<PathBuf>,
    display_name: impl Into<String>,
    status: protocol::LocalBrowserEntryPointStatus,
    failure: Option<LocalBrowserEntryPointResolveFailure>,
) -> ResolvedLocalBrowserEntryPoint {
    ResolvedLocalBrowserEntryPoint {
        entry_point_kind,
        canonical_path,
        display_name: display_name.into(),
        status,
        platform: local_browser_entry_point_platform(),
        failure,
    }
}

pub(crate) fn status_for_path(path: &Path) -> protocol::LocalBrowserEntryPointStatus {
    status_for_path_with_resolver(&PlatformLocalBrowserPathStatusResolver, path)
}

pub(crate) fn status_for_path_with_resolver(
    resolver: &dyn LocalBrowserPathStatusResolver,
    path: &Path,
) -> protocol::LocalBrowserEntryPointStatus {
    resolver.resolve_status(path).status()
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn resolve_platform_path_status(path: &Path) -> LocalBrowserPathStatusProbe {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, GetFileAttributesW, INVALID_FILE_ATTRIBUTES,
    };

    let attributes = unsafe { GetFileAttributesW(nul_terminated_wide(path).as_ptr()) };
    if attributes != INVALID_FILE_ATTRIBUTES {
        if attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
            return LocalBrowserPathStatusProbe::Directory;
        }
        return LocalBrowserPathStatusProbe::NonDirectory;
    }

    windows_path_status_probe_from_error(unsafe { GetLastError() })
}

#[cfg(windows)]
pub(crate) fn windows_path_status_probe_from_error(error: u32) -> LocalBrowserPathStatusProbe {
    use windows_sys::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_ACCESS_DENIED_APPDATA, ERROR_BAD_NETPATH,
        ERROR_CLOUD_FILE_ACCESS_DENIED, ERROR_CLOUD_FILE_AUTHENTICATION_FAILED,
        ERROR_CLOUD_FILE_NETWORK_UNAVAILABLE, ERROR_CLOUD_FILE_PROVIDER_NOT_RUNNING,
        ERROR_CLOUD_FILE_PROVIDER_TERMINATED, ERROR_CLOUD_FILE_REQUEST_TIMEOUT,
        ERROR_CLOUD_FILE_UNSUCCESSFUL, ERROR_FILE_NOT_FOUND, ERROR_NETWORK_UNREACHABLE,
        ERROR_NOT_READY, ERROR_PATH_NOT_FOUND,
    };

    match error {
        ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => LocalBrowserPathStatusProbe::NotFound,
        ERROR_ACCESS_DENIED
        | ERROR_ACCESS_DENIED_APPDATA
        | ERROR_CLOUD_FILE_ACCESS_DENIED
        | ERROR_CLOUD_FILE_AUTHENTICATION_FAILED => LocalBrowserPathStatusProbe::PermissionBlocked,
        ERROR_NOT_READY
        | ERROR_BAD_NETPATH
        | ERROR_NETWORK_UNREACHABLE
        | ERROR_CLOUD_FILE_NETWORK_UNAVAILABLE
        | ERROR_CLOUD_FILE_PROVIDER_NOT_RUNNING
        | ERROR_CLOUD_FILE_PROVIDER_TERMINATED
        | ERROR_CLOUD_FILE_REQUEST_TIMEOUT
        | ERROR_CLOUD_FILE_UNSUCCESSFUL => LocalBrowserPathStatusProbe::Unavailable,
        _ => LocalBrowserPathStatusProbe::Unavailable,
    }
}

#[cfg(not(windows))]
fn resolve_platform_path_status(path: &Path) -> LocalBrowserPathStatusProbe {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => LocalBrowserPathStatusProbe::Directory,
        Ok(_) => LocalBrowserPathStatusProbe::NonDirectory,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            LocalBrowserPathStatusProbe::NotFound
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            LocalBrowserPathStatusProbe::PermissionBlocked
        }
        Err(_) => LocalBrowserPathStatusProbe::Unavailable,
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn resolve_platform_entry_points()
-> Result<LocalBrowserEntryPointResolution, LocalBrowserEntryPointResolveFailure> {
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
                protocol::LocalBrowserEntryPointKind::SystemDriveRoot,
                Some(path.clone()),
                path_to_display_name(&path),
                status,
                None,
            ));
            Some(path)
        }
        Err(failure) => {
            entries.push(entry(
                protocol::LocalBrowserEntryPointKind::SystemDriveRoot,
                None,
                "System Drive",
                protocol::LocalBrowserEntryPointStatus::Unavailable,
                Some(failure),
            ));
            None
        }
    };

    match resolve_windows_volume_roots() {
        Ok(volume_roots) => {
            let system_key = system_drive_root
                .as_deref()
                .map(crate::local_browser_entry_points::normalize_local_browser_path_key);
            for volume in volume_roots {
                if volume.drive_type == DRIVE_FIXED {
                    if system_key.as_deref()
                        == Some(normalize_local_browser_path_key(&volume.path).as_str())
                    {
                        continue;
                    }
                    entries.push(entry(
                        protocol::LocalBrowserEntryPointKind::LocalDataVolumeRoot,
                        Some(volume.path.clone()),
                        path_to_display_name(&volume.path),
                        status_for_path(&volume.path),
                        None,
                    ));
                } else if volume.drive_type == DRIVE_REMOVABLE {
                    entries.push(entry(
                        protocol::LocalBrowserEntryPointKind::RemovableVolumeRoot,
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
        protocol::LocalBrowserEntryPointKind::UserHome,
        &FOLDERID_Profile,
        "Home",
    ));
    entries.push(resolve_known_folder_entry(
        protocol::LocalBrowserEntryPointKind::Desktop,
        &FOLDERID_Desktop,
        "Desktop",
    ));
    entries.push(resolve_known_folder_entry(
        protocol::LocalBrowserEntryPointKind::Downloads,
        &FOLDERID_Downloads,
        "Downloads",
    ));
    entries.push(resolve_known_folder_entry(
        protocol::LocalBrowserEntryPointKind::Music,
        &FOLDERID_Music,
        "Music",
    ));

    return Ok(LocalBrowserEntryPointResolution {
        entries,
        failure: resolution_failure,
    });

    struct WindowsVolumeRoot {
        path: PathBuf,
        drive_type: u32,
    }

    fn resolve_known_folder_entry(
        entry_point_kind: protocol::LocalBrowserEntryPointKind,
        folder_id: &GUID,
        display_name: &'static str,
    ) -> ResolvedLocalBrowserEntryPoint {
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
                protocol::LocalBrowserEntryPointStatus::Unavailable,
                Some(failure),
            ),
        }
    }

    fn known_folder_path(
        folder_id: &GUID,
    ) -> Result<PathBuf, LocalBrowserEntryPointResolveFailure> {
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
                protocol::LocalBrowserEntryPointFailureCode::KnownFolderUnavailable,
                format!("SHGetKnownFolderPath failed with HRESULT {result:#x}"),
            ));
        }

        let path = unsafe { wide_ptr_to_path_buf(raw_path as PCWSTR) };
        unsafe {
            CoTaskMemFree(raw_path.cast());
        }
        path.ok_or_else(|| {
            entry_failure(
                protocol::LocalBrowserEntryPointFailureCode::KnownFolderUnavailable,
                "SHGetKnownFolderPath returned an empty path",
            )
        })
    }

    fn resolve_system_drive_root() -> Result<PathBuf, LocalBrowserEntryPointResolveFailure> {
        let windows_directory = windows_directory_path()?;
        volume_path_for(&windows_directory)
    }

    fn windows_directory_path() -> Result<PathBuf, LocalBrowserEntryPointResolveFailure> {
        let mut buffer = vec![0_u16; MAX_WINDOWS_PATH_BUFFER];
        let length = unsafe {
            GetSystemWindowsDirectoryW(
                buffer.as_mut_ptr(),
                u32::try_from(buffer.len()).expect("Windows path buffer fits u32"),
            )
        };
        if length == 0 || usize::try_from(length).map_or(true, |len| len >= buffer.len()) {
            return Err(entry_failure(
                protocol::LocalBrowserEntryPointFailureCode::SystemDriveUnavailable,
                "GetSystemWindowsDirectoryW failed to resolve the system Windows directory",
            ));
        }
        buffer.truncate(usize::try_from(length).expect("Windows path length fits usize"));
        Ok(PathBuf::from(OsString::from_wide(&buffer)))
    }

    fn volume_path_for(path: &Path) -> Result<PathBuf, LocalBrowserEntryPointResolveFailure> {
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
                protocol::LocalBrowserEntryPointFailureCode::SystemDriveUnavailable,
                "GetVolumePathNameW failed to resolve the system volume root",
            ));
        }
        wide_buffer_to_path_buf(&buffer).ok_or_else(|| {
            entry_failure(
                protocol::LocalBrowserEntryPointFailureCode::SystemDriveUnavailable,
                "GetVolumePathNameW returned an empty system volume root",
            )
        })
    }

    fn resolve_windows_volume_roots()
    -> Result<Vec<WindowsVolumeRoot>, LocalBrowserEntryPointResolveFailure> {
        let logical_drives = unsafe { GetLogicalDrives() };
        if logical_drives == 0 {
            return Err(entry_failure(
                protocol::LocalBrowserEntryPointFailureCode::VolumeEnumerationUnavailable,
                "GetLogicalDrives failed while resolving local browser entry points",
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
-> Result<LocalBrowserEntryPointResolution, LocalBrowserEntryPointResolveFailure> {
    let platform = local_browser_entry_point_platform();
    let entries = [
        protocol::LocalBrowserEntryPointKind::SystemDriveRoot,
        protocol::LocalBrowserEntryPointKind::LocalDataVolumeRoot,
        protocol::LocalBrowserEntryPointKind::RemovableVolumeRoot,
        protocol::LocalBrowserEntryPointKind::UserHome,
        protocol::LocalBrowserEntryPointKind::Desktop,
        protocol::LocalBrowserEntryPointKind::Downloads,
        protocol::LocalBrowserEntryPointKind::Music,
    ]
    .into_iter()
    .map(|kind| ResolvedLocalBrowserEntryPoint {
        entry_point_kind: kind,
        canonical_path: None,
        display_name: default_display_name(kind).to_string(),
        status: protocol::LocalBrowserEntryPointStatus::UnsupportedPlatform,
        platform,
        failure: Some(entry_failure(
            protocol::LocalBrowserEntryPointFailureCode::UnsupportedPlatform,
            "local browser entry point resolution is Windows-only in V0",
        )),
    })
    .collect();

    Ok(LocalBrowserEntryPointResolution {
        entries,
        failure: None,
    })
}

pub(crate) fn normalize_local_browser_path_key(path: &Path) -> String {
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
fn default_display_name(kind: protocol::LocalBrowserEntryPointKind) -> &'static str {
    match kind {
        protocol::LocalBrowserEntryPointKind::SystemDriveRoot => "System Drive",
        protocol::LocalBrowserEntryPointKind::LocalDataVolumeRoot => "Local Data Volume",
        protocol::LocalBrowserEntryPointKind::RemovableVolumeRoot => "Removable Volume",
        protocol::LocalBrowserEntryPointKind::UserHome => "Home",
        protocol::LocalBrowserEntryPointKind::Desktop => "Desktop",
        protocol::LocalBrowserEntryPointKind::Downloads => "Downloads",
        protocol::LocalBrowserEntryPointKind::Music => "Music",
    }
}
