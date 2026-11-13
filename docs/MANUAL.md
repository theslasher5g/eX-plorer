# eX-plorer – Manual

A modern file explorer for Windows 10/11, inspired by the macOS Finder.
Backend in Rust with Tauri 2, interface in Svelte 5 + TypeScript.

> Detailed manual: usage, architecture, security, measurements, and **honest
> limitations**. The overview with pictures is in the [README](../README.md).
>
> **Status: version 0.1.0.** All seven planned phases are implemented. **Most of it has
> so far only run in CI and in the browser with demo data; feedback from real Windows
> machines is explicitly welcome.**
>
> The interface is German only for now; menu and setting names below are given in
> English with the German label in quotes where that helps.

The look follows the "Lichtung" design: the tokens (colors, radii, shadows, motion)
live in `src/app.css`. Light and dark follow the system or can be set in the settings.

## Requirements

- Windows 10 (version 1809 or later) or Windows 11
- [Rust](https://rustup.rs) (stable, MSVC toolchain) with "Desktop development with C++"
  from the Visual Studio Build Tools
- [Node.js](https://nodejs.org) 22 or newer
- WebView2 runtime (preinstalled on Windows 11)

## Run and build

```bash
npm install
```

Development mode with hot reload:

```bash
npm run tauri dev
```

Release build with installers (NSIS and MSI) in `src-tauri/target/release/bundle/`:

```bash
npm run tauri build
```

Just the interface in a browser, with made-up demo data and no access to the file
system (handy for design work, also on Linux/macOS):

```bash
npm run dev
```

## Checks

```bash
npm run check
```

In the `src-tauri` folder:

```bash
cargo fmt --all --check
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
cargo test --workspace
```

```bash
cargo audit
```

The same steps run in GitHub Actions on `windows-latest` (`.github/workflows/ci.yml`);
the finished installer is attached to the run as an artifact.

### Measurements (as of phase 7)

Measured on Linux in the development environment, so **not** on Windows: the numbers
show orders of magnitude, not the speed of real file-system calls.

| Measurement | Result |
|---|---|
| Core: list 100,000 files (in batches of 2000) | about 0.5 s; the test has a 20 s ceiling against quadratic behavior |
| UI, 100,000 entries (demo backend, headless Chromium) | first row after about 0.35 s, list ~500 DOM nodes, scrolling median 16.5 ms/frame |
| Ctrl+A on 100,000 entries | about 0.1 s |
| Sorting 100,000 entries | about 0.4 s |
| Switching list / tiles / columns with 100,000 entries | under 50 ms, 400–750 DOM nodes |

The lists are virtualized (fixed row height); the number of DOM nodes does not depend
on the folder size. A measurement with real folders, network drives, and thumbnails is
still outstanding.

## Usage

| Action | Input |
| --- | --- |
| Open a folder | Double-click or Enter |
| Back / forward | Alt+Left / Alt+Right, mouse buttons 4/5 |
| Parent folder | Alt+Up or Backspace |
| Type a path | Ctrl+L or F4, or click to the right of the path |
| Search (folder and subfolders) | Ctrl+F, then type; `*` and `?` are wildcards (`*.pdf`, `inv?ice*`) |
| Jump to a file by keyboard | Type the first letters of its name; the same letter repeatedly jumps to the next match |
| New tab / close tab / switch tab | Ctrl+T / Ctrl+W or middle-click / Ctrl+Tab |
| Refresh | F5 |
| Selection | Click, Ctrl+click, Shift+click, arrow keys, Ctrl+A |
| Sort, show hidden items | Column header or the sort menu in the toolbar |
| Copy / cut / paste | Ctrl+C / Ctrl+X / Ctrl+V (also with the Explorer and other programs) |
| Delete | Del (recycle bin, with confirmation), Shift+Del (permanent, with confirmation) |
| Rename | F2; Enter commits, Esc discards, clicking away commits |
| New folder | Ctrl+Shift+N or the toolbar button |
| Move / copy with the mouse | Drag onto a folder, tab, breadcrumb segment, or sidebar entry; Ctrl forces copy, Shift forces move |
| Drag into other programs | Drag the selected files out of the window (the real Windows drag takes over) |
| Open a file | Double-click or Enter (executable types only after confirmation) |
| Quick look | Space; arrow keys switch files, Page Up/Down for PDF pages, Esc or Space closes |
| Group | Sort menu → "Group by" ("Gruppieren nach", list and tiles); clicking a group header selects the group |
| Select by dragging | Press on empty space and drag; Ctrl/Shift keep the selection |
| Switch view | Ctrl+1 list, Ctrl+2 tiles, Ctrl+3 columns, Ctrl+4 tree, or the switcher in the toolbar |
| Column view | Clicking a folder opens it to the right, clicking a file shows the preview; Up/Down select, Right/Left change column, Enter or double-click opens (folders: the tab navigates there) |
| Tree view | Right/Left expands/collapses, Enter opens, clicking "… and N more" loads more |
| Pin a favorite | Right-click a folder → "Pin to Quick access" ("An Schnellzugriff anheften", or "An Favoriten anheften"); right-clicking empty space pins the open folder; unpin via right-click on the sidebar entry or in the folder menu; reorder by dragging (or Alt+Up/Down) |
| Settings | Ctrl+, or the entry at the bottom of the sidebar |
| OneDrive: keep / free up | Right-click files or folders in the OneDrive folder: "Always keep on this device" / "Free up space" |
| Network | "Network drives" ("Netzlaufwerke") entry in the sidebar, or type `\\server\share` in the path bar |
| Context menu | Menu key or Shift+F10 (at the selected row), or right-click a file, folder, or empty space (Shift+right-click shows extended entries); "More options …" shows the Windows menu |

## Architecture

```
src-tauri/
  crates/core/        Platform-neutral core, testable without Tauri
    src/path/         Path validation and normalization
    src/listing/      Listing folders (Windows: FindFirstFileExW)
    src/search/       Name search in subfolders with limits, cancellable
    src/ops/          Planning copy/move/delete/rename, new folder
    src/tree/         Tree scan with limits (depth, total, per folder), cancellable
    src/settings.rs   Settings with enforced value ranges
    src/preview.rs    Preview rules (kind, limits) and safe text decoding
    src/imaging.rs    BGRA → PNG for icons and thumbnails
    src/exec.rs       Which file types need confirmation before opening
    src/attrs.rs      Cloud placeholders, sync state, pin attributes
    src/network.rs    Error classification, validation of server input
    src/timeout.rs    Timeout and cool-down for blocking network calls
    src/onedrive.rs   Merging OneDrive accounts (registry, environment)
    src/watch.rs      Folder watcher (notify), batched notifications
    src/winerr.rs     HRESULT/Win32 error codes → error kinds
  src/
    commands/         Tauri commands (the only way from the frontend into the system)
    shell/            Windows shell calls: known folders, drives,
                      IFileOperation, clipboard (CF_HDROP), OLE drag, key state,
                      icons/thumbnails, context menu (own thread), opening files,
                      OneDrive (accounts, pin/unpin), registry (read-only),
                      network (drives, sign-in, shares)
    protocol.rs       shellimg URI scheme (icons/thumbnails as PNG)
    workers.rs        Worker pool for image requests (newest first, bounded queue)
    state.rs          Window material (Mica), network timeouts
    error.rs          Errors with stable codes for the frontend
  capabilities/       Tauri permissions of the window
src/
  lib/api/            Typed backend interface, demo backend for the browser
  lib/stores/         Folder view (Pane), tabs, file operations, drag & drop,
                      search, favorites, quick look, toast and confirm dialog
  lib/components/     Title bar with tabs, sidebar, toolbar, path bar, sort menu,
                      list, tiles, columns, tree, context menu, settings
  app.css             Design tokens and shared building blocks
  lib/i18n/           Texts (currently German)
```

## Security

- **No file-system access in the frontend.** There are no Tauri plugins for the
  file system or shell. The capabilities only allow window control and window
  events; everything else goes through the app's own commands.
- **Every path is validated in the backend** (`crates/core/src/path`): absolute,
  no `..` components, no device namespaces (`\\.\`, `\\?\`), no alternate data
  streams, no reserved names (CON, NUL, COM1 …), no components ending in a dot or
  space, length limit. Accesses then use the `\\?\` form so Windows cannot
  reinterpret the validated path.
- **Listing reads metadata only.** No file is opened; OneDrive placeholders are
  therefore not downloaded.
- **Nothing is opened or executed automatically.** Files open only on double-click,
  Enter, or a menu command, and types that can run code (exe, bat, ps1, msi, lnk,
  js …) only after explicit confirmation. The list of these types lives in the
  backend (`exec.rs`, tested).
- **OneDrive works with metadata only.** The sync state comes from the directory
  entries (attributes, reparse tag, Cloud Files API `CfGetPlaceholderStateFromFindData`);
  "Keep" and "Free up" only set the PINNED/UNPINNED attributes; downloading and
  freeing is done by the OneDrive client. The registry is only read, and setting is
  restricted to paths inside the detected OneDrive folders (not their root).
- **Network accesses have a timeout and never block.** Every access runs in its own
  thread; after 4 s (probe), 8 s (shares), or 10 s (folders) the UI gets "not
  reachable". A server that timed out is treated as hanging for 15 s, and further
  requests fail immediately instead of piling up hanging threads. Network paths get no
  icons, thumbnails, or automatic deep search (the tree reads one level). The limit is
  adjustable in the settings (5–60 s).
- **Sign-in goes through Windows, not the app.** The app has no password fields and
  stores nothing: `WNetAddConnection3W` opens the Windows dialog and, if you wish,
  stores the data in the Credential Manager. This deliberately differs from the
  sign-in dialog in the design, which shows fields in the app.
- **Cloud placeholders are never read.** For "cloud only" files (OneDrive) the
  backend creates neither icons nor thumbnails nor previews; the attributes are
  checked beforehand via metadata. The same applies to icons on network paths.
- **Previews are limited:** images up to 40 MiB, PDF up to 64 MiB, text the first
  256 KiB. SVG is shown as text, never as an image, so nothing can be executed.
  Text is displayed via `textContent`, never as HTML.
- **Operations are secured twice.** The core plans (`ops`): no drive/share roots, no
  folder into itself, duplicate and nested selections are merged, a ceiling of 20,000
  items. Execution goes through the Windows shell, which knows the recycle bin and
  conflicts.
- **Deleting always asks.** Where there is no recycle bin (network drive, USB, UNC)
  the dialog says so clearly; the shell asks as well.
- **The tree scan is limited and cancellable.** Depth, total count, and entries per
  folder are enforced in the backend (also against manipulated calls); symlinks and
  junctions are never entered automatically; on network drives only one level is read
  automatically. The memory needed per folder does not depend on its size.
- **Search is limited and cancellable.** At most 5,000 hits, 400,000 entries read,
  depth 32; links and junctions are never entered, cloud-only folders are skipped, and
  network paths are not searched recursively.
- **Context menu commands are checked.** The ID sent by the UI must exist in the
  currently open menu before it is executed.
- **Strict CSP**, no remote content. Relaxations and why: `style-src 'unsafe-inline'`
  (Svelte sets inline styles for the virtualized list), `script-src 'wasm-unsafe-eval'`
  (pdf.js uses WebAssembly for JPEG2000; it does not allow `eval`), `connect-src 'self'`
  and `worker-src 'self' blob:` (pdf.js loads fonts and its worker from the app),
  `img-src 'self' blob: http://shellimg.localhost` (previews, icons, and menu icons as
  blob URLs; deliberately no `data:`), and `font-src 'self' data: blob:` (fonts
  generated by pdf.js). Plus `object-src 'none'`, `base-uri 'none'`, `form-action 'none'`,
  and `frame-ancestors 'none'`.
- **File names** are only rendered as text, never via `{@html}` or `innerHTML`.
- `unsafe` appears only in `crates/core/src/listing/win.rs` and `src-tauri/src/shell/`.
  `unwrap()`/`expect()` are banned by a Clippy lint.
- The program needs no admin rights; the NSIS installer installs per user
  (`installMode: currentUser`). The MSI installer is system-wide by Tauri/WiX design
  and therefore asks for administrator rights.
- **Review in phase 7** (code read, not tested from the outside): every command that
  takes a path calls `validate_path`; menu commands are checked against the entries of
  the open menu before `InvokeCommand` runs; there are no plugins for the file system
  or shell, no remote URLs, no `eval`; file names never reach `{@html}`. `cargo audit`
  reports no vulnerabilities, only two notices about unmaintained GTK libraries (Linux
  dependencies of Tauri, not part of Windows builds). **This does not replace an
  independent security audit.**

## Known limitations

- **Phases 2 to 6 have not been tested on real Windows yet.** File operations, the
  clipboard, dragging into other programs, icon retrieval, the native context menu,
  and opening files reach deep into Windows and can only be verified there. They
  compile and are statically checked; the UI flows ran in the browser against a demo
  backend. Please test especially: Ctrl+C in the Explorer and Ctrl+V here (and the
  other way round), dragging files into and out of the Explorer, right-click (does
  7-Zip appear?), icons and quick look in a folder with many images.
- **Context menu (own rendering):** Windows builds the classic menu (like "Show more
  options" in Windows 11, so with 7-Zip, Git, "Send to", etc.), but the app does not
  display it: the backend reads text, icon, check marks, submenus, and shortcuts, and
  the UI draws them in the app's style (light/dark, rounded corners; mouse and
  keyboard: arrows, Right/Left for submenus, Enter, Esc). A click executes the command
  through Windows; submenus are loaded only when opened. Entries that only draw
  themselves ("owner-drawn", without text or with a callback icon) cannot be shown;
  for those there is **"More options …"** at the bottom, which shows the real Windows
  menu at the mouse position. Cut, copy, paste, delete, rename, and open are executed
  by the app itself (so confirmations and display are right), everything else by the
  shell. Entries of modern packaged apps that only appear in the new menu are partly
  missing. **Not yet seen on real Windows:** the rendering of icons (alpha channel),
  submenus of certain extensions, and the order of entries should be checked on the
  first run. "New" entries on empty space create files but do not select them for
  renaming.
- **Icons:** files show the Windows icon or thumbnail (32 px, visible rows only, three
  worker threads); while it loads, or where nothing may be generated (cloud, network),
  the drawn type symbol appears. Folders keep the design's drawn icon. The backend
  un-premultiplies alpha values; for partially transparent edge pixels this can differ
  slightly.
- **Quick look:** images, text, and PDF. Other types show name, kind, size, date and,
  if Windows provides one, a large thumbnail. No video or audio playback; for PDF no
  text selection and no links. PDFs with fonts defined only through Adobe CMaps
  (mainly CJK) and not embedded may render incorrectly; the CMaps are not included.
  pdf.js adds about 4 MB to the app and is only loaded on the first PDF.
- **Files on network drives** are read for the preview (on an explicit action); there
  is no timeout for that yet, nor for file operations (copy, delete). Windows shows
  its own progress and error dialog there.
- **Network:**
  - There is no automatic discovery of computers on the network, because Windows
    itself can hardly do that reliably any more. The "Shares on the network" list shows
    the servers you add (stored in the settings); mapped drives appear automatically.
  - Shares are listed with `NetShareEnum`; administrative shares (`C$`, `IPC$`) stay
    hidden. Some servers refuse to list, in which case typing `\\server\share`
    directly is the only way.
  - "Sign in" on a server connects `\\server\IPC$`; that unlocks the share list but
    does not assign a drive letter. The app does not hand out new drive letters (the
    Explorer does that under "Map network drive"); an existing drive can be reconnected.
  - For Windows errors I cannot classify, "not reachable" is shown. "Access denied" at
    the root of a share counts as "sign-in required".
  - The context menu thread is single: if a right-click hangs on a dead network path,
    further menus wait until it returns.
- **Progress and conflicts are shown by Windows**, not the app: for large operations
  the familiar Windows dialog appears, as for name conflicts. There is no progress
  card of its own.
- **No undo in the app.** Deleted items can be restored from the recycle bin; there is
  no Ctrl+Z.
- **Drops from other programs** are reported by the window as a copy. Whether it
  becomes copy or move is decided by the app: same drive moves, another drive copies,
  and Ctrl and Shift override that. The Explorer therefore does not delete the source
  itself.
- **Live updates** only watch the folders of the open tabs, not recursively and not on
  network drives (F5 helps there). Very many quick changes are batched; the display
  can lag by fractions of a second.
- **Paths are passed to the shell in normal form;** paths over 260 characters only
  work if Windows has long paths enabled.
- **Tree view:** it shows and opens but does not edit: no copy, delete, rename, drag,
  or quick look there (right-click shows the Windows menu, of which only "Open" takes
  effect in the app). It does not update live; changing folders, changing limits, or
  F5 rescan, and folders you expanded by hand collapse again. Sorting is always by
  name (folders first); search and the sort menu have no effect there. At the total
  limit the scan reads level by level, so the upper levels are complete. A folder with
  very many entries is still counted completely once (for "… and N more"), which can
  take a few seconds with millions of entries but is cancellable.
- **Tile view:** thumbnails come from Windows at 256 px. Folders show the drawn icon,
  not a content preview.
- **Column view (like the Finder):** every column is its own folder with its own
  selection; copy, cut, paste, delete, rename, new folder, context menu, dragging, and
  quick look act on the active column (the one with the strong highlight). Sorting and
  hidden files apply to all columns, search only to the first. The preview on the right
  shows icon or thumbnail, kind, size, and date; cloud files get no thumbnail. The
  breadcrumb shows the folder of the first column, not the deepest one. The column
  width is fixed. All open columns are watched live (at most 64 folders in total across
  all tabs).
- **Grouping** (sort menu, "Group by") exists in list and tiles: by name (first
  letter), type, date modified (Today, Yesterday, This week … like the Explorer), and
  size (Empty to Gigantic, folders separate), ascending or descending. Clicking a group
  header selects the whole group (Ctrl: in addition). Groups cannot be collapsed, and
  there is no grouping in column and tree view. Grouping is per tab and not saved. Date
  groups refer to the moment of calculation (midnight changes them only on the next
  sort or load).
- **Dragging inside the app runs without a Windows drag:** while the pointer is inside
  the window the app drags by itself (target highlighted blue, a hint such as "Move to
  …/Copy to …" at the pointer, Esc cancels, Ctrl copies, Shift moves). A blocking
  Windows drag froze the UI so the target was not highlighted live. Only when the
  pointer leaves the window does the real Windows drag start for other programs. The
  app detects leaving through `mouseout` without a target; **not yet seen on Windows**
  (especially the transition at the window edge). There is no automatic scrolling while
  dragging.
- **Search in subfolders:** from the first character the view filters the folder
  immediately and, after a short pause, searches all subfolders in the background
  (names, case-insensitive). Hits show their subfolder and can be opened, copied,
  dragged, or deleted like files. Limits: at most 5,000 hits, 400,000 entries read,
  depth 32; after that "Search limited" appears in the status bar. Only metadata is
  read; links and junctions are never entered, **cloud-only folders (OneDrive) are
  skipped** (the status bar gives the number), and **on network paths only the folder
  itself is filtered**. There is no index: very large drives take correspondingly long
  (cancel by changing or clearing the search). With the context menu, for hits from
  different folders only those from the clicked hit's folder take part.
- **Type-ahead** jumps in list, tiles, and columns to the first entry whose displayed
  name starts with the typed letters (a 900 ms pause ends the input; a space only
  counts in the middle of the input).
- **After long idle time:** folders that have not been loaded for more than 45 seconds
  are re-read when you return to the window, and failed list batches are retried up to
  three times instead of showing "Empty". **The actual cause of the observed unloading
  after about ten minutes is not confirmed** (suspected: Windows/WebView2 holds the
  window back). WebView2 background throttling is therefore disabled; whether that is
  enough has to be shown by a real Windows run.
- **Rubber-band selection:** in list, tiles, and columns, dragging with the mouse on
  empty space selects everything the rectangle touches; Ctrl or Shift keep the earlier
  selection. At the top/bottom edge the area scrolls along (also in very long
  folders). In the list, rows count as soon as they are touched vertically, so the
  rectangle does not have to hit the row at its name. Dragging only starts on empty
  space; from a row, dragging the files starts as before.
- **Favorites / Quick access:** pinned folders appear in the sidebar under "Favorites"
  after the fixed entries and are stored in the settings (at most 40, valid paths
  only). These are the **app's favorites**, not the Windows Quick access: the app does
  not read its pinned and frequent folders, and "Pin to Quick access" in the menu only
  affects the app's favorites, not the Explorer. Pinning and unpinning only work
  through right-click menus (no toolbar button). In the sidebar, pinned folders can be
  reordered by dragging (a blue line shows the insertion point, Esc cancels; Alt+Up/Down
  on the focused entry as a keyboard alternative); the fixed entries stay on top. A
  deleted folder stays until you unpin it.
- **Settings** (four sections as in the design, Ctrl+,): *Appearance* (light / dark /
  automatic, show file extensions, hidden files, start view for new tabs, tile size),
  *Tree view* (depth, entries in total and per folder), *OneDrive* (show sync status,
  detected accounts), and *Network* (timeout 5–60 s). "Defaults" resets only the
  section that is open, never the server list. The appearance also sets the window
  theme (title bar, Mica); with "Automatic" the app follows Windows. Without
  extensions, files appear without their extension in the list; renaming still uses the
  full name. The timeout applies to folders on the network (default 10 s); the
  reachability probe (40 %), share lists (80 %), and tree levels (150 %) derive from
  it. The settings are stored as `settings.json` in the app's config folder.
- **Deliberately not taken from the design:** "Download files on demand" and
  "Automatically free up space" (the OneDrive client controls that; the app never loads
  content), "Connect network drives at sign-in" (Windows does that itself), and "Find
  shares automatically" (the app only shows shares of the servers you add, no network
  discovery).
- **The view is per tab** and not saved; new tabs start with the start view from the
  settings.
- **Tabs** remember path, history, view, sorting, and selection, but not the scroll
  position, and are not saved on exit.
- **OneDrive:**
  - Accounts come from the registry (`HKCU\Software\Microsoft\OneDrive\Accounts`) and
    the environment variables `OneDrive`, `OneDriveConsumer`, `OneDriveCommercial`,
    without duplicates. SharePoint folders synced as libraries outside the account
    folder are not detected.
  - The status appears in the type column (list) or as a corner mark (tiles), not in
    the tree view. "Syncing" also applies to normal files in the OneDrive folder that
    the client has not converted yet; hidden and system files (desktop.ini) have no
    status. If OneDrive is paused, an entry stays "Syncing …" for that long.
  - The two actions appear in the context menu; if OneDrive already has them in its
    own menu (labeled in German or English), the app does not insert them. With another
    Windows language they may appear twice.
  - For folders the attribute is set on the folder; whether OneDrive applies it to the
    existing content is up to the client. The status changes only after OneDrive has
    reacted.
  - The Cloud Files API (`cldapi.dll`) exists from Windows 10 1709.
- **Strictly validated paths:** folders whose name ends in a dot or space or is a
  reserved device name (for example created by WSL) cannot be opened. File names with
  invalid UTF-16 are shown with replacement characters.
- **Mica** exists only on Windows 11; Windows 10 gets a solid background. Because of
  the custom title bar, the Snap Layout flyout is missing when hovering "Maximize"
  (Win+Z and dragging to the edge work).
- **Tested so far:** core logic by unit tests (among others path validation, operation
  planning, sync state and OneDrive accounts, network error codes, server input and the
  timeout with cool-down, watcher, tree limits with 100,000 files in one folder, a link
  loop, search, settings), Windows code by cross-compilation checks on Linux and in CI
  on Windows, the interface in the browser against the demo backend (also with 100,000
  entries). A test of the finished app on real Windows is still outstanding.

## Roadmap

1. ✅ Project setup, window, theme, sidebar, listing and navigating folders
2. ✅ File operations, drag & drop, tabs, live updates
3. ✅ Icons, thumbnails, context menu, quick look, opening files
4. ✅ Tree view with limits, cancel, and export; plus tile view and settings
5. ✅ OneDrive integration
6. ✅ Network drives
7. ✅ Polish: measurements, tests, security review, installers

Added after phase 7: column view, context menu in its own style, grouping, rubber-band
selection, subfolder search, type-ahead, favorites, and the settings sections.
