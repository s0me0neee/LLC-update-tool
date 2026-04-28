# LLC Update Tool (Limbus Company)

CLI tool to download a GitHub release asset (for example from [LocalizeLimbusCompany](https://github.com/LocalizeLimbusCompany/LocalizeLimbusCompany)) and install the extracted `Lang/` content into the Limbus Company game directory.

## Warning

Font installation is optional but recommended for some languages. If the game shows missing glyphs, install the font when prompted, or install it manually:
[LLCCN-Font.7z](https://raw.githubusercontent.com/LocalizeLimbusCompany/LocalizeLimbusCompany/refs/heads/main/Fonts/LLCCN-Font.7z).

## Description

This project automates the common workflow of updating language/localization files for Limbus Company. It fetches releases from a GitHub repository, lets you pick a release and asset, downloads the archive with progress reporting, extracts it, and installs the extracted language content into the game’s `LimbusCompany_Data/Lang` folder. It also stores a small lock/config file (`lock.json`) that records what was downloaded and its checksum so future runs can skip redundant downloads.

## Getting Started

### Dependencies

- Rust toolchain (edition 2024)
- Network access to GitHub (GitHub API is rate-limited if unauthenticated)
- OS: Linux / Windows (supported), macOS (limited)
- Optional: `make` and/or `just` (helpers included)

### OS Support

- Linux: supported
- Windows: supported (registry-based Steam lookup is available)
- macOS: limited support. Steam path discovery works, but Limbus Company may not be available via Steam. Wine/Steam-on-Wine is untested, and the default Steam path search may not match your setup.

### Installing

- Clone the repository:

```bash
git clone <repo-url>
cd LLC-update-tool
```

- Build:

```bash
cargo build
```

- Optional: install as a local binary:

```bash
cargo install --path .
```

Notes:

- The current codebase contains test overrides for app data and (on non-Windows) game directory resolution. See the “Help” section for details.

### Executing program

- Run with Cargo:

```bash
cargo run
```

- Or using helpers:

```bash
make run
```

```bash
just run
```

- Enable more logging:

```bash
RUST_LOG=info cargo run
```

The program will prompt you for:

- GitHub repository URL
- Release selection
- Asset selection

## Help

Common issues and useful commands:

- GitHub rate limits:
  - If release fetching fails, retry later or run with authentication (not implemented by default).
- Unsupported archive:
  - Only `.zip` and `.7z` assets are supported.
- Test overrides (important):
  - When `TEST=1` (or on GitHub Actions), app data is overridden to `./test/llc` (see `src/path.rs`).
  - When `TEST=1` (or on GitHub Actions), the game data directory is set to `./test/LimbusCompany_Data` (see `src/main.rs`).
  - For production usage, remove/disable these overrides so OS/Steam discovery paths are used.
  - You can create the expected local test directories via `make prepare-test-dirs` or `just prepare_test_dirs`.

Tests:

```bash
cargo test
```

Integration tests:

```bash
TEST=1 cargo test
```

Notes:

- Test mode is enabled when `TEST` is truthy (`1/true/yes/...`) or when running on GitHub Actions (`GITHUB_ACTIONS` is set).

## Authors

Maintainers / contributors:

- See repository commit history.

## Version History

- 0.1.0
  - Initial release

## Roadmap

- Font installation support
- Multiple-language + version tracking (per language)
- Automatic extraction target (no hard-coded folder assumptions)
- Support for other Git sources (not only GitHub)
- Optional GUI

## License

This project is licensed under the MIT License. See [LICENSE](./LICENSE).

## Acknowledgments

Inspiration, templates, etc.:

- [awesome-readme](https://github.com/matiassingers/awesome-readme)
- [Localize Limbus Company](https://github.com/LocalizeLimbusCompany/LocalizeLimbusCompany/tree/main/LLC_zh-CN)
- [LLC Mod Toolbox](https://github.com/LocalizeLimbusCompany/LLC_MOD_Toolbox)
- [LLC Chinese font](https://raw.githubusercontent.com/LocalizeLimbusCompany/LocalizeLimbusCompany/refs/heads/main/Fonts/LLCCN-Font.7z)
