<p align="center">
  <img src="assets/povardock.png" width="112" alt="PovarDock icon">
</p>

<h1 align="center">PovarDock</h1>

<p align="center">A customizable dock and keyboard launcher for Windows 11.</p>

<p align="center">
  <a href="https://github.com/povargg/PovarDock/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/povargg/PovarDock?style=flat-square&amp;color=4f87ff"></a>
  <a href="LICENSE"><img alt="MIT license" src="https://img.shields.io/badge/license-MIT-4f87ff?style=flat-square"></a>
  <img alt="Windows 11 x64" src="https://img.shields.io/badge/windows-11%20x64-24334d?style=flat-square">
</p>

<p align="center">
  <a href="https://github.com/povargg/PovarDock/releases/latest/download/povardock-x64.exe"><strong>Download for Windows x64</strong></a>
  ·
  <a href="https://github.com/povargg/PovarDock/releases">Release notes</a>
</p>

PovarDock is a fork of Wisp by Sumir Seth. I adapted the dock for my own Windows
setup and am sharing it for anyone who wanted more control over the dock, its
appearance, and the way it fits into everyday work. [Original Wisp project](https://github.com/SumirSeth/wisp).

## What you can do

| Feature | What it gives you |
| --- | --- |
| **Keyboard launcher** | Press <kbd>Alt</kbd>+<kbd>Space</kbd> to find apps, files, windows, and emoji. The launcher also runs commands and handles calculations and web searches. |
| **Dock and tray** | Pin and reorder apps, switch between windows, open Jump Lists, and arrange the dock's sections. Choose a display and keep the dock visible or let it hide. |
| **System controls** | Reach the clock, volume, media, battery, Wi-Fi, and Bluetooth from the dock. The power menu gives you quick access to sleep, restart, and shutdown. |
| **Themes** | Use palettes such as Gruvbox, Tokyo Night, Nord, Catppuccin, and Rosé Pine. Adjust color and backdrop, or try Liquid Glass and tune its rim, refraction, and transparency. |
| **Virtual desktops** | Use the arrows beside the desktop number or press **Alt + 1–9** to select an existing desktop. Change the modifiers in **Settings → General**. |
| **Portable app** | The dock, launcher, and Settings live in the same executable. Check for new PovarDock builds under **Settings → About**. |

> [!NOTE]
> Liquid Glass is in early development. Its refraction and edges can still look rough on some backgrounds. Set **Transparency** to **0%** for an almost opaque surface.

## Install

1. Download `povardock-x64.exe` from the [latest release](https://github.com/povargg/PovarDock/releases/latest).
2. Move the file to a folder you can write to, such as `%LOCALAPPDATA%\Programs\PovarDock`, and run it.
3. Choose a launcher shortcut and whether to start the app with Windows in **Settings**. The default launcher shortcut is <kbd>Alt</kbd>+<kbd>Space</kbd>.

To replace an older portable build, exit PovarDock from its notification-area icon
and replace the executable in the same folder. Future updates are available on
the **About** page. The release includes `SHA256SUMS.txt` for checking the download.

## Build from source

Use Windows x64, the Rust toolchain in `rust-toolchain.toml`, Visual Studio C++
Build Tools for the MSVC toolchain, and the Windows SDK's `fxc.exe` shader
compiler. Set `GPUI_FXC_PATH` to the compiler path if it is outside your `PATH`.

```powershell
cargo build --locked --release -p wisp --bin wisp
.\scripts\package-release.ps1 -Executable .\target\release\wisp.exe -OutputDirectory .\dist
cargo test --workspace --locked
```

The package step creates `dist\povardock-x64.exe` and `dist\SHA256SUMS.txt`.

The release workflow runs on stable version tags. Bump the version in both Cargo
manifests, update `Cargo.lock`, and use a matching tag such as `v0.9.1`.

## Credits and license

Sumir Seth and the Wisp contributors built the original dock and launcher. The
original MIT license and copyright notice are included with the source.
[MIT license](LICENSE) · [Wisp contributors](https://github.com/SumirSeth/wisp/graphs/contributors).

PovarDock also includes emoji data from Gemoji and Unicode CLDR, and icons from
Lucide. [Gemoji notice](assets/LICENSE-GEMOJI) · [Unicode notice](assets/LICENSE-CLDR) · [Lucide notice](settings/assets/icons/LICENSE-LUCIDE).
