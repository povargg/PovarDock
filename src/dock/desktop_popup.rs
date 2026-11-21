//! While the dock supplies desktop feedback, clip the small Explorer window
//! that would otherwise duplicate it. Keep Explorer's actual window alive.

use std::cell::RefCell;

use windows::Win32::{
    Foundation::{HWND, RECT},
    Graphics::Gdi::*,
    UI::{Accessibility::*, HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
};
use windows::core::{PCWSTR, w};

const CLASS: PCWSTR = w!("XamlExplorerHostIslandWindow");
const STYLE: WINDOW_EX_STYLE =
    WINDOW_EX_STYLE(WS_EX_NOACTIVATE.0 | WS_EX_NOREDIRECTIONBITMAP.0 | WS_EX_TOOLWINDOW.0 | WS_EX_TOPMOST.0);

thread_local! {
    static WATCHER: RefCell<Option<Watcher>> = const { RefCell::new(None) };
}

struct Watcher {
    explorer: u32,
    hook: HWINEVENTHOOK,
    clipped: Vec<HWND>,
}

/// Run on the launcher thread with a message loop. An Explorer restart
/// supplies a new process and a new set of shell-owned windows.
pub fn watch(enabled: bool) {
    let explorer = if enabled {
        process(unsafe { GetShellWindow() })
    } else {
        0
    };
    WATCHER.with(|slot| {
        let mut value = slot.borrow_mut();
        if explorer == 0 {
            value.take();
        } else {
            if value.as_ref().is_none_or(|old| old.explorer != explorer) {
                let hook = unsafe {
                    SetWinEventHook(
                        EVENT_OBJECT_SHOW,
                        EVENT_OBJECT_SHOW,
                        None,
                        Some(on_show),
                        explorer,
                        0,
                        WINEVENT_OUTOFCONTEXT,
                    )
                };
                *value = Some(Watcher {
                    explorer,
                    hook,
                    clipped: Vec::new(),
                });
            }
            if let Some(watcher) = &mut *value {
                watcher.scan();
            }
        }
    });
}

/// Explorer may already have created its popup on a previous switch.
pub fn clip_existing() {
    WATCHER.with(|slot| {
        if let Ok(mut value) = slot.try_borrow_mut()
            && let Some(watcher) = &mut *value
        {
            watcher.scan();
        }
    });
}

unsafe extern "system" fn on_show(
    _hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    object: i32,
    child: i32,
    _thread: u32,
    _time: u32,
) {
    if object != OBJID_WINDOW.0 || child != CHILDID_SELF as i32 {
        return;
    }
    WATCHER.with(|slot| {
        if let Ok(mut value) = slot.try_borrow_mut()
            && let Some(watcher) = &mut *value
        {
            watcher.clip(hwnd);
        }
    });
}

impl Watcher {
    fn scan(&mut self) {
        self.clipped.retain(|&hwnd| process(hwnd) == self.explorer);
        let mut after = None;
        while let Ok(hwnd) = unsafe { FindWindowExW(None, after, CLASS, PCWSTR::null()) } {
            self.clip(hwnd);
            after = Some(hwnd);
        }
    }

    fn clip(&mut self, hwnd: HWND) {
        if !is_desktop_popup(hwnd, self.explorer) {
            return;
        }
        unsafe {
            let mut bounds = RECT::default();
            // Leave any region the shell already owns alone.
            if GetWindowRgnBox(hwnd, &mut bounds) != RGN_ERROR {
                return;
            }
            let region = CreateRectRgn(0, 0, 0, 0);
            if region.is_invalid() {
                return;
            }
            if SetWindowRgn(hwnd, Some(region), true) == 0 {
                let _ = DeleteObject(region.into());
            } else if !self.clipped.contains(&hwnd) {
                self.clipped.push(hwnd);
            }
        }
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        unsafe {
            if !self.hook.is_invalid() {
                let _ = UnhookWinEvent(self.hook);
            }
            for &hwnd in &self.clipped {
                if process(hwnd) == self.explorer && same_class(hwnd) {
                    let mut bounds = RECT::default();
                    if GetWindowRgnBox(hwnd, &mut bounds) == NULLREGION {
                        let _ = SetWindowRgn(hwnd, None, true);
                    }
                }
            }
        }
    }
}

fn process(hwnd: HWND) -> u32 {
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    pid
}

fn same_class(hwnd: HWND) -> bool {
    let mut name = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut name) } as usize;
    &name[..len] == unsafe { CLASS.as_wide() }
}

fn is_desktop_popup(hwnd: HWND, explorer: u32) -> bool {
    if process(hwnd) != explorer || !same_class(hwnd) {
        return false;
    }
    unsafe {
        // The desktop notification island is ownerless and anchored
        // to the center of the bottom monitor edge.
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        if WINDOW_EX_STYLE(GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32) != STYLE
            || style & (WS_POPUP | WS_CLIPSIBLINGS).0 != (WS_POPUP | WS_CLIPSIBLINGS).0
            || GetWindowTextLengthW(hwnd) != 0
            || GetWindow(hwnd, GW_OWNER).is_ok()
            || GetPropW(hwnd, w!("NonRudeHWND")).is_invalid()
            || IsHungAppWindow(hwnd).as_bool()
        {
            return false;
        }
        let mut window = RECT::default();
        let mut monitor = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        GetWindowRect(hwnd, &mut window).is_ok()
            && GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut monitor).as_bool()
            && popup_bounds(window, monitor.rcMonitor, GetDpiForWindow(hwnd))
    }
}

fn popup_bounds(window: RECT, monitor: RECT, dpi: u32) -> bool {
    let width = window.right - window.left;
    let height = window.bottom - window.top;
    let scale = dpi.max(96) as f32 / 96.0;
    let dip_height = height as f32 / scale;
    let centered = (window.left + window.right - monitor.left - monitor.right).abs();
    window.bottom == monitor.bottom
        && centered <= 2
        && (52.0..=76.0).contains(&dip_height)
        && width > height
        && width < monitor.right - monitor.left
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_accepts_the_small_centered_bottom_island_at_its_monitor_dpi() {
        let screen = RECT {
            left: -1920,
            top: 0,
            right: 0,
            bottom: 1080,
        };
        let toast = RECT {
            left: -1066,
            top: 1004,
            right: -854,
            bottom: 1080,
        };
        assert!(popup_bounds(toast, screen, 120));
        assert!(!popup_bounds(toast, screen, 168));
        assert!(!popup_bounds(RECT { top: 650, ..toast }, screen, 120));
        assert!(!popup_bounds(
            RECT {
                left: -400,
                right: -30,
                ..toast
            },
            screen,
            120
        ));
        assert!(!popup_bounds(
            RECT {
                top: 600,
                bottom: 676,
                ..toast
            },
            screen,
            120
        ));
    }
}
