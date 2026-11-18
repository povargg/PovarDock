//! Opens apps and URLs and runs commands through the Windows shell.

use windows::Win32::Foundation::{E_FAIL, ERROR_CANCELLED};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::SystemInformation::GetWindowsDirectoryW;
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, IsIconic, SW_RESTORE, SW_SHOWNORMAL, SetForegroundWindow, ShowWindow,
};
use windows::core::{HSTRING, PCWSTR, Result, w};

/// Uses `AppsFolder` to open both desktop and Microsoft Store apps.
pub fn app(id: &str, path: Option<&str>, admin: bool) -> Result<()> {
    if admin && let Some(path) = path {
        return execute(path, None, true);
    }
    unsafe {
        let name = HSTRING::from(format!("shell:AppsFolder\\{id}"));
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        SHParseDisplayName(&name, None, &mut pidl, 0, None)?;
        let mut info = SHELLEXECUTEINFOW {
            cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_INVOKEIDLIST | SEE_MASK_FLAG_NO_UI,
            lpIDList: pidl.cast(),
            lpVerb: if admin { w!("runas") } else { PCWSTR::null() },
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };
        let result = ShellExecuteExW(&mut info);
        CoTaskMemFree(Some(pidl.cast()));
        result
    }
}

pub fn open(target: &str) -> Result<()> {
    execute(target, None, false)
}

pub fn shell(command: &str, admin: bool) -> Result<()> {
    execute(&system_tool(r"System32\cmd.exe"), Some(&format!("/k {command}")), admin)
}

pub fn terminal(command: &str) -> Result<()> {
    // /S removes the enclosing quotes, leaving the user's quoted arguments intact.
    execute(
        &system_tool(r"System32\cmd.exe"),
        Some(&format!("/d /s /k \"{command}\"")),
        false,
    )
}

pub fn notepad(path: &str) -> Result<()> {
    execute(
        &system_tool(r"System32\notepad.exe"),
        Some(&format!("\"{path}\"")),
        false,
    )
}

/// Shows the file's Properties, as the taskbar's menu does.
pub fn properties(path: &str) -> Result<()> {
    let file = HSTRING::from(path);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_INVOKEIDLIST | SEE_MASK_FLAG_NO_UI,
        lpVerb: w!("properties"),
        lpFile: PCWSTR(file.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    unsafe { ShellExecuteExW(&mut info) }
}

pub fn reveal(path: &str) -> Result<()> {
    execute(
        &system_tool("explorer.exe"),
        Some(&format!("/select,\"{path}\"")),
        false,
    )
}

/// Open Settings in a separate process from the same executable.
pub fn settings(welcome: bool) -> Result<()> {
    let settings_title = HSTRING::from(crate::channel::SETTINGS_NAME);
    let welcome_title = HSTRING::from(crate::channel::WELCOME_NAME);
    for title in [PCWSTR(settings_title.as_ptr()), PCWSTR(welcome_title.as_ptr())] {
        if let Ok(window) = unsafe { FindWindowW(None, title) } {
            unsafe {
                if IsIconic(window).as_bool() {
                    let _ = ShowWindow(window, SW_RESTORE);
                }
                let _ = SetForegroundWindow(window);
            }
            return Ok(());
        }
    }
    open_settings(if welcome { "--settings --welcome" } else { "--settings" })
}

pub fn update_settings() -> Result<()> {
    open_settings("--settings --page about")
}

fn open_settings(args: &str) -> Result<()> {
    let exe = std::env::current_exe().map_err(|e| windows::core::Error::new(E_FAIL, e.to_string()))?;
    execute(&exe.to_string_lossy(), Some(args), false)
}

/// Uses an absolute path to avoid running a same-named executable in the
/// current directory.
fn system_tool(relative: &str) -> String {
    let mut buf = [0u16; 260];
    let len = unsafe { GetWindowsDirectoryW(Some(&mut buf)) } as usize;
    let windows = if len > 0 && len < buf.len() {
        String::from_utf16_lossy(&buf[..len])
    } else {
        r"C:\Windows".into()
    };
    format!(r"{windows}\{relative}")
}

fn execute(file: &str, params: Option<&str>, admin: bool) -> Result<()> {
    let file = HSTRING::from(file);
    let params = params.map(HSTRING::from);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_FLAG_NO_UI,
        lpVerb: if admin { w!("runas") } else { w!("open") },
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: params.as_ref().map_or(PCWSTR::null(), |p| PCWSTR(p.as_ptr())),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    unsafe { ShellExecuteExW(&mut info) }
}

/// Identifies user cancellation, including a dismissed UAC prompt.
pub fn is_cancelled(error: &windows::core::Error) -> bool {
    error.code() == ERROR_CANCELLED.to_hresult()
}
