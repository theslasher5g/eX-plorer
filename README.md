<div align="center">

<img src="src-tauri/icons/icon.svg" width="96" alt="eX-plorer logo" />

# eX-plorer

**A modern file explorer for Windows 10 and 11, inspired by the macOS Finder.**
Fast, tidy, with a column view, search that looks into subfolders, and the real Windows shell underneath.

[![CI](https://github.com/theslasher5g/eX-plorer/actions/workflows/ci.yml/badge.svg)](https://github.com/theslasher5g/eX-plorer/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/theslasher5g/eX-plorer?display_name=tag&sort=semver)](https://github.com/theslasher5g/eX-plorer/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/Windows-10%20%7C%2011-0078D4?logo=windows&logoColor=white)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-stable-DEA584?logo=rust&logoColor=white)
![Svelte 5](https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte&logoColor=white)

[**Download**](https://github.com/theslasher5g/eX-plorer/releases/latest) ·
[Features](#features) ·
[Usage](#usage) ·
[Build from source](#build-from-source) ·
[Security](#security) ·
[Manual](docs/MANUAL.md)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/media/hero-dark.png">
  <img src="docs/media/hero-light.png" alt="eX-plorer in column view with a preview pane" width="900">
</picture>

<sub>Screenshots and GIFs come from the demo mode with made-up data. The app's interface is currently in German.</sub>

</div>

---

> [!NOTE]
> **Early version (0.1).** Much of this reaches deep into Windows (shell, drag & drop, OneDrive,
> network) and has so far only been tried by a few people on real machines. Bug reports are welcome.
> The honest limits are listed in the [manual](docs/MANUAL.md#known-limitations).

> [!NOTE]
> **Language:** the user interface is German only for now. All texts live in one file
> (`src/lib/i18n/de.ts`), so adding a translation is mostly a matter of providing a second file.

## Features

<table>
<tr>
<td width="50%" valign="top">

### 🗂 Four views
**List**, **tiles**, **columns** like in the Finder (with a preview on the right), and a
**tree view** with hard limits. Smooth even with 100,000 files because everything is
virtualized. `Ctrl+1…4` switches views.

</td>
<td width="50%" valign="top">

### 🔎 Search that looks into subfolders
Typing filters instantly, then searches **all subfolders** in the background
(`*` and `?` work as wildcards). **Type-ahead** jumps to a file as you type
the first letters of its name.

</td>
</tr>
<tr>
<td valign="top">

### 🖱 Power-user touches
**Rubber-band** selection, **drag & drop** with a live-highlighted target,
**grouping** by name, type, date, and size, **tabs**, and favorites you pin with
a right-click and reorder by dragging.

</td>
<td valign="top">

### 🪟 The real Windows shell
Copy, move, and delete run through `IFileOperation` (recycle bin, conflict dialogs).
The **context menu** is built by Windows – 7-Zip, Git & co. show up – and drawn in the
app's own style.

</td>
</tr>
<tr>
<td valign="top">

### ☁️ OneDrive and network
Multiple OneDrive accounts with **status icons**, "Always keep on this device" and
"Free up space" – without ever downloading a file. **Network drives** with timeouts so
the UI never hangs; sign-in goes through the Windows dialog.

</td>
<td valign="top">

### 👁 Quick look
`Space` previews images, text, and **PDF** (pdf.js, bundled locally) without opening
the file. Icons and thumbnails come asynchronously from Windows.

</td>
</tr>
<tr>
<td valign="top">

### 🌗 Light, dark, Mica
Follows Windows or can be set explicitly; Mica backdrop on Windows 11,
rounded corners, Segoe UI Variable.

</td>
<td valign="top">

### 🔒 Security first
Minimal Tauri permissions, **every path is validated in the backend**, strict CSP,
nothing is opened automatically. Runs without administrator rights.
[More below](#security)

</td>
</tr>
</table>

### In action

<table>
<tr>
<td width="50%"><img src="docs/media/views.gif" alt="Switching between list, tiles, columns, and tree" /><br><sub><b>Views:</b> list, tiles, columns with preview, tree</sub></td>
<td width="50%"><img src="docs/media/search.gif" alt="Subfolder search and type-ahead" /><br><sub><b>Search:</b> subfolders, wildcards, type-ahead</sub></td>
</tr>
<tr>
<td><img src="docs/media/drag.gif" alt="Rubber-band selection, drag and drop, grouping" /><br><sub><b>Selecting &amp; dragging:</b> rubber band, drag &amp; drop, grouping</sub></td>
<td><img src="docs/media/favorites.gif" alt="Context menu, pinning favorites, dark mode" /><br><sub><b>Menu &amp; favorites:</b> pin, reorder, dark theme</sub></td>
</tr>
</table>

### More screenshots

<table>
<tr>
<td width="33%"><img src="docs/media/context-menu.png" alt="Context menu with submenu"><br><sub>Context menu with submenus</sub></td>
<td width="33%"><img src="docs/media/list-grouped.png" alt="List grouped by date modified"><br><sub>Grouped by date modified</sub></td>
<td width="33%"><img src="docs/media/tiles.png" alt="Tile view"><br><sub>Tile view</sub></td>
</tr>
<tr>
<td><img src="docs/media/search.png" alt="Search results from subfolders"><br><sub>Hits from subfolders</sub></td>
<td><img src="docs/media/onedrive.png" alt="OneDrive with status icons"><br><sub>OneDrive status</sub></td>
<td><img src="docs/media/network.png" alt="Network drives with status"><br><sub>Network drives</sub></td>
</tr>
<tr>
<td><img src="docs/media/settings.png" alt="Settings"><br><sub>Settings</sub></td>
<td><img src="docs/media/quicklook.png" alt="Quick look"><br><sub>Quick look (PDF)</sub></td>
<td><img src="docs/media/tree.png" alt="Tree view"><br><sub>Tree view with limits</sub></td>
</tr>
</table>

## Install

1. Download one of these from the [latest release](https://github.com/theslasher5g/eX-plorer/releases/latest):
   - **`eX-plorer_…_x64-setup.exe`** – installs **per user**, no administrator rights needed *(recommended)*
   - **`eX-plorer_…_x64_de-DE.msi`** – system-wide, asks for administrator rights
2. Optionally verify the checksum against `SHA256SUMS.txt` (`Get-FileHash <file>` in PowerShell).
3. Run it. The WebView2 runtime (present on Windows 11) is downloaded if it is missing.

> [!WARNING]
> The installers are **not code-signed**, so Windows SmartScreen may show "Windows protected your PC".
> Choose *More info → Run anyway* to continue. If you would rather not, [build it yourself](#build-from-source).

**Requirements:** Windows 10 (version 1809 or later) or Windows 11, x64.

## Usage

| Action | Input |
| --- | --- |
| Switch view | `Ctrl+1` list · `Ctrl+2` tiles · `Ctrl+3` columns · `Ctrl+4` tree |
| Search (including subfolders) | `Ctrl+F`, then type – `*` and `?` are wildcards |
| Jump to a file | Type the first letters of its name |
| Quick look | `Space` |
| Enter a path | `Ctrl+L` or `F4` |
| Back / forward / up | `Alt+←` / `Alt+→` / `Alt+↑` (or mouse buttons 4/5, Backspace) |
| New / close / switch tab | `Ctrl+T` / `Ctrl+W` / `Ctrl+Tab` |
| Copy, cut, paste | `Ctrl+C` / `Ctrl+X` / `Ctrl+V` (also works with the Windows Explorer) |
| Rename, delete | `F2` · `Del` (recycle bin), `Shift+Del` (permanent) |
| New folder | `Ctrl+Shift+N` |
| Context menu | Right-click, `Menu` key, or `Shift+F10` |
| Pin a favorite | Right-click a folder, or the empty area to pin the open folder |
| Select with the mouse | Drag on empty space (`Ctrl`/`Shift` keep the selection) |
| Settings | `Ctrl+,` |

Full workflows, limits, and notes are in the [manual](docs/MANUAL.md#usage).

## Build from source

Requirements: [Rust](https://rustup.rs) (stable, MSVC toolchain, "Desktop development with C++" from the
Visual Studio Build Tools), [Node.js](https://nodejs.org) 22 or newer, Windows 10/11.

```bash
git clone https://github.com/theslasher5g/eX-plorer.git
cd eX-plorer
npm install
npm run tauri dev        # development mode with hot reload
npm run tauri build      # installers (NSIS + MSI) in src-tauri/target/release/bundle/
```

Just the interface in a browser, with made-up demo data and no file-system access
(handy on Linux/macOS too):

```bash
npm run dev
```

Checks (the CI runs the same on `windows-latest`):

```bash
npm run check
cd src-tauri
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo audit
```

## Architecture

```
src/                    Svelte 5 + TypeScript (UI, stores, demo backend)
src-tauri/
  crates/core/          Platform-neutral core: path validation, listing, search, tree, settings …
  src/                  Tauri app: commands and a thin Windows layer (shell, COM, registry, WNet)
docs/                   Manual, media
```

The frontend has **no** file-system access; it only talks to its own validating Tauri
commands. The core is testable without Windows (`cargo test -p explorer-core`); the
Windows calls live in small, clearly bounded modules. More in the [manual](docs/MANUAL.md#architecture).

## Security

- **Minimal permissions:** no Tauri plugins for the file system or shell; capabilities only cover window control.
- **Every path is validated in the backend** – traversal, junctions, reserved names, alternate data streams, long paths.
- **Strict CSP**, no remote content, no `eval`, file names are never rendered as HTML.
- **Nothing is opened or executed automatically**; programs (exe, bat, ps1 …) only after confirmation.
- **Metadata only when listing:** OneDrive placeholders are never read, network paths have timeouts.
- `unsafe` only in small Windows modules, `unwrap()` banned by lint, `clippy -D warnings` and `cargo audit` in CI.

Found a vulnerability? Please report it **privately** – see [SECURITY.md](SECURITY.md).

## Contributing

Report bugs, suggest ideas, send code – see [CONTRIBUTING.md](CONTRIBUTING.md).
You publish a new version with `npm run version:set -- x.y.z`, an entry in the
[changelog](CHANGELOG.md), and a `vx.y.z` tag – or by running the
[release workflow](.github/workflows/release.yml) under *Actions → Release → Run workflow*,
which builds the installers and publishes them.

## Acknowledgements

Built with [Tauri](https://tauri.app), [Svelte](https://svelte.dev), [pdf.js](https://mozilla.github.io/pdf.js/),
[Lucide](https://lucide.dev), and the [`windows` crate](https://github.com/microsoft/windows-rs).
The look follows the "Lichtung" design.

## License

[MIT](LICENSE) © 2026 theslasher5g
