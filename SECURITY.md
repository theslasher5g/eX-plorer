# Security Policy

eX-plorer works with your files, so security matters here and reports are welcome.

## Supported versions

Security fixes are provided for the **latest published version**
(see [Releases](https://github.com/theslasher5g/eX-plorer/releases)). Older versions
do not receive fixes; please update first.

| Version | Supported |
|---|---|
| latest 0.x | ✅ |
| older | ❌ |

## Reporting a vulnerability

**Please do not open a public issue for security problems.**

1. Open the **Security** tab of this repository and choose
   **"Report a vulnerability"** (private reporting through GitHub Security Advisories).
2. Describe what you found, how to reproduce it (Windows version, app version,
   steps), and what the impact could be.
3. You will normally get a first reply **within 7 days**. Once the problem is
   confirmed and fixed it is mentioned in the changelog (with credit if you like).

If private reporting is not available to you, open an issue **without details** that
says "security issue, please get in touch", and we will work out a way to talk.

## What counts as a vulnerability

For example:

- Access to files or paths the app does not intend to reach (path escapes via `..`,
  junctions, symlinks, alternate data streams, reserved names, long paths).
- Automatically running or opening files without an explicit user action.
- Breaking out of the UI: script execution through file names or previews (XSS),
  bypassing the content security policy, loading remote content.
- Fetching OneDrive cloud files ("hydration") or touching network paths when the
  app should only do so on an explicit action.
- Flaws in the permission checks of the Tauri commands (see `src-tauri/capabilities`).
- Crashes or hangs caused by crafted folders (denial of service) with a realistic trigger.

Out of scope: problems that require administrator rights or already-running malicious
code on the machine, and bugs in third-party software (7-Zip, Git …) whose menu
entries the app merely displays.

## How the app is hardened

The binding rules are in the [manual](docs/MANUAL.md#security). In short:

- **Minimal permissions.** There are no Tauri plugins for the file system or shell;
  the capabilities only allow window control and window events. Everything else goes
  through the app's own commands, which **validate every path in the backend**.
- **Strict CSP**, no remote content, no `eval`, file names never rendered as HTML.
- **Nothing is opened or executed automatically**; programs (exe, bat, ps1 …) only
  after explicit confirmation.
- **Metadata only when listing.** OneDrive placeholders are never read or downloaded;
  network paths have timeouts and no automatic deep search.
- **No administrator rights needed**, no credentials stored in the app (Windows asks itself).
- `unsafe` only in small Windows modules, `unwrap()`/`expect()` banned by a Clippy lint,
  and `cargo clippy -- -D warnings` plus `cargo audit` run in CI.

> The installers are **not code-signed**, so Windows SmartScreen may warn.
> Verify the SHA-256 sums in `SHA256SUMS.txt` attached to each release.

This is a hobby project provided without warranty; no independent security audit has been carried out.
