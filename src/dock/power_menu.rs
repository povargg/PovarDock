//! The dock's power menu, tracked outside the dock's state borrow.

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{HSTRING, w};

use crate::commands::power::{self, Power};

pub fn track(dock: HWND, owner: HWND, anchor: RECT) {
    let chosen = unsafe {
        let Ok(menu) = CreatePopupMenu() else {
            return;
        };
        for (index, power) in Power::DOCK_MENU.into_iter().enumerate() {
            let _ = AppendMenuW(menu, MF_STRING, index + 1, &HSTRING::from(power.title()));
        }
        let _ = SetForegroundWindow(dock);
        let chosen = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTALIGN | TPM_BOTTOMALIGN,
            anchor.right,
            anchor.top - 4,
            None,
            dock,
            None,
        );
        let _ = DestroyMenu(menu);
        chosen.0 as usize
    };
    let Some(action) = chosen
        .checked_sub(1)
        .and_then(|index| Power::DOCK_MENU.get(index))
        .copied()
    else {
        return;
    };
    let title = HSTRING::from(format!("{} now?", action.title()));
    if action.needs_confirmation()
        && unsafe {
            MessageBoxW(
                Some(dock),
                w!("Save any open work before continuing."),
                &title,
                MB_OKCANCEL | MB_ICONWARNING | MB_DEFBUTTON2,
            )
        } != IDOK
    {
        return;
    }
    if let Err(error) = power::run(action) {
        let title = format!("Could not {}", action.title().to_lowercase());
        crate::tray::notify(owner, &title, &error, true);
    }
}
