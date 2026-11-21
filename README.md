<p align="center">
  <img src="assets/povardock-icon.png" width="112" alt="PovarDock icon">
</p>

<h1 align="center">PovarDock</h1>

<p align="center">A customizable dock and keyboard launcher for Windows 11.</p>

<p align="center">
  <strong>English</strong> · <a href="README.ru.md">Русский</a> · <a href="README.de.md">Deutsch</a>
</p>

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

## What you can do

With PovarDock, you can pull up an app, a file, or an emoji from the keyboard
and jump straight to the desktop you need. Pin the apps you open all the time
and give the dock a look you like with a color palette or Liquid Glass.

| Feature | What it gives you |
| --- | --- |
| **Keyboard launcher** | Open the launcher with <kbd>Alt</kbd>+<kbd>Space</kbd> to find apps, files, windows, or emoji. You can also run a command, do a calculation, or search the web from there. |
| **Dock and tray** | Pin your apps in the order you want, switch between windows, and open Jump Lists from the dock. You can move the sections around, choose a display, and keep the dock visible or let it hide. |
| **System controls** | The clock, volume, media controls, battery, Wi-Fi, and Bluetooth are right in the dock. Sleep, restart, and shutdown are all in the power menu. |
| **Themes** | Pick a palette like Gruvbox, Tokyo Night, Nord, Catppuccin, or Rosé Pine and adjust the color and backdrop. With Liquid Glass, you can also tune the rim, refraction, and transparency. |
| **Virtual desktops** | Use the arrows beside the number or jump straight to a desktop with **Alt + 1–9**. If that desktop is missing, the shortcut creates it and any desktops in between. The right arrow creates the next one when you're on the last desktop. You can change the modifiers in **Settings → General**. |
| **Portable app** | You get the dock, launcher, and settings in one executable. Check for new versions in **Settings → About**. |

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

## Origin and license

PovarDock is based on [Wisp](https://github.com/SumirSeth/wisp) by Sumir Seth and
its contributors. The original copyright notice and
[MIT license](LICENSE) are included with the source.

PovarDock also includes emoji data from Gemoji and Unicode CLDR, and icons from
Lucide. [Gemoji notice](assets/LICENSE-GEMOJI) · [Unicode notice](assets/LICENSE-CLDR) · [Lucide notice](settings/assets/icons/LICENSE-LUCIDE).
