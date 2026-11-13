# Changelog

All notable changes are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/): while the major version is 0, minor
versions may still change things.

## [Unreleased]

## [0.1.0] - 2026-11-13

First public release.

### Added

- **Interface** inspired by the macOS Finder ("Lichtung" design): light/dark (can also be
  set explicitly), Mica on Windows 11, rounded corners, Segoe UI font, virtualized
  lists (tested with 100,000 entries).
- **Views:** list, tiles, columns (with a preview like the Finder), and a tree view
  with limits (depth, total, per folder) and export as `tree` text.
- **Navigation:** sidebar (favorites, OneDrive accounts, drives, network), breadcrumbs
  and typed paths, tabs, history, mouse back/forward buttons.
- **File operations** through the Windows shell (`IFileOperation`): copy, cut, paste,
  move, rename, new folder, delete with recycle bin and confirmation; clipboard
  compatible with the Explorer.
- **Drag & drop** inside the app (target highlighted live) and from/to other programs.
- **Context menu:** Windows builds the menu (7-Zip, Git, "Send to" …), the app draws it in
  its own style; "More options" shows the native menu.
- **Quick look** (Space) for images, text, and PDF (pdf.js bundled locally), Windows
  icons and thumbnails loaded asynchronously.
- **Search** in the folder and all subfolders (with `*` and `?`), type-ahead with the
  first letters, grouping (name, type, date, size), sorting.
- **Rubber-band selection** with the mouse, with auto-scroll.
- **Favorites:** pin via right-click, reorder by dragging.
- **OneDrive:** multiple accounts, status icons, "Always keep"/"Free up space" – without
  ever downloading files.
- **Network drives:** UNC paths, status with timeouts, sign-in through the Windows dialog
  (no passwords in the app).
- **Live updates** of folders (`notify`), settings (appearance, extensions, start view,
  limits, network timeout).
- **Installers** (NSIS per user, MSI) that run without administrator rights.

### Security

- Minimal Tauri capabilities, no file-system plugin, strict CSP.
- Every path is validated in the backend (traversal, junctions, reserved names,
  alternate data streams, long paths).
- Nothing is opened or executed automatically; executable types only after confirmation.

### Known limitations

- The installers are not code-signed (SmartScreen may warn).
- Much of this reaches deep into Windows and has so far only been tried by a few people
  on real machines. Details in the [manual](docs/MANUAL.md#known-limitations).
- The interface is German only for now.

[Unreleased]: https://github.com/theslasher5g/eX-plorer/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/theslasher5g/eX-plorer/releases/tag/v0.1.0
