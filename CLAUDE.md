# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
cargo build

# Run (with info logging and clean test dirs first)
just run        # or: make run

# Lint
cargo clippy --all-targets --all-features

# Format
cargo fmt

# Unit tests only (no network, no game directory required)
cargo test

# Integration tests (uses ./test/ directories instead of real paths)
TEST=1 cargo test

# Create required local test directories before integration tests
just prepare_test_dirs   # or: make prepare-test-dirs

# Run a specific test by name
cargo test <test_name>

# Verbose test output
cargo test --verbose
```

Logging is controlled via `RUST_LOG` (default: `warn`). Options: `error`, `warn`, `info`, `debug`, `trace`.

## Architecture

The binary (`llc`) is a single-binary async CLI (tokio) with no subcommands except `--list`. The main flow in `src/main.rs` is:

1. Parse CLI args (`src/cli.rs` — only a `--list` flag currently)
2. Load or create `Setting` (persisted as `lock.json` via `src/setting.rs`)
3. Prompt for GitHub repo URL → fetch releases via `octocrab` (`src/llc.rs`)
4. User picks a release and asset → download with progress bar → SHA-256 checksum against GitHub digest
5. Extract `.zip` or `.7z` archive into `LimbusCompany_Data/Lang/`
6. Flatten nested extraction structure if needed (`src/fs.rs:install_and_clean`)
7. Optionally download and install a font package into per-language `Font/` subdirectories

**Key module responsibilities:**

- `src/path.rs` — resolves app data dir and Steam path; switches to `./test/` paths in test mode
- `src/steam.rs` — finds the game install directory: Windows uses registry (`winreg`), Linux/macOS parses Steam's `libraryfolders.vdf` and per-app `.acf` manifests using `keyvalues_parser`
- `src/setting.rs` — `Setting` struct (wraps a `Vec<Lock>` and optional font `Lock`); each `Lock` records asset name, source URL, file path, and SHA-256 checksum; persisted to `lock.json`
- `src/conf.rs` — generic `Config` trait with `read`/`write` methods backed by `serde_json`; `Setting` implements it
- `src/llc.rs` — GitHub API calls, download with `reqwest` + `indicatif` progress, extract `.zip`/`.7z`
- `src/fs.rs` — post-extraction install step: if the archive contained a nested `LimbusCompany_Data/Lang/` wrapper, it is flattened into the real lang dir and the wrapper is removed
- `src/lang.rs` — scans the installed `Lang/` directory and reads `config.json` to identify the active language

**Test mode** (`src/path.rs:is_test_mode`): active when `TEST=1` (or any truthy value) or `GITHUB_ACTIONS` is set. Redirects app data to `./test/llc/` and game data to `./test/LimbusCompany_Data/`. Integration tests need these directories to exist — create them with `just prepare_test_dirs`.

The `LLC_CONFIG_FILE` environment variable overrides the `lock.json` path (used in `setting_test` to isolate test state).

## Commit Messages

Write commit messages in English (enforced by `.trae/rules/git-commit-message.md`).
