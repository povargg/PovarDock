//! Windows' virtual desktops: which exist, which one shows, and switching.
//!
//! Explorer keeps the desktop list in the registry. Switching goes through
//! Windows' own shortcut, Ctrl+Win+Left or Right, because the COM interface
//! for it is undocumented and changes between Windows builds.

use std::time::{Duration, Instant};

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_BINARY, RRF_RT_REG_SZ, RegGetValueW};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, HOT_KEY_MODIFIERS, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, MOD_NOREPEAT, RegisterHotKey, SendInput, UnregisterHotKey, VIRTUAL_KEY, VK_CONTROL, VK_LCONTROL,
    VK_LEFT, VK_LWIN, VK_MENU, VK_RIGHT, VK_RWIN, VK_SHIFT,
};
use windows::core::{HSTRING, PCWSTR, w};

pub const HOTKEY_FIRST: usize = 101;
pub const HOTKEY_COUNT: usize = 9;

const KEY: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Explorer\VirtualDesktops");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Desktops {
    /// The desktop that shows, from 0.
    pub current: usize,
    /// Each desktop's name, or "Desktop n" if the user never named it.
    pub names: Vec<String>,
}

impl Desktops {
    pub fn label(&self) -> String {
        let number = format!("Desktop {}", self.current + 1);
        let name = &self.names[self.current];
        if name == &number {
            number
        } else {
            format!("{number} · {name}")
        }
    }
}

/// Reads the desktops. With one desktop, Explorer may not have written the list yet.
pub fn read() -> Desktops {
    let ids = reg_binary(KEY, w!("VirtualDesktopIDs"));
    let current = reg_binary(KEY, w!("CurrentVirtualDesktop"));
    let (current, count) = position(&ids, &current);
    let names = (0..count)
        .map(|i| {
            ids.as_chunks::<16>()
                .0
                .get(i)
                .and_then(|id| {
                    let sub = HSTRING::from(format!(
                        r"Software\Microsoft\Windows\CurrentVersion\Explorer\VirtualDesktops\Desktops\{}",
                        guid(id)
                    ));
                    reg_string(PCWSTR(sub.as_ptr()), w!("Name"))
                })
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| format!("Desktop {}", i + 1))
        })
        .collect();
    Desktops { current, names }
}

pub fn coordinates() -> (usize, usize) {
    position(
        &reg_binary(KEY, w!("VirtualDesktopIDs")),
        &reg_binary(KEY, w!("CurrentVirtualDesktop")),
    )
}

/// The current desktop's index and the number of desktops, from Explorer's
/// list of 16-byte desktop IDs.
fn position(ids: &[u8], current: &[u8]) -> (usize, usize) {
    let count = (ids.len() / 16).max(1);
    let index = ids
        .as_chunks::<16>()
        .0
        .iter()
        .position(|id| id[..] == *current)
        .unwrap_or(0);
    (index, count)
}

/// A desktop ID as its registry key name, such as `{2EDA9EEE-5A3A-4CC0-...}`.
fn guid(id: &[u8]) -> String {
    let u32le = u32::from_le_bytes([id[0], id[1], id[2], id[3]]);
    let u16le = |i: usize| u16::from_le_bytes([id[i], id[i + 1]]);
    let hex = |range: std::ops::Range<usize>| id[range].iter().map(|b| format!("{b:02X}")).collect::<String>();
    format!(
        "{{{u32le:08X}-{:04X}-{:04X}-{}-{}}}",
        u16le(4),
        u16le(6),
        hex(8..10),
        hex(10..16)
    )
}

/// Shows the next desktop to the right, or to the left.
pub fn switch(right: bool) -> bool {
    let arrow = if right { VK_RIGHT } else { VK_LEFT };
    let key = |vk: VIRTUAL_KEY, up: bool| {
        let mut flags = if up { KEYEVENTF_KEYUP } else { Default::default() };
        if vk != VK_LCONTROL {
            // The Windows and arrow keys are extended keys.
            flags |= KEYEVENTF_EXTENDEDKEY;
        }
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    dwFlags: flags,
                    ..Default::default()
                },
            },
        }
    };
    let inputs = [
        key(VK_LCONTROL, false),
        key(VK_LWIN, false),
        key(arrow, false),
        key(arrow, true),
        key(VK_LWIN, true),
        key(VK_LCONTROL, true),
    ];
    // Fails when an elevated window has the focus; Windows blocks the keys then.
    if unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) } != inputs.len() as u32 {
        eprintln!("Dock desktops: {}", windows::core::Error::from_thread());
        return false;
    }
    true
}

/// Each hop waits for Explorer to acknowledge the previous one in the registry.
pub struct Jump {
    target: usize,
    deadline: Instant,
    pending: Option<(usize, Instant)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    Wait,
    Move(bool),
    Arrived,
    Cancel,
}

impl Jump {
    pub fn new(target: usize, now: Instant) -> Self {
        Self {
            target,
            deadline: now + Duration::from_secs(6),
            pending: None,
        }
    }

    pub fn retarget(&mut self, target: usize, now: Instant) {
        self.target = target;
        self.deadline = now + Duration::from_secs(6);
    }

    pub fn advance(&mut self, (current, count): (usize, usize), now: Instant, keys_down: bool) -> Step {
        if self.target >= count || now >= self.deadline {
            return Step::Cancel;
        }
        if let Some((from, expires)) = self.pending {
            if from == current {
                return if now < expires { Step::Wait } else { Step::Cancel };
            }
            self.pending = None;
        }
        if current == self.target {
            return Step::Arrived;
        }
        // An Alt/Shift/Win shortcut can still be physically held when WM_HOTKEY arrives.
        if keys_down {
            return Step::Wait;
        }
        self.pending = Some((current, now + Duration::from_millis(1500)));
        Step::Move(current < self.target)
    }
}

pub fn modifiers_down() -> bool {
    [VK_MENU, VK_CONTROL, VK_SHIFT, VK_LWIN, VK_RWIN]
        .into_iter()
        .any(|key| unsafe { GetAsyncKeyState(i32::from(key.0)) < 0 })
}

/// Registers a full bank, or restores the previous one if a key is taken.
pub fn register_shortcuts(owner: HWND, registered: &mut u32, wanted: u32) -> Result<(), usize> {
    if *registered == wanted {
        return Ok(());
    }
    let previous = std::mem::take(registered);
    if previous != 0 {
        unregister_shortcuts(owner, HOTKEY_COUNT);
    }
    match register_bank(owner, wanted) {
        Ok(()) => {
            *registered = wanted;
            Ok(())
        }
        Err(key) => {
            if register_bank(owner, previous).is_ok() {
                *registered = previous;
            }
            Err(key)
        }
    }
}

fn register_bank(owner: HWND, modifiers: u32) -> Result<(), usize> {
    if modifiers == 0 {
        return Ok(());
    }
    let modifiers = HOT_KEY_MODIFIERS(modifiers) | MOD_NOREPEAT;
    for index in 0..HOTKEY_COUNT {
        let id = (HOTKEY_FIRST + index) as i32;
        let key = u32::from(b'1') + index as u32;
        if unsafe { RegisterHotKey(Some(owner), id, modifiers, key) }.is_err() {
            unregister_shortcuts(owner, index);
            return Err(index + 1);
        }
    }
    Ok(())
}

fn unregister_shortcuts(owner: HWND, count: usize) {
    for id in HOTKEY_FIRST..HOTKEY_FIRST + count {
        let _ = unsafe { UnregisterHotKey(Some(owner), id as i32) };
    }
}

fn reg_binary(sub: PCWSTR, value: PCWSTR) -> Vec<u8> {
    let mut size = 0u32;
    unsafe {
        if RegGetValueW(
            HKEY_CURRENT_USER,
            sub,
            value,
            RRF_RT_REG_BINARY,
            None,
            None,
            Some(&mut size),
        )
        .is_err()
        {
            return Vec::new();
        }
        let mut data = vec![0u8; size as usize];
        if RegGetValueW(
            HKEY_CURRENT_USER,
            sub,
            value,
            RRF_RT_REG_BINARY,
            None,
            Some(data.as_mut_ptr().cast()),
            Some(&mut size),
        )
        .is_err()
        {
            return Vec::new();
        }
        data.truncate(size as usize);
        data
    }
}

fn reg_string(sub: PCWSTR, value: PCWSTR) -> Option<String> {
    let mut buffer = [0u16; 256];
    let mut size = size_of_val(&buffer) as u32;
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            sub,
            value,
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut size),
        )
        .ok()
        .ok()?;
    }
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..len]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_current_desktop_in_the_list() {
        let a = [1u8; 16];
        let b = [2u8; 16];
        let c = [3u8; 16];
        let ids: Vec<u8> = [a, b, c].concat();
        assert_eq!(position(&ids, &b), (1, 3));
        assert_eq!(position(&ids, &c), (2, 3));
        // An unknown or missing current desktop falls back to the first.
        assert_eq!(position(&ids, &[9u8; 16]), (0, 3));
        // Before the user adds a desktop, Explorer may write no list.
        assert_eq!(position(&[], &[]), (0, 1));
    }

    #[test]
    fn formats_ids_as_registry_key_names() {
        let id = [
            0xEE, 0x9E, 0xDA, 0x2E, 0x3A, 0x5A, 0xC0, 0x4C, 0x8A, 0xBD, 0x84, 0x14, 0xB8, 0x76, 0x5F, 0x35,
        ];
        assert_eq!(guid(&id), "{2EDA9EEE-5A3A-4CC0-8ABD-8414B8765F35}");
    }

    #[test]
    fn jump_waits_for_released_modifiers_and_real_desktop_changes() {
        let now = Instant::now();
        let mut jump = Jump::new(2, now);
        assert_eq!(jump.advance((0, 3), now, true), Step::Wait);
        assert_eq!(jump.advance((0, 3), now, false), Step::Move(true));
        assert_eq!(jump.advance((0, 3), now, false), Step::Wait);
        assert_eq!(jump.advance((1, 3), now, false), Step::Move(true));
        assert_eq!(jump.advance((2, 3), now, false), Step::Arrived);
    }

    #[test]
    fn jump_handles_retargeting_missing_desktops_and_refused_switches() {
        let now = Instant::now();
        let mut jump = Jump::new(2, now);
        assert_eq!(jump.advance((0, 1), now, false), Step::Cancel);
        assert_eq!(jump.advance((0, 3), now, false), Step::Move(true));
        jump.retarget(0, now);
        assert_eq!(jump.advance((0, 3), now, false), Step::Wait);
        assert_eq!(jump.advance((1, 3), now, false), Step::Move(false));
        assert_eq!(jump.advance((1, 3), now + Duration::from_secs(2), false), Step::Cancel);
    }

    #[test]
    fn reads_this_machine_without_failing() {
        let desktops = read();
        assert!(desktops.current < desktops.names.len());
        assert!(desktops.names.iter().all(|name| !name.is_empty()));
    }
}
