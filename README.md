# LLC Update Tool (Limbus Company)

CLI tool to download a GitHub release language asset (for example from [LocalizeLimbusCompany](https://github.com/LocalizeLimbusCompany/LocalizeLimbusCompany)) and install the content into the Limbus Company game directory.

## Warning

Font installation is optional but recommended for some languages. If the game shows missing glyphs, install the font when prompted, or install it manually:
[LLCCN-Font.7z](https://raw.githubusercontent.com/LocalizeLimbusCompany/LocalizeLimbusCompany/refs/heads/main/Fonts/LLCCN-Font.7z).

## Description

This project automates the common workflow of updating language/localization files for Limbus Company. It fetches releases from a GitHub repository, lets you pick a release and asset, downloads the archive with progress reporting, extracts it, and installs the extracted language content into the game’s `LimbusCompany_Data/Lang` folder. It also has a lock/config file (`lock.json`) that records what was downloaded and its checksum so future runs can skip redundant downloads.

## Getting Started

### Dependencies

- OS: Windows / Linux (supported), macOS (limited support)

#### Development Dependencies

- Rust toolchain (edition 2024)
- Optional: `make` and/or `just` (for development)

### OS Support

- Linux: supported
- Windows: supported (registry-based Steam lookup is available)
- macOS: limited support. Steam path discovery works, but Limbus Company may not be available via Steam. Wine/Steam-on-Wine is untested, and the default Steam path search may not match your setup.

### Installing

- Install with Cargo:

```bash
cargo install llc-updater
```

- Install from binaries
  The [release page](https://github.com/s0me0neee/LLC-update-tool/releases) includes precompiled binaries for Linux, macOS, and Windows.

- Install from source:

```bash
git clone https://github.com/s0me0neee/LLC-update-tool
cd LLC-update-tool
cargo build
cargo install --path .
```

Notes:

- The current codebase contains test overrides for app data and (on non-Windows) game directory resolution. See the “Help” section for details.

### Usage

- Run

```
llc
```

```
llc --help
```

- Or using Just or Make:

```bash
make run
```

```bash
just run
```

- Enable more logging:

```bash
RUST_LOG=info llc
```

## Help

Common issues and useful commands:

- GitHub rate limits:
  - If release fetching fails, retry later or run with authentication (not implemented by default).
- Unsupported archive:
  - Only `.zip` and `.7z` assets are supported.
- Test overrides (important):
  - When `TEST=1` (or on GitHub Actions), app data is overridden to `./test/llc` (see `src/path.rs`).
  - When `TEST=1` (or on GitHub Actions), the game data directory is set to `./test/LimbusCompany_Data` (see `src/main.rs`).
  - You can create the expected local test directories via targets `make prepare-test-dirs` or `just prepare_test_dirs`.
- Logs:
  - Use `RUST_LOG` to set log level, log level is default to `warn`
    - Options are:
      - `error`
      - `warn`
      - `info`
      - `debug`
      - `trace`

Notes:

- Test mode is enabled when `TEST` is truthy (`1/true/yes/...`) or when running on GitHub Actions (`GITHUB_ACTIONS` is set).

## Version History

- 0.1.0
  - Initial release

## Road Map

- Add more
- cli flags features
- Multiple-language + version tracking (per language)
- Automatic extraction target (no hard-coded folder assumptions)
- Support for other Git sources (not only GitHub)
- Optional GUI

## License

This project is licensed under the MIT License. See [LICENSE](./LICENSE).

## Acknowledgments

Inspiration, templates, etc.:

- [Localize Limbus Company](https://github.com/LocalizeLimbusCompany/LocalizeLimbusCompany/tree/main/LLC_zh-CN)
- [LLC Mod Toolbox](https://github.com/LocalizeLimbusCompany/LLC_MOD_Toolbox)
- [LLC Chinese font](https://raw.githubusercontent.com/LocalizeLimbusCompany/LocalizeLimbusCompany/refs/heads/main/Fonts/LLCCN-Font.7z)
