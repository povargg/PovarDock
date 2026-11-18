// Keep a console for debug logs. Release builds run without a console.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod calc;
mod channel;
mod commands;
mod config;
mod deferred;
mod dock;
mod drag;
mod ease;
mod editor;
mod emoji;
mod files;
mod glass;
mod history;
mod hotkey;
mod indexer;
mod panic_log;
#[allow(dead_code)]
mod portfolio;
mod win_key;
// Settings also uses package-manager detection and update instructions.
#[allow(dead_code)]
mod install;
mod launch;
mod messages;
mod morph;
mod render;
mod screens;
mod search;
mod surface;
mod system;
mod taskbar;
mod theme;
mod tray;
mod tray_host;
mod update_notice;
mod util;

fn main() {
    panic_log::install();
    if std::env::args_os().nth(1).is_some_and(|arg| arg == "--settings") {
        wisp_settings::run(true);
        return;
    }
    if taskbar::dispatch_watchdog() {
        return;
    }
    if let Err(e) = app::run() {
        util::error_box(&format!("{} could not start:\n{}", channel::NAME, e.message()));
    }
}
