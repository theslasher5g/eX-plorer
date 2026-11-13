# Contributing

Thanks for wanting to improve eX-plorer! Bug reports, ideas, and pull requests are welcome.

## Reporting bugs and suggesting ideas

- **Bugs:** use the "Bug report" template. Helpful details: Windows version, app version
  (it is in the installer's file name), steps to reproduce, and a screenshot.
- **Ideas:** use the "Feature request" template. Describe the problem, not just the solution.
- **Security problems:** do not report them publicly, see [SECURITY.md](SECURITY.md).

## Development setup

Requirements and commands are in the [README](README.md#build-from-source). In short:

```bash
npm install
npm run tauri dev      # app with hot reload (Windows)
npm run dev            # just the UI in a browser with demo data (any OS)
```

The core (`src-tauri/crates/core`) is platform-neutral and can also be tested on
Linux/macOS: `cargo test -p explorer-core`.

## Before opening a pull request

```bash
npm run check                                    # svelte-check
cd src-tauri
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo audit
```

The CI runs the same on `windows-latest`.

## Code rules

These points are **binding** (see also [Security in the manual](docs/MANUAL.md#security)):

- The frontend **never** touches the file system directly; new features are Tauri
  commands that validate **every path** with `validate_path`.
- No Tauri plugins for the file system/shell; keep capabilities as small as possible.
- No `{@html}`/`innerHTML` with file names or file contents.
- Never open or execute anything without an explicit user action.
- When listing, read metadata only; never open cloud placeholders or network paths automatically.
- `unsafe` only in the small Windows API modules (`src-tauri/src/shell`,
  `crates/core/src/listing/win.rs`), with a `// SAFETY:` comment.
- No `unwrap()`/`expect()` in production code; errors use `thiserror`.
- UI texts belong in `src/lib/i18n/de.ts`, not in components.
- New API methods go into `types.ts`, `tauri.ts` **and** `mock.ts` (demo mode).

## Commits and pull requests

- Small, focused commits with a meaningful message.
- Describe in the PR what changes and how you checked it. Please say explicitly when a
  change can only be verified on Windows (shell, drag & drop, OneDrive, network).
- New features come with tests (core: Rust tests; UI: describe how the behavior can be
  reproduced in demo mode).

## Code of conduct

The [Code of Conduct](CODE_OF_CONDUCT.md) applies.
