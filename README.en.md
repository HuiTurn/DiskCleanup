<p align="center">
  <img src="src/assets/brand/app-logo-v2.png" width="96" alt="DiskCleanup logo" />
</p>

<h1 align="center">DiskCleanup</h1>

<p align="center">Cross-platform (macOS / Windows) disk analysis and cleanup tool</p>

<p align="center">
  <a href="https://github.com/HuiTurn/DiskCleanup/actions/workflows/ci.yml"><img src="https://github.com/HuiTurn/DiskCleanup/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI" /></a>
  <a href="./LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT" /></a>
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows-lightgrey.svg" alt="Platforms: macOS | Windows" />
  <img src="https://img.shields.io/badge/Tauri-2-24C8DB.svg" alt="Tauri 2" />
</p>

<p align="center">
  English | <a href="./README.md">简体中文</a>
</p>

Scan disk usage, find large and duplicate files, and clean junk with per-platform rules. Every deletion goes to the system Trash / Recycle Bin and can be restored.

Built on **Tauri 2 + React 18 + TypeScript + Zustand**; a Rust engine handles scanning, cleanup, and scheduling.

## UI Preview

| [<img src="docs/screenshots/home.png" width="480" alt="Home: disk usage overview" />](docs/screenshots/home.png)<br>Home | [<img src="docs/screenshots/analyze.png" width="480" alt="Disk analysis: live progress and file stream" />](docs/screenshots/analyze.png)<br>Disk Analysis |
|---|---|

<details>
<summary><b>Expand the other 9 screens</b></summary>

| [<img src="docs/screenshots/results.png" width="480" alt="Analysis results" />](docs/screenshots/results.png)<br>Analysis Results | [<img src="docs/screenshots/junk.png" width="480" alt="Junk cleanup" />](docs/screenshots/junk.png)<br>Junk Cleanup |
|---|---|
| [<img src="docs/screenshots/large-files.png" width="480" alt="Large files" />](docs/screenshots/large-files.png)<br>Large Files | [<img src="docs/screenshots/duplicates.png" width="480" alt="Duplicate files" />](docs/screenshots/duplicates.png)<br>Duplicate Files |
| [<img src="docs/screenshots/file-migration.png" width="480" alt="File migration" />](docs/screenshots/file-migration.png)<br>File Migration | [<img src="docs/screenshots/launch-items.png" width="480" alt="Launch items" />](docs/screenshots/launch-items.png)<br>Launch Items |
| [<img src="docs/screenshots/system-tools.png" width="480" alt="System tools" />](docs/screenshots/system-tools.png)<br>System Tools | [<img src="docs/screenshots/done.png" width="480" alt="Cleanup done" />](docs/screenshots/done.png)<br>Cleanup Done |
| [<img src="docs/screenshots/settings.png" width="480" alt="Settings" />](docs/screenshots/settings.png)<br>Settings | |

</details>

> Screenshots are rendered automatically by CI in mock-data mode (`scripts/screenshot.mjs`); the latest set is available as an Actions artifact on every push. Run `node scripts/screenshot.mjs` locally to regenerate them into `docs/screenshots/`.

## Features

### Disk analysis

- **Home**: system drive usage overview, start-analysis entry, and 4 common feature shortcuts.
- **Scan engine**: a Rust background thread walks the disk and streams progress plus the file feed in real time; cancellable at any point.
- **Four result views**: Top folders / same-named directory aggregation / 6-bucket size histogram / scan-history trend with period-over-period deltas.
- Drill-down navigation and scan exclusion directories.

### Cleanup

- **Junk cleanup**: scans with per-platform rule sets (macOS caches / logs / dev caches…, Windows temp files / update caches…) and cleans the selected items in one click.
- **Large files**: threshold filtering from 10MB to 1GB, keyword search, batch cleanup.
- **Duplicate files**: size pre-filter → head hash → full hash. Similar files are scored by normalized file names (`photo.jpg` / `photo copy.jpg` / `photo (1).jpg`) + size within ±40% + 4KB fragment overlap, and shown separately from identical files (SHA-256).
- **Cleanup done**: space / file count / duration statistics with restore guidance.

### Files and system

- **File migration (Windows)**: redirect Desktop, Documents, Downloads, Pictures, Music, and Videos to another local disk while updating the system folder location.
- **Launch items**: reads macOS Login Items and the Windows Run registry.
- **System tools**: a per-platform tool grid; Windows-only entries are mapped to macOS equivalents.

### Automation and UX

- **Auto cleanup**: registers a launchd LaunchAgent on macOS (`~/Library/LaunchAgents/cn.aipalnet.diskcleanup.autoclean.plist`) and a scheduled task `DiskCleanupAutoClean` on Windows. Both run `diskcleanup --auto-clean` headlessly, cleaning only cache items marked as safe and recording `lastAutoCleanTs`. The Settings page offers "Run now".
- **Exit confirmation**: intercepts `CloseRequested` and closes only after an in-app confirmation (can be disabled in Settings).
- **Personalization**: launch at login, scan exclusions, large-file threshold, auto cleanup, dark theme.

### File migration (Windows)

The entry sits below "Large Files" in the sidebar. Select a system folder and a target drive → "Check and continue" → confirm the full paths, file count, and space usage → start the migration.

- Uses the Windows Known Folder Manager `Redirect` API to copy content, delete the copied source content, and update the system folder location; the target is `<target drive>\Users\<user folder name>\<system folder name>`.
- Only one folder is migrated at a time; it rejects same-volume operations, pre-existing target folders, nested system folders, OneDrive-managed folders, reparse points, offline placeholder files, EFS-encrypted files, and folders that cannot be fully read.
- Only local fixed disks are supported; the real folder location and free space are re-checked before starting, reserving at least 256MB or 5% extra capacity.
- Windows shows its own copy UI during migration and the app forbids normal exit. Failure, cancellation, or abnormal process exit may leave partial files; the app does not roll back automatically, so verify against the plan and result stored in `%LOCALAPPDATA%\diskcleanup\migrations`.
- macOS displays a platform note and does not perform folder redirection.

## Safety design

- All deletions go through the system Trash / Recycle Bin (the `trash` crate); files are never unlinked directly.
- Every cleanup writes `cleanup_log.json`. Windows / Linux can restore programmatically via `trash::os_limited::restore_all`; macOS is limited by the OS, so the app falls back to "open Trash + guide to Put Back".
- Auto cleanup only touches cache items matched by safety rules and never user documents.

## Download and installation

Download the latest build from [Releases](https://github.com/HuiTurn/DiskCleanup/releases):

- macOS (Apple Silicon / Intel): universal DMG — drag the app into Applications
- Windows x64: NSIS installer
- Checksums: `SHA256SUMS.txt`

All installers are unsigned; allow them manually on first launch:

- macOS: if you see "is damaged and can't be opened", run `xattr -cr /Applications/DiskCleanup.app` and reopen it.
- Windows: when SmartScreen appears, choose "More info → Run anyway".

You can also build from source (see below).

## Development

Requirements:

- Node.js 18+ (CI uses 20 / 22)
- Rust stable
- macOS: Xcode Command Line Tools
- Windows: MSVC Build Tools + WebView2 Runtime (bundled with Windows 11)

```bash
npm install
npm run tauri dev     # Dev mode (starts Vite + native window)
npm run tauri build   # Production build (macOS .app / Windows NSIS .exe)
```

Tests:

```bash
npm test                            # Frontend unit tests (node --test)
cd src-tauri && cargo test --lib    # Rust engine unit tests
```

Rust tests cover directory size statistics, large-file filtering and sorting, identical and similar file detection, junk rule statistics, settings persistence round-trip, and exclusion-directory enforcement. Frontend tests cover analysis aggregation, duplicate retention policy, and scan caching.

CI:

- `.github/workflows/ci.yml`: runs frontend tests and builds on Ubuntu / Windows / macOS, runs Rust unit tests on all three platforms, and refreshes screenshots in mock mode.
- `.github/workflows/validation.yml`: runs both test suites, the frontend build, and `cargo check` on macOS / Windows.

## Project status and known limitations

- Version `0.1.0`; UI and data structures may still change. Unsigned installers are available.
- The Windows file-migration engine has passed target-platform Rust type checking, but development happens on macOS, so real Windows Shell migration still needs acceptance testing on a Windows machine.
- Browser-based interaction verification uses mocked interfaces and does not move real files.
- macOS does not support programmatic restore; the app only provides Trash guidance.
- macOS / Windows are supported; Linux is not adapted yet.

## Project structure

```
src/                       # Frontend (platform-agnostic)
  pages/                   # 11 pages
  components/              # Sidebar / charts / common UI / icons
  stores/appStore.ts       # Zustand global state
  services/bridge.ts       # The single entry point for backend IPC
  services/mock.ts         # Mock implementation for browser and CI screenshots
  styles/tokens.css        # Design tokens (including dark-theme variables)
src-tauri/src/
  engine/                  # Business logic (platform-agnostic)
    scanner.rs             # Scan engine (background thread + event push + cancellation)
    cleaner.rs             # Cleanup (uniformly into the recycle bin + cleanup log)
    junk.rs / duplicates.rs / large_files.rs / history.rs
    autoclean.rs           # Auto-cleanup scheduling
    safety.rs              # Path validation and protection rules
    task.rs                # Background tasks and events
    settings.rs            # Settings persistence
    migration/             # Windows system-folder migration (plan / validate / execute)
    platform/              # ★ Platform adaptation layer
      macos.rs / windows.rs
  commands.rs              # Tauri IPC commands
```

## Platform adaptation strategy

- The frontend depends only on `bridge.ts` and the abstract functions exposed by `platform`; adding Linux support only requires adding `linux.rs` under `platform/` plus one cfg branch in each dispatch function.
- Keyboard shortcuts follow system conventions (macOS Cmd / Windows Ctrl, handled by the system webview).
- Font stacks fall back per platform: SF Pro / PingFang SC (macOS), Segoe UI / Microsoft YaHei (Windows).
- Windows-only concepts (registry, defragmentation, Windows Update cleanup) are mapped to equivalent features on macOS (log / cache cleanup, disk tools, system update cleanup), keeping page count and feature parity across both platforms.

## Contributing

Issues and PRs are welcome:

- Bug reports and feature requests: [Issues](https://github.com/HuiTurn/DiskCleanup/issues)
- Before submitting, make sure `npm test`, `npm run build`, and `cd src-tauri && cargo test --lib` all pass.
- Include screenshots for UI changes; regenerate them with `node scripts/screenshot.mjs`.
- The Chinese README is the source of truth; keep the English version in sync.

## License

[MIT](./LICENSE) © 2026 huiturn

## Acknowledgements

[Tauri](https://tauri.app/) · [React](https://react.dev/) · [Zustand](https://zustand-demo.pmnd.rs/), plus the Rust ecosystem crates `trash`, `walkdir`, `sha2`, `sysinfo`, and `dirs`.
