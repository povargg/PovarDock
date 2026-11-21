//! Opt-in Windows-key taps and desktop shortcuts. Hooks run on their own thread.
//!
//! Elevated foreground apps can block the hook or SendInput through UIPI;
//! the shell then handles the original keys. Only the selected digits are consumed.

use std::cell::{Cell, RefCell};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::Result;

use crate::dock::desktops::{HOTKEY_COUNT, HOTKEY_FIRST};

const LAUNCHER_TAP: u32 = 0x10;
const MODIFIERS: u32 = MOD_ALT.0 | MOD_CONTROL.0 | MOD_SHIFT.0 | MOD_WIN.0;

/// All physical keys participate, including modifiers held before Win-down.
/// Two overlapping Win keys cancel the tap until both have been released.
#[derive(Clone, Copy, Default)]
struct Tap {
    held: [u64; 4],
    candidate: Option<u32>,
}

impl Tap {
    fn key(&mut self, vk: u32, down: bool, injected: bool) -> bool {
        if injected || vk >= 256 {
            return false;
        }
        let slot = vk as usize / 64;
        let bit = 1 << (vk % 64);
        let was_down = self.held[slot] & bit != 0;
        let win = vk == VK_LWIN.0 as u32 || vk == VK_RWIN.0 as u32;
        if down {
            if !was_down {
                self.candidate = if win && self.held == [0; 4] { Some(vk) } else { None };
                self.held[slot] |= bit;
            }
            false
        } else {
            self.held[slot] &= !bit;
            let tap = was_down && self.candidate == Some(vk);
            if win {
                self.candidate = None;
            }
            tap
        }
    }

    fn click(&mut self) {
        self.candidate = None;
    }

    fn pressed(&self, vk: u32) -> bool {
        self.held[vk as usize / 64] & (1 << (vk % 64)) != 0
    }

    fn modifiers(&self) -> u32 {
        [
            (VK_LMENU, VK_RMENU, MOD_ALT),
            (VK_LCONTROL, VK_RCONTROL, MOD_CONTROL),
            (VK_LSHIFT, VK_RSHIFT, MOD_SHIFT),
            (VK_LWIN, VK_RWIN, MOD_WIN),
        ]
        .into_iter()
        .filter(|&(left, right, _)| self.pressed(u32::from(left.0)) || self.pressed(u32::from(right.0)))
        .fold(0, |mask, (_, _, modifier)| mask | modifier.0)
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Action {
    Pass,
    Consume,
    Launcher,
    Desktop(usize),
}

#[derive(Clone, Copy, Default)]
struct Keyboard {
    tap: Tap,
    /// Number keys already consumed. Their repeats and releases stay consumed
    /// even when a modifier is released or Settings changes in the meantime.
    consumed_digits: u16,
}

impl Keyboard {
    fn key(&mut self, vk: u32, down: bool, injected: bool, shortcuts: u32) -> Action {
        if injected || vk >= 256 {
            return Action::Pass;
        }
        let was_down = self.tap.pressed(vk);
        let tapped = self.tap.key(vk, down, false);
        if let Some(desktop) = vk
            .checked_sub(u32::from(b'1'))
            .filter(|index| *index < HOTKEY_COUNT as u32)
        {
            let bit = 1 << desktop;
            if self.consumed_digits & bit != 0 {
                if !down {
                    self.consumed_digits &= !bit;
                }
                return Action::Consume;
            }
            let wanted = shortcuts & MODIFIERS;
            if down && !was_down && wanted & MOD_WIN.0 != 0 && self.tap.modifiers() == wanted {
                return Action::Desktop(desktop as usize);
            }
        }
        if tapped && shortcuts & LAUNCHER_TAP != 0 {
            Action::Launcher
        } else {
            Action::Pass
        }
    }

    fn consume_digit(&mut self, desktop: usize) {
        self.consumed_digits |= 1 << desktop;
    }
}

struct Target {
    hwnd: HWND,
    shortcuts: Arc<AtomicU32>,
}

thread_local! {
    static KEYBOARD: Cell<Keyboard> = Cell::new(Keyboard::default());
    static TARGET: RefCell<Option<Target>> = const { RefCell::new(None) };
}

/// Owns the hook thread; dropping it on reload removes all input monitoring.
///
/// The hooks get their own thread and message loop. Every key press and mouse
/// event system-wide waits for this thread, so it must never be busy drawing
/// or loading like the launcher's thread, or the pointer would stutter and
/// Windows would drop the hook after it exceeds `LowLevelHooksTimeout`.
pub struct Hook {
    thread: u32,
    handle: Option<std::thread::JoinHandle<()>>,
    shortcuts: Arc<AtomicU32>,
}

impl Hook {
    pub fn install(hwnd: HWND) -> Result<Self> {
        let target = hwnd.0 as isize;
        let shortcuts = Arc::new(AtomicU32::new(0));
        let hook_shortcuts = Arc::clone(&shortcuts);
        let (ready, started) = std::sync::mpsc::channel();
        let handle = std::thread::Builder::new()
            .name("win-key".into())
            .spawn(move || unsafe {
                let installed = (|| -> Result<(HHOOK, HHOOK)> {
                    let module = GetModuleHandleW(None)?;
                    let keyboard = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), Some(module.into()), 0)?;
                    match SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), Some(module.into()), 0) {
                        Ok(mouse) => Ok((keyboard, mouse)),
                        Err(error) => {
                            let _ = UnhookWindowsHookEx(keyboard);
                            Err(error)
                        }
                    }
                })();
                let (keyboard, mouse) = match installed {
                    Ok(hooks) => hooks,
                    Err(error) => {
                        let _ = ready.send(Err(error));
                        return;
                    }
                };
                // Seed held keys when enabling midway through a chord.
                let mut tap = Tap::default();
                for vk in 8..256 {
                    // Low-level events identify the left/right modifier, not its
                    // generic alias. Mouse buttons are excluded from this range.
                    if !matches!(vk, 0x10..=0x12) && GetAsyncKeyState(vk) < 0 {
                        tap.key(vk as u32, true, false);
                    }
                }
                tap.candidate = None;
                KEYBOARD.with(|state| {
                    state.set(Keyboard {
                        tap,
                        ..Keyboard::default()
                    })
                });
                TARGET.with(|t| {
                    *t.borrow_mut() = Some(Target {
                        hwnd: HWND(target as _),
                        shortcuts: hook_shortcuts,
                    });
                });
                let mut msg = MSG::default();
                // Create the message queue before publishing the thread ID to Drop.
                let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
                let _ = ready.send(Ok(windows::Win32::System::Threading::GetCurrentThreadId()));
                while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
                    DispatchMessageW(&msg);
                }
                let _ = UnhookWindowsHookEx(keyboard);
                let _ = UnhookWindowsHookEx(mouse);
            })
            .map_err(|error| windows::core::Error::new(windows::core::HRESULT(-1), error.to_string()))?;
        match started.recv() {
            Ok(Ok(thread)) => Ok(Self {
                thread,
                handle: Some(handle),
                shortcuts,
            }),
            Ok(Err(error)) => {
                let _ = handle.join();
                Err(error)
            }
            Err(_) => {
                let _ = handle.join();
                Err(windows::core::Error::new(
                    windows::core::HRESULT(-1),
                    "The Win key hook stopped.",
                ))
            }
        }
    }

    pub fn configure(&self, launcher_tap: bool, desktop_modifiers: u32) {
        let launcher = if launcher_tap { LAUNCHER_TAP } else { 0 };
        self.shortcuts.store(launcher | desktop_modifiers, Ordering::Relaxed);
    }
}

impl Drop for Hook {
    fn drop(&mut self) {
        unsafe {
            let _ = PostThreadMessageW(self.thread, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let message = wparam.0 as u32;
        if matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP) {
            let target = TARGET.with(|target| {
                target
                    .borrow()
                    .as_ref()
                    .map(|t| (t.hwnd, t.shortcuts.load(Ordering::Relaxed)))
            });
            if let Some((hwnd, shortcuts)) = target {
                let action = KEYBOARD.with(|state| {
                    let mut keys = state.get();
                    let action = keys.key(
                        event.vkCode,
                        matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN),
                        event.flags.contains(LLKHF_INJECTED),
                        shortcuts,
                    );
                    state.set(keys);
                    action
                });
                match action {
                    Action::Consume => return LRESULT(1),
                    Action::Launcher if mask_start_menu() => {
                        let _ = unsafe { PostMessageW(Some(hwnd), WM_HOTKEY, WPARAM(0), LPARAM(0)) };
                    }
                    Action::Desktop(desktop) if mask_start_menu() => {
                        let chord = (event.vkCode << 16) | (shortcuts & MODIFIERS);
                        let posted = unsafe {
                            PostMessageW(
                                Some(hwnd),
                                WM_HOTKEY,
                                WPARAM(HOTKEY_FIRST + desktop),
                                LPARAM(chord as isize),
                            )
                        };
                        if posted.is_ok() {
                            KEYBOARD.with(|state| {
                                let mut keys = state.get();
                                keys.consume_digit(desktop);
                                state.set(keys);
                            });
                            return LRESULT(1);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    // Injected desktop-switch chords (dock/desktops.rs) pass through unchanged
    // and never change Tap, including their Win-down and Win-up events.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn mask_start_menu() -> bool {
    // Explorer must see a chord before Win-up. 0xE8 is unassigned and is
    // already used to mask Start when a bare Win tap opens the launcher.
    let key = |flags| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0xE8),
                dwFlags: flags,
                ..Default::default()
            },
        },
    };
    let inputs = [key(Default::default()), key(KEYEVENTF_KEYUP)];
    unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) == inputs.len() as u32 }
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32
        && matches!(
            wparam.0 as u32,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN | WM_MOUSEWHEEL | WM_MOUSEHWHEEL
        )
    {
        let event = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
        if event.flags & LLMHF_INJECTED == 0 {
            KEYBOARD.with(|state| {
                let mut keys = state.get();
                keys.tap.click();
                state.set(keys);
            });
        }
    }
    // Preserve Win+click and Win+wheel. Pointer movement alone allows a tap.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use super::*;

    const L: u32 = VK_LWIN.0 as u32;
    const R: u32 = VK_RWIN.0 as u32;
    const E: u32 = b'E' as u32;
    const ONE: u32 = b'1' as u32;
    const WIN: u32 = MOD_WIN.0;

    #[test]
    fn both_windows_keys_select_each_numbered_desktop() {
        for win in [L, R] {
            let mut keys = Keyboard::default();
            assert_eq!(keys.key(win, true, false, WIN), Action::Pass);
            for index in 0..HOTKEY_COUNT {
                let vk = ONE + index as u32;
                assert_eq!(keys.key(vk, true, false, WIN), Action::Desktop(index));
                keys.consume_digit(index);
                assert_eq!(keys.key(vk, false, false, WIN), Action::Consume);
            }
            assert_eq!(keys.key(win, false, false, WIN), Action::Pass);
        }
    }

    #[test]
    fn win_desktops_require_the_configured_modifiers_and_leave_other_chords_alone() {
        for (modifier, flag) in [
            (VK_RCONTROL, MOD_CONTROL.0),
            (VK_LSHIFT, MOD_SHIFT.0),
            (VK_RMENU, MOD_ALT.0),
        ] {
            let modifier = u32::from(modifier.0);
            let mut keys = Keyboard::default();
            assert_eq!(keys.key(L, true, false, WIN | flag), Action::Pass);
            assert_eq!(keys.key(ONE, true, false, WIN | flag), Action::Pass);
            assert_eq!(keys.key(ONE, false, false, WIN | flag), Action::Pass);
            assert_eq!(keys.key(modifier, true, false, WIN | flag), Action::Pass);
            assert_eq!(keys.key(E, true, false, WIN | flag), Action::Pass);
            assert_eq!(keys.key(ONE, true, false, WIN | flag), Action::Desktop(0));
            assert_eq!(keys.key(ONE, false, false, WIN | flag), Action::Pass);
            assert_eq!(keys.key(ONE, true, false, WIN), Action::Pass);
        }
        let mut keys = Keyboard::default();
        keys.key(u32::from(VK_LMENU.0), true, false, MOD_ALT.0);
        assert_eq!(keys.key(ONE, true, false, MOD_ALT.0), Action::Pass);
    }

    #[test]
    fn a_consumed_digit_ignores_repeats_and_keeps_its_release_after_the_modifiers_change() {
        let mut keys = Keyboard::default();
        keys.key(L, true, false, WIN);
        assert_eq!(keys.key(ONE, true, false, WIN), Action::Desktop(0));
        keys.consume_digit(0);
        assert_eq!(keys.key(ONE, true, false, WIN), Action::Consume);
        assert_eq!(keys.key(L, false, false, WIN), Action::Pass);
        assert_eq!(keys.key(ONE, false, true, 0), Action::Pass);
        assert_eq!(keys.key(ONE, true, false, 0), Action::Consume);
        assert_eq!(keys.key(ONE, false, false, 0), Action::Consume);
        assert_eq!(keys.key(ONE, true, false, 0), Action::Pass);
    }

    #[test]
    fn desktop_chords_coexist_with_a_bare_win_tap() {
        let mut keys = Keyboard::default();
        let shortcuts = WIN | LAUNCHER_TAP;
        keys.key(L, true, false, shortcuts);
        assert_eq!(keys.key(ONE, true, true, shortcuts), Action::Pass);
        assert_eq!(keys.key(L, false, false, shortcuts), Action::Launcher);
        keys.key(L, true, false, shortcuts);
        assert_eq!(keys.key(ONE, true, false, shortcuts), Action::Desktop(0));
        keys.consume_digit(0);
        assert_eq!(keys.key(ONE, false, false, shortcuts), Action::Consume);
        assert_eq!(keys.key(L, false, false, shortcuts), Action::Pass);
    }

    fn sequence(events: &[(u32, bool, bool)], expected: &[usize]) {
        let mut tap = Tap::default();
        let taps: Vec<_> = events
            .iter()
            .enumerate()
            .filter_map(|(i, &(vk, down, injected))| tap.key(vk, down, injected).then_some(i))
            .collect();
        assert_eq!(taps, expected);
    }

    #[test]
    fn lone_and_repeated_taps_on_either_side() {
        sequence(
            &[
                (L, true, false),
                (L, false, false),
                (R, true, false),
                (R, false, false),
                (L, true, false),
                (L, false, false),
            ],
            &[1, 3, 5],
        );
    }

    #[test]
    fn chords_cancel_even_after_other_key_released() {
        sequence(
            &[(L, true, false), (E, true, false), (E, false, false), (L, false, false)],
            &[],
        );
        sequence(
            &[(L, true, false), (E, true, false), (L, false, false), (E, false, false)],
            &[],
        );
        sequence(
            &[(E, true, false), (L, true, false), (E, false, false), (L, false, false)],
            &[],
        );
    }

    #[test]
    fn overlapping_win_keys_cancel_both_taps() {
        for (a, b) in [(L, R), (R, L)] {
            sequence(
                &[
                    (a, true, false),
                    (b, true, false),
                    (a, false, false),
                    (b, false, false),
                    (a, true, false),
                    (a, false, false),
                ],
                &[5],
            );
        }
    }

    #[test]
    fn win_auto_repeat_toggles_once() {
        sequence(
            &[
                (L, true, false),
                (L, true, false),
                (L, true, false),
                (L, false, false),
                (L, false, false),
            ],
            &[3],
        );
    }

    #[test]
    fn injected_events_do_not_start_cancel_or_complete_taps() {
        sequence(
            &[(L, true, true), (E, true, true), (E, false, true), (L, false, true)],
            &[],
        );
        sequence(
            &[
                (L, true, false),
                (E, true, true),
                (E, false, true),
                (L, false, true),
                (L, false, false),
            ],
            &[4],
        );
        // The dock injects Ctrl+Win+Arrow, all of which must be ignored.
        sequence(
            &[
                (0xA2, true, true),
                (L, true, true),
                (0x27, true, true),
                (0x27, false, true),
                (L, false, true),
                (0xA2, false, true),
            ],
            &[],
        );
    }

    #[test]
    fn mouse_click_cancels_until_next_tap() {
        let mut tap = Tap::default();
        assert!(!tap.key(L, true, false));
        tap.click();
        assert!(!tap.key(L, false, false));
        assert!(!tap.key(L, true, false));
        assert!(tap.key(L, false, false));
    }
}
