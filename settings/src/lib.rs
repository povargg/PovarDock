mod assets;
#[path = "../../src/dock/pins.rs"]
mod pins;

// Shared with the launcher; this app uses only some of the names.
#[allow(dead_code)]
#[path = "../../src/channel.rs"]
mod channel;
mod config;
mod hotkey;
#[path = "../../src/install.rs"]
mod install;
mod palette;
#[path = "../../src/portfolio.rs"]
mod portfolio;
// Shared with the launcher, which also finds a screen by its ID.
#[allow(dead_code)]
#[path = "../../src/screens.rs"]
mod screens;
mod system;
mod ui;
mod update;
#[path = "../../src/update_notice.rs"]
mod update_notice;

pub use update::cleanup_previous_update;
pub const REPOSITORY_URL: &str = "https://github.com/povargg/WinDock";

use gpui_kit::{AppContext, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size};

pub fn run(embedded: bool) {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--check-updates") {
        update::check_hidden();
        return;
    }
    let welcome = args.iter().any(|a| a == "--welcome");
    // `--page search` opens a page directly, for screenshots and testing.
    let page = args
        .iter()
        .position(|a| a == "--page")
        .and_then(|i| args.get(i + 1))
        .cloned();
    if embedded {
        cleanup_previous_update();
    }
    gpui_kit::application()
        .with_assets(crate::assets::SettingsAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            let (title, width, height) = if welcome {
                (channel::WELCOME_NAME, 560., 460.)
            } else {
                (channel::SETTINGS_NAME, 820., 580.)
            };
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(width), px(height)),
                    cx,
                ))),
                titlebar: Some(TitlebarOptions {
                    title: Some(title.into()),
                    ..Default::default()
                }),
                window_min_size: Some(size(px(520.), px(420.))),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| ui::Settings::new(welcome, page.as_deref(), embedded, window, cx))
            })
            .expect("failed to open Windock Settings");
            cx.activate(true);
        });
}
