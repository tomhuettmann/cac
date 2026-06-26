# Co-Author Committer

![Rust](https://img.shields.io/badge/Rust-2024-orange.svg)
![Platform](https://img.shields.io/badge/platform-macOS-lightgrey.svg)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A fast terminal UI tool to amend your latest git commit with co-authors. 

## Preview

```
┌ cac ─────────────────────────────────-────── v.X.Y.Z ┐
│ Amending: a1b2c3d feat: add user authentication      │
└──────────────────────────────────────────────────────┘
┌ Search contributors ─────────────────────────────────┐
│ ali                                                  │
└──────────────────────────────────────────────────────┘
┌ Contributors (3) ────────────────────────────────────┐
│ ✓ Alice Smith <alice@example.com>                    │
│   Alina Mueller <alina@example.com>                  │
│   Ali Hassan <ali.hassan@example.com>                │
└──────────────────────────────────────────────────────┘
 ↑↓ navigate   Tab toggle   Enter confirm   Esc cancel
```

## Workflow

```
commit → cac → push
```

## Features

- **Smart Contributor Discovery**: Scans your git history to find contributors automatically
- **Fuzzy Search**: Quickly filter contributors by name or email
- **Multi-select**: Add multiple co-authors at once
- **Amend Mode**: Modifies your latest commit in-place — no extra commits
- **Fast**: Built with Rust, uses libgit2 natively

## Installation

### Via Homebrew

```bash
brew tap tomhuettmann/tap
brew install tomhuettmann/tap/cac
```

### From Release

Download the latest release from [GitHub Releases](https://github.com/tomhuettmann/cac/releases).

### From Source

```bash
cargo install --path .
```

## Usage

Run in any git repository after making a commit:

```bash
# Use current directory
cac

# Specify a repository path
cac -d /path/to/repo
```

### Controls

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate contributor list |
| `Tab` | Toggle co-author selection |
| `Enter` | Confirm and amend commit |
| `Esc` | Cancel (no changes) |
| Type | Fuzzy search contributors |

## Requirements

- macOS arm64 (Apple Silicon)
- Git installed and configured
- A git repository with at least one commit

## How It Works

1. Reads your latest commit
2. Scans your entire git history for unique contributors (excluding yourself and noreply emails)
3. Presents an interactive fuzzy-searchable list
4. Amends the commit with selected `Co-authored-by:` trailers

```
Your commit message

Co-authored-by: Alice Smith <alice@example.com>
Co-authored-by: Bob Jones <bob@example.com>
```

## Development

```bash
cargo run
cargo test
cargo build --release
```

## License

MIT — see [LICENSE](LICENSE).

## Author

Tom Hüttmann — [@tomhuettmann](https://github.com/tomhuettmann)
