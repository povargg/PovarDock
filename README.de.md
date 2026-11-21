<p align="center">
  <img src="assets/povardock-icon.png" width="112" alt="PovarDock-Symbol">
</p>

<h1 align="center">PovarDock</h1>

<p align="center">Ein anpassbares Dock mit Tastatur-Launcher für Windows 11.</p>

<p align="center">
  <a href="README.md">English</a> · <a href="README.ru.md">Русский</a> · <strong>Deutsch</strong>
</p>

<p align="center">
  <a href="https://github.com/povargg/PovarDock/releases/latest"><img alt="Aktuelle Version" src="https://img.shields.io/github/v/release/povargg/PovarDock?style=flat-square&amp;color=4f87ff"></a>
  <a href="LICENSE"><img alt="MIT-Lizenz" src="https://img.shields.io/badge/license-MIT-4f87ff?style=flat-square"></a>
  <img alt="Windows 11 x64" src="https://img.shields.io/badge/windows-11%20x64-24334d?style=flat-square">
</p>

<p align="center">
  <a href="https://github.com/povargg/PovarDock/releases/latest/download/povardock-x64.exe"><strong>Für Windows x64 herunterladen</strong></a>
  ·
  <a href="https://github.com/povargg/PovarDock/releases">Versionshinweise</a>
</p>

## Was PovarDock kann

Apps, Dateien und Emojis lassen sich direkt per Tastatur finden. Mit einem
Tastenkürzel geht es zum gewünschten virtuellen Desktop. Häufig genutzte Apps
können im Dock angeheftet werden. Für die Optik stehen Farbschemata und
Liquid Glass zur Auswahl.

| Funktion | Was möglich ist |
| --- | --- |
| **Launcher** | <kbd>Alt</kbd>+<kbd>Leertaste</kbd> öffnet die Suche nach Apps, Dateien, Fenstern und Emojis. Dort lassen sich auch Befehle ausführen, Rechnungen lösen und Websuchen starten. |
| **Dock und Infobereich** | Apps lassen sich anheften und anordnen. Das Dock erlaubt auch den Wechsel zwischen Fenstern und den Zugriff auf Sprunglisten. Bereiche und Bildschirm sind einstellbar. Das Dock kann sichtbar bleiben oder sich automatisch ausblenden. |
| **Systemfunktionen** | Im Dock gibt es Uhr, Lautstärke, Mediensteuerung, Akkustand, WLAN und Bluetooth. Im Energiemenü stehen Energiesparmodus, Neustart und Herunterfahren zur Auswahl. |
| **Designs** | Zur Auswahl stehen unter anderem Gruvbox, Tokyo Night, Nord, Catppuccin und Rosé Pine. Farbe und Hintergrund lassen sich anpassen. Bei Liquid Glass sind auch Rand, Lichtbrechung und Transparenz einstellbar. |
| **Virtuelle Desktops** | Mit den Pfeilen neben der Nummer oder mit **Alt + 1–9** lässt sich der Desktop wechseln. Das Tastenkürzel legt den gewählten Desktop an, falls er fehlt, und erstellt auch die Desktops dazwischen. Auf dem letzten Desktop legt der rechte Pfeil einen weiteren an. Die Modifikatortasten sind unter **Settings → General** einstellbar. |
| **Portable Anwendung** | Dock, Launcher und Einstellungen stecken in derselben ausführbaren Datei. Unter **Settings → About** lässt sich nach neuen Versionen von PovarDock suchen. |

> [!NOTE]
> Liquid Glass befindet sich in einer frühen Entwicklungsphase. Auf manchen
> Hintergründen können Lichtbrechung und Ränder noch ungleichmäßig aussehen.
> Bei einem Wert von **0 %** für **Transparency** ist die Oberfläche nahezu
> undurchsichtig.

## Installation

1. Laden Sie `povardock-x64.exe` von der [Seite der aktuellen Version](https://github.com/povargg/PovarDock/releases/latest) herunter.
2. Verschieben Sie die Datei in einen Ordner mit Schreibzugriff, etwa `%LOCALAPPDATA%\Programs\PovarDock`, und führen Sie sie aus.
3. Legen Sie unter **Settings** das Tastenkürzel für den Launcher fest und wählen Sie, ob die Anwendung mit Windows gestartet werden soll. Standardmäßig öffnet <kbd>Alt</kbd>+<kbd>Leertaste</kbd> den Launcher.

Zum Ersetzen einer älteren portablen Version beenden Sie PovarDock über das Symbol
im Infobereich und ersetzen Sie die ausführbare Datei im selben Ordner. Weitere
Updates stehen unter **About** zur Verfügung. Die Datei `SHA256SUMS.txt` ist
dem Release zur Überprüfung des Downloads beigefügt.

## Aus dem Quellcode kompilieren

Für den Build werden Windows x64, die in `rust-toolchain.toml` angegebene
Rust-Toolchain, die Visual Studio C++ Build Tools für MSVC und der
Shader-Compiler `fxc.exe` aus dem Windows SDK benötigt. Falls das Verzeichnis
des Compilers nicht in `PATH` enthalten ist, setzen Sie `GPUI_FXC_PATH` auf
den Pfad zu `fxc.exe`.

```powershell
cargo build --locked --release -p wisp --bin wisp
.\scripts\package-release.ps1 -Executable .\target\release\wisp.exe -OutputDirectory .\dist
cargo test --workspace --locked
```

Beim Paketieren werden `dist\povardock-x64.exe` und `dist\SHA256SUMS.txt` erstellt.

Der Release-Workflow wird durch Tags für stabile Versionen ausgelöst. Passen Sie
die Version in beiden Cargo-Manifesten an, aktualisieren Sie `Cargo.lock` und
verwenden Sie einen passenden Tag wie `v0.9.1`.

## Herkunft und Lizenz

PovarDock basiert auf [Wisp](https://github.com/SumirSeth/wisp) von Sumir Seth
und weiteren Mitwirkenden. Der ursprüngliche Urheberrechtshinweis und die
[MIT-Lizenz](LICENSE) liegen dem Quellcode bei.

PovarDock enthält außerdem Emoji-Daten aus Gemoji und Unicode CLDR sowie Symbole
von Lucide. [Hinweis zu Gemoji](assets/LICENSE-GEMOJI) · [Hinweis zu Unicode](assets/LICENSE-CLDR) · [Hinweis zu Lucide](settings/assets/icons/LICENSE-LUCIDE).
