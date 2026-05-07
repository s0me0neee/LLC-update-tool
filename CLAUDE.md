# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`llc-updater` is a CLI tool (`llc`) that automates downloading and installing Limbus Company zh-CN localization files from GitHub releases into the game's `LimbusCompany_Data/Lang/` directory. It fetches releases from GitHub via the Octocrab API, prompts for release/asset selection interactively (using `inquire`), downloads with progress bars, extracts `.zip`/`.7z` archives, and optionally installs font packages.

The project uses Rust edition 2024 and requires the **nightly** toolchain (per CI).

## Commands

```bash
cargo build
cargo test
TEST=1 cargo test                # run integration tests (uses ./test/ overrides)
cargo clippy --all-targets --all-features
cargo fmt

# Via just:
just run                         # clean test dirs then run with RUST_LOG=info
just test_integration            # TEST=1 cargo test
just prepare_test_dirs           # mkdir ./test/llc/cache/ ./test/llc/lang/ ./test/LimbusCompany_Data/Lang/
just clean                       # rm -rf test dirs
```

Enable verbose logging at runtime:
```bash
RUST_LOG=info llc
RUST_LOG=debug llc
```

## Architecture

### Module map

| Module | Responsibility |
|---|---|
| `main.rs` | Entry point, `Paths` struct, top-level async flow: prompt → download → extract → install → font |
| `cli.rs` | `clap`-derived `Args` (currently just `--list`) |
| `llc.rs` | GitHub release fetching (`octocrab`), asset download (`reqwest` + streaming), archive extraction (`.zip`/`.7z`) |
| `conf.rs` | Generic `Config` trait: `read()`/`write()` backed by a JSON file on disk |
| `setting.rs` | `Setting` (lock file state) and `Lock` (per-asset record with SHA-256 checksum); implements `Config` |
| `path.rs` | App data / Steam path resolution; `is_test_mode()` gate |
| `steam.rs` | Game directory discovery — VDF parsing on Linux/macOS; Windows registry lookup under `steam::windows` |
| `lang.rs` | Reads installed languages from `Lang/` directory and the active language from `Lang/config.json` |
| `fs.rs` | Post-extraction install step: moves inner `LimbusCompany_Data/Lang/` contents up and removes the redundant wrapper directory |

### Key data flow

1. `main` resolves `lbc_data_dir` (test override → VDF/registry) and builds a `Paths` struct.
2. `llc::select_release` fetches up to 5 latest GitHub releases; user picks one interactively.
3. `llc::download_asset` streams the file into `~/.local/share/llc/cache/`.
4. `llc::extract_asset` extracts `.zip` or `.7z` directly into `LimbusCompany_Data/Lang/`.
5. `fs::install_and_clean` detects when the archive contained a redundant `LimbusCompany_Data/Lang/` prefix and flattens it in-place.
6. `setting::Setting` (JSON at `~/.local/share/llc/lock.json`) records each downloaded asset's name, source URL, and SHA-256 checksum so future runs can skip redundant downloads.

### Test mode

`path::is_test_mode()` returns `true` when `TEST=1` (or `GITHUB_ACTIONS` is set). In test mode:
- App data resolves to `./test/llc/`
- Steam path resolves to `./test/Steam/`
- Game data dir resolves to `./test/LimbusCompany_Data/`

Run `just prepare_test_dirs` before integration tests.

The `LLC_CONFIG_FILE` env var overrides the lock file path (used in `lock_test` to isolate test state).

### Platform notes

- **Linux**: Steam path via `$XDG_DATA_HOME/Steam`; game dir via `libraryfolders.vdf` + `appmanifest_*.acf` parsing.
- **Windows**: Registry lookup under `HKCU\Software\Valve\Steam\Apps` and `HKLM\SOFTWARE\...Uninstall\Steam App <id>`. Windows-only dependency: `winreg`.
- **macOS**: Steam path discovery works but Limbus Company availability is limited; marked as limited support.

## Commit Messages

Write commit messages in English.
