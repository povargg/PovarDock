//! Checks WinDock's GitHub releases and updates the portable executable in place.
//!
//! The launcher and Settings share an executable. The updater verifies the download
//! before renaming the current executable to .old and moving the new file into place.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use sha2::{Digest, Sha256};
use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE, LPARAM, WAIT_ABANDONED, WAIT_OBJECT_0, WPARAM},
        Networking::WinHttp::*,
        System::Threading::*,
        UI::WindowsAndMessaging::*,
    },
    core::{PCWSTR, PWSTR, w},
};

const LATEST_URL: &str = "https://api.github.com/repos/povargg/WinDock/releases/latest";
const ASSET_NAME: &str = "windock-x64.exe";
const CHECKSUMS_NAME: &str = "SHA256SUMS.txt";
/// The launcher quits when it receives this message (WM_APP_QUIT in the launcher).
const LAUNCHER_QUIT: u32 = WM_APP + 4;

#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    /// The tag, such as `v0.2.3`.
    pub tag: String,
    /// The release page.
    pub page: String,
    assets: Vec<(String, String)>,
}

impl Release {
    /// The version without the leading `v`.
    pub fn version(&self) -> &str {
        self.tag.trim_start_matches('v')
    }

    fn asset_url(&self, name: &str) -> Result<&str, String> {
        self.assets
            .iter()
            .find(|(asset, _)| asset == name)
            .map(|(_, url)| url.as_str())
            .ok_or_else(|| format!("Release {} has no file named {name}.", self.tag))
    }
}

/// Returns the latest release if it is newer than this app.
pub fn check() -> Result<Option<Release>, String> {
    // Nightly builds are installed by hand; stable releases would replace them.
    if crate::channel::NIGHTLY {
        return Ok(None);
    }
    let body = get(LATEST_URL).map_err(|error| format!("Cannot check for updates: {error}"))?;
    let release = parse_release(&body)?;
    if !is_newer(release.version(), env!("CARGO_PKG_VERSION")) {
        return Ok(None);
    }
    release.asset_url(ASSET_NAME)?;
    release.asset_url(CHECKSUMS_NAME)?;
    Ok(Some(release))
}

/// Runs before GPUI starts. Errors leave the previous notice alone.
pub fn check_hidden() {
    if crate::channel::NIGHTLY {
        return;
    }
    let Ok(config_path) = crate::config::path() else { return };
    let Ok(config) = crate::config::Store::new(config_path.clone()).load() else {
        return;
    };
    if !config.check_for_updates || !unmetered() {
        return;
    }
    let Ok(release) = check() else { return };
    let path = config_path.with_file_name(crate::update_notice::FILE_NAME);
    let mut state = crate::update_notice::State::load(&path);
    state.version = release.map(|r| r.version().to_owned()).unwrap_or_default();
    debug_assert!(state.version.is_empty() || state.available());
    if state.save(&path).is_err() {
        return;
    }
    unsafe {
        if let Ok(hwnd) = FindWindowW(crate::channel::LAUNCHER_CLASS, None) {
            let _ = PostMessageW(Some(hwnd), crate::update_notice::MESSAGE, WPARAM(0), LPARAM(0));
        }
    }
}

fn unmetered() -> bool {
    use windows::Win32::{Networking::NetworkListManager::*, System::Com::*};
    unsafe {
        if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
            return false;
        }
        let result = (|| {
            let manager: INetworkCostManager = CoCreateInstance(&NetworkListManager, None, CLSCTX_ALL).ok()?;
            let mut cost = 0;
            manager.GetCost(&mut cost, std::ptr::null()).ok()?;
            Some(cost == NLM_CONNECTION_COST_UNRESTRICTED.0 as u32)
        })()
        .unwrap_or(false);
        CoUninitialize();
        result
    }
}

/// Installs the verified release. Restarts the launcher if it was running.
pub fn install(release: &Release) -> Result<PathBuf, String> {
    if let Some(manager) = crate::install::current_manager() {
        return Err(manager.instruction().into());
    }
    let executable = current_executable()?;
    let _lock = UpdateLock::acquire().ok_or("Another update is running. Try again when it is done.")?;
    let staged = stage_release(release, &executable, get)?;

    let remove_staged = || {
        let _ = std::fs::remove_file(&staged);
    };
    let launcher_was_running = stop_launcher(&executable).inspect_err(|_| remove_staged())?;
    if let Err(error) = swap(&executable, &staged) {
        remove_staged();
        if launcher_was_running && executable.exists() {
            let _ = std::process::Command::new(&executable).spawn();
        }
        return Err(error);
    }
    if launcher_was_running {
        std::process::Command::new(&executable).spawn().map_err(|error| {
            format!(
                "WinDock was updated. Start it again from {}: {error}",
                executable.display()
            )
        })?;
    }
    Ok(executable)
}

/// Stages the verified download alongside the executable so renames stay on one volume.
fn stage_release(
    release: &Release,
    target: &Path,
    mut download: impl FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<PathBuf, String> {
    let sums = download(release.asset_url(CHECKSUMS_NAME)?)?;
    let sums = std::str::from_utf8(&sums).map_err(|error| format!("Cannot read {CHECKSUMS_NAME}: {error}"))?;
    let bytes = download(release.asset_url(ASSET_NAME)?)?;
    verify(ASSET_NAME, &bytes, &parse_sums(sums))?;

    let staged = with_suffix(target, ".new");
    if let Err(error) = std::fs::write(&staged, bytes) {
        let _ = std::fs::remove_file(&staged);
        return Err(format!(
            "Cannot save the update: {error}. Download it from the release page instead."
        ));
    }
    Ok(staged)
}

fn verify(asset: &str, bytes: &[u8], sums: &[(&str, &str)]) -> Result<(), String> {
    let expected = sums
        .iter()
        .find(|(name, _)| *name == asset)
        .map(|(_, hash)| *hash)
        .ok_or_else(|| format!("SHA256SUMS.txt has no checksum for {asset}."))?;
    if !sha256_hex(bytes).eq_ignore_ascii_case(expected) {
        return Err(format!("{asset} does not match its checksum. Nothing was changed."));
    }
    Ok(())
}

/// Starts the new executable on the About page. The caller then closes the old window.
pub fn restart_settings(executable: &Path) -> Result<(), String> {
    std::process::Command::new(executable)
        .args(["--settings", "--page", "about"])
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Cannot restart Settings: {error}"))
}

/// Removes the previous executable once its Settings process releases it.
pub fn cleanup_previous_update() {
    if crate::install::current_manager().is_some() {
        return;
    }
    let Ok(executable) = current_executable() else {
        return;
    };
    let previous = with_suffix(&executable, ".old");
    if !previous.exists() {
        return;
    }
    std::thread::spawn(move || {
        for _ in 0..20 {
            if let Some(_lock) = UpdateLock::acquire()
                && (!previous.exists() || std::fs::remove_file(&previous).is_ok())
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    });
}

/// A named mutex held while an update replaces files. Each thread that takes
/// it releases it on the same thread, as Windows requires.
struct UpdateLock(HANDLE);

impl UpdateLock {
    fn acquire() -> Option<Self> {
        unsafe {
            let mutex = CreateMutexW(None, false, w!("Local\\WinDock.Settings.Update")).ok()?;
            let wait = WaitForSingleObject(mutex, 0);
            if wait == WAIT_OBJECT_0 || wait == WAIT_ABANDONED {
                Some(Self(mutex))
            } else {
                let _ = CloseHandle(mutex);
                None
            }
        }
    }
}

impl Drop for UpdateLock {
    fn drop(&mut self) {
        unsafe {
            let _ = ReleaseMutex(self.0);
            let _ = CloseHandle(self.0);
        }
    }
}

fn current_executable() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|error| format!("Cannot locate the running WinDock executable: {error}"))
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

fn swap(target: &Path, new: &Path) -> Result<(), String> {
    let old = with_suffix(target, ".old");
    let _ = std::fs::remove_file(&old);
    let fail = |error: std::io::Error| format!("Cannot replace {}: {error}", target.display());
    std::fs::rename(target, &old).map_err(fail)?;
    std::fs::rename(new, target).map_err(|error| {
        let reason = fail(error);
        match std::fs::rename(&old, target) {
            Ok(()) => reason,
            Err(restore) => format!(
                "{reason}. Restore the previous executable from {}: {restore}",
                old.display()
            ),
        }
    })
}

/// Stops the launcher if it is running from the executable being updated.
fn stop_launcher(executable: &Path) -> Result<bool, String> {
    unsafe {
        let Ok(hwnd) = FindWindowW(crate::channel::LAUNCHER_CLASS, None) else {
            return Ok(false);
        };
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let process = OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
            .map_err(|error| format!("Cannot find the running WinDock: {error}"))?;
        let result = if !same_file(&process_path(process), executable) {
            Ok(false)
        } else if PostMessageW(Some(hwnd), LAUNCHER_QUIT, WPARAM(0), LPARAM(0)).is_ok()
            && WaitForSingleObject(process, 5000) == WAIT_OBJECT_0
        {
            Ok(true)
        } else {
            Err("WinDock did not close. Quit it from its notification area icon, then try again.".into())
        };
        let _ = CloseHandle(process);
        result
    }
}

fn process_path(process: HANDLE) -> PathBuf {
    let mut buffer = vec![0u16; 32_768];
    let mut len = buffer.len() as u32;
    unsafe {
        if QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut len).is_err() {
            return PathBuf::new();
        }
    }
    PathBuf::from(String::from_utf16_lossy(&buffer[..len as usize]))
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a.as_os_str().eq_ignore_ascii_case(b.as_os_str()),
        _ => false,
    }
}

fn parse_release(body: &[u8]) -> Result<Release, String> {
    let json: serde_json::Value =
        serde_json::from_slice(body).map_err(|error| format!("Cannot read the release from GitHub: {error}"))?;
    let text = |value: &serde_json::Value, key: &str| value.get(key).and_then(|v| v.as_str()).map(str::to_owned);
    let tag = text(&json, "tag_name").ok_or("The release from GitHub has no version.")?;
    let page = text(&json, "html_url").unwrap_or_else(|| format!("{}/releases", crate::REPOSITORY_URL));
    let assets = json
        .get("assets")
        .and_then(|assets| assets.as_array())
        .into_iter()
        .flatten()
        .filter_map(|asset| Some((text(asset, "name")?, text(asset, "browser_download_url")?)))
        .collect();
    Ok(Release { tag, page, assets })
}

/// Reads `HASH  name` lines, the format the release workflow writes.
fn parse_sums(text: &str) -> Vec<(&str, &str)> {
    text.lines()
        .filter_map(|line| {
            let (hash, name) = line.trim().split_once(char::is_whitespace)?;
            Some((name.trim().trim_start_matches('*'), hash))
        })
        .collect()
}

fn version_parts(version: &str) -> Option<Vec<u64>> {
    let version = version.trim().trim_start_matches('v');
    // A pre-release suffix such as `-beta` is ignored.
    let version = version.split(['-', '+']).next()?;
    version.split('.').map(|part| part.parse().ok()).collect()
}

fn is_newer(latest: &str, current: &str) -> bool {
    match (version_parts(latest), version_parts(current)) {
        (Some(latest), Some(current)) => latest > current,
        _ => false,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Requests an https URL and returns the body. WinHTTP follows redirects and
/// uses the system proxy settings.
fn get(url: &str) -> Result<Vec<u8>, String> {
    let rest = url
        .strip_prefix("https://")
        .ok_or("Only https downloads are allowed.")?;
    let (host, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    let path = if path.is_empty() { "/" } else { path };
    let (host, path) = (wide(host), wide(path));
    let fail = |step: &str| format!("{step} failed: {}", windows::core::Error::from_thread().message());

    struct Handle(*mut core::ffi::c_void);
    impl Drop for Handle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    let _ = WinHttpCloseHandle(self.0);
                }
            }
        }
    }

    unsafe {
        let agent = wide(format!("WinDock/{}", env!("CARGO_PKG_VERSION")));
        let session = Handle(WinHttpOpen(
            PCWSTR(agent.as_ptr()),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            PCWSTR::null(),
            PCWSTR::null(),
            0,
        ));
        if session.0.is_null() {
            return Err(fail("Opening a connection"));
        }
        let connection = Handle(WinHttpConnect(
            session.0,
            PCWSTR(host.as_ptr()),
            INTERNET_DEFAULT_HTTPS_PORT,
            0,
        ));
        if connection.0.is_null() {
            return Err(fail("Connecting"));
        }
        let request = Handle(WinHttpOpenRequest(
            connection.0,
            w!("GET"),
            PCWSTR(path.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            std::ptr::null(),
            WINHTTP_FLAG_SECURE,
        ));
        if request.0.is_null() {
            return Err(fail("Creating the request"));
        }
        let headers: Vec<u16> = "Accept: application/vnd.github+json, application/octet-stream\r\n"
            .encode_utf16()
            .collect();
        WinHttpSendRequest(request.0, Some(&headers), None, 0, 0, 0)
            .and_then(|()| WinHttpReceiveResponse(request.0, std::ptr::null_mut()))
            .map_err(|error| format!("The download failed: {}", error.message()))?;

        let mut status = 0u32;
        let mut size = size_of::<u32>() as u32;
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            PCWSTR::null(),
            Some((&raw mut status).cast()),
            &mut size,
            std::ptr::null_mut(),
        )
        .map_err(|error| error.message())?;
        if status != 200 {
            return Err(format!("GitHub returned HTTP {status}."));
        }

        let mut body = Vec::new();
        let mut buffer = vec![0u8; 64 * 1024];
        loop {
            let mut read = 0u32;
            WinHttpReadData(request.0, buffer.as_mut_ptr().cast(), buffer.len() as u32, &mut read)
                .map_err(|error| format!("The download failed: {}", error.message()))?;
            if read == 0 {
                break;
            }
            body.extend_from_slice(&buffer[..read as usize]);
        }
        Ok(body)
    }
}

fn wide(value: impl AsRef<str>) -> Vec<u16> {
    value.as_ref().encode_utf16().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions_by_number() {
        assert!(is_newer("v0.2.3", "0.2.2"));
        assert!(is_newer("v0.10.0", "0.9.9"));
        assert!(is_newer("1.0.0", "0.99.0"));
        assert!(!is_newer("v0.2.2", "0.2.2"));
        assert!(!is_newer("v0.2.1", "0.2.2"));
        assert!(!is_newer("v0.2.3-beta", "0.2.3"));
        assert!(!is_newer("nightly", "0.2.2"));
    }

    #[test]
    fn reads_the_release_json() {
        let body = br#"{
            "tag_name": "v0.2.3",
            "html_url": "https://github.com/povargg/WinDock/releases/tag/v0.2.3",
            "assets": [
                {"name": "SHA256SUMS.txt", "browser_download_url": "https://example.com/sums"},
                {"name": "broken"}
            ]
        }"#;
        let release = parse_release(body).unwrap();
        assert_eq!(release.version(), "0.2.3");
        assert_eq!(release.asset_url("SHA256SUMS.txt").unwrap(), "https://example.com/sums");
        assert!(release.asset_url("broken").is_err());
        assert!(parse_release(b"{}").is_err());
    }

    #[test]
    fn reads_checksum_lines() {
        let sums = parse_sums("ABC123  windock-x64.exe\r\ndef456 *notes.txt\r\n");
        assert_eq!(sums, [("windock-x64.exe", "ABC123"), ("notes.txt", "def456")]);
    }

    #[test]
    fn hashes_with_sha256() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    fn portable_release() -> Release {
        Release {
            tag: "v99.0.0".into(),
            page: crate::REPOSITORY_URL.into(),
            assets: vec![
                (ASSET_NAME.into(), "exe".into()),
                (CHECKSUMS_NAME.into(), "sums".into()),
            ],
        }
    }

    #[test]
    fn installs_verified_update_at_the_current_filename() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("My WinDock.exe");
        std::fs::write(&target, b"old executable").unwrap();
        let binary = b"new executable";
        let sums = format!("{}  {ASSET_NAME}\n", sha256_hex(binary));
        let staged = stage_release(&portable_release(), &target, |url| match url {
            "sums" => Ok(sums.as_bytes().to_vec()),
            "exe" => Ok(binary.to_vec()),
            _ => panic!("unexpected asset {url}"),
        })
        .unwrap();

        assert_eq!(std::fs::read(&target).unwrap(), b"old executable");
        swap(&target, &staged).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), binary);
        assert_eq!(std::fs::read(with_suffix(&target, ".old")).unwrap(), b"old executable");
        assert!(!staged.exists());
    }

    #[test]
    fn rejects_missing_or_wrong_checksums_before_writing() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join(ASSET_NAME);
        std::fs::write(&target, b"old executable").unwrap();

        for sums in ["", "bad-hash  windock-x64.exe\n"] {
            let result = stage_release(&portable_release(), &target, |url| match url {
                "sums" => Ok(sums.as_bytes().to_vec()),
                "exe" => Ok(b"new executable".to_vec()),
                _ => panic!("unexpected asset {url}"),
            });
            assert!(result.is_err());
            assert_eq!(std::fs::read(&target).unwrap(), b"old executable");
            assert!(!with_suffix(&target, ".new").exists());
            assert!(!with_suffix(&target, ".old").exists());
        }
    }

    #[test]
    fn restores_previous_executable_if_the_swap_fails() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join(ASSET_NAME);
        std::fs::write(&target, b"old executable").unwrap();
        assert!(swap(&target, &with_suffix(&target, ".new")).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"old executable");
        assert!(!with_suffix(&target, ".old").exists());
    }
}
