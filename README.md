<div align="center">

# 📖 Sage

**A blazingly fast, asynchronous terminal-based web novel reader built in Rust.**

Read, cache, and export web novels from NovelBuddy and NovelFire — entirely from your terminal.

[![Release](https://img.shields.io/github/v/release/musprodev/sage?style=for-the-badge&color=success)](https://github.com/musprodev/sage/releases/latest)
[![Rust](https://img.shields.io/badge/rust-1.86%2B-blue.svg?style=for-the-badge)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-linux%20%7C%20macOS%20%7C%20windows-lightgrey?style=for-the-badge)](#)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=for-the-badge)](LICENSE)

![Sage TUI web novel reader demo](./assets/demo.gif)

</div>

---

## What is Sage?

Sage is a **high-performance TUI (terminal user interface)** application for reading web novels. It connects to online novel platforms, fetches chapter catalogs, stores everything in a **local SQLite database**, and renders formatted text in a fully customizable terminal reader.

Built with [ratatui](https://github.com/ratatui/ratatui) and [crossterm](https://github.com/crossterm-rs/crossterm), Sage runs on **Linux, macOS, and Windows** without requiring a browser or GUI.

### Key Capabilities

| Feature | Description |
|---|---|
| **Cloudflare Bypass** | JA3 fingerprint impersonation via `primp` with Chrome V144 — requests appear as legitimate browser traffic |
| **Offline Reading** | Background-download entire novels into SQLite; read without internet |
| **Multi-Source** | Built-in providers for [NovelBuddy](https://novelbuddy.com) and [NovelFire](https://novelfire.net) via an extensible `NovelProvider` trait |
| **EPUB Export** | Export downloaded novels to `.epub` for Kindle, Kobo, or any e-reader |
| **Storage Manager** | `ncdu`-style interface to visualize per-novel disk usage and clear caches |
| **Premium Reader** | Adjustable text width, margins, spacing, color schemes (Sepia, Paper, Soft Dark), and alignment |
| **Themes** | Tokyo Night, Dracula, Catppuccin Mocha, Solarized Dark |

---

## Installation

### Download Prebuilt Binaries (Recommended)

Grab the latest release for your platform from the [Releases page](https://github.com/musprodev/sage/releases/latest):

| Platform | Download |
|---|---|
| Linux x86_64 | [`sage-linux-x86_64.tar.gz`](https://github.com/musprodev/sage/releases/latest/download/sage-linux-x86_64.tar.gz) |
| macOS x86_64 | [`sage-macos-x86_64.tar.gz`](https://github.com/musprodev/sage/releases/latest/download/sage-macos-x86_64.tar.gz) |
| macOS Apple Silicon | [`sage-macos-aarch64.tar.gz`](https://github.com/musprodev/sage/releases/latest/download/sage-macos-aarch64.tar.gz) |
| Windows x86_64 | [`sage-windows-x86_64.zip`](https://github.com/musprodev/sage/releases/latest/download/sage-windows-x86_64.zip) |

```bash
# Example: install on Linux
curl -LO https://github.com/musprodev/sage/releases/latest/download/sage-linux-x86_64.tar.gz
tar xzf sage-linux-x86_64.tar.gz
sudo mv sage /usr/local/bin/
```

### Build from Source on Fedora Linux

Install the required system packages and the Rust toolchain:

```bash
# Install system dependencies
sudo dnf install sqlite-devel openssl-devel gcc pkg-config perl-FindBin
```

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

```bash
# Clone, build, and install
git clone https://github.com/musprodev/sage.git
cd sage
cargo build --release
sudo cp target/release/sage /usr/local/bin/
```

### Build from Source (Generic)

Ensure you have the [Rust toolchain](https://rustup.rs/) installed (1.86+):

```bash
git clone https://github.com/musprodev/sage.git
cd sage
cargo build --release
```

The binary is output to `target/release/sage`. Move it to your `PATH`:

```bash
sudo mv target/release/sage /usr/local/bin/
```

---

## Quick Start

Launch Sage:

```bash
sage
```

Search for a novel with `/`, select it with `Enter` to add to your library, then press `Enter` again to load chapters. Press `Enter` on a chapter to read it.

Download an entire novel for offline reading with `D`, or export to EPUB with `E`.

---

## Architecture

Sage is structured as a single Rust binary crate with the following modules:

```
src/
├── main.rs              # Entry point, terminal setup, keyboard dispatch
├── app.rs               # Application state, event handling, task spawning
├── ui.rs                # TUI rendering (ratatui layouts, widgets, theming)
├── scraper.rs           # NovelProvider trait + NovelBuddy/NovelFire impls
├── db.rs                # SQLite persistence (rusqlite, WAL mode)
├── downloader.rs        # Background download manager (semaphore, rate-limit)
├── exporter.rs          # EPUB export via epub-builder
├── reader_settings.rs   # Reader customization enums and cycling logic
├── theme.rs             # 4 terminal color themes
├── models.rs            # Novel, Chapter, Progress domain structs
├── config.rs            # JSON config (~/.config/sage/)
└── error.rs             # SageError enum (thiserror)
```

### How Cloudflare Bypass Works

Sage uses the [`primp`](https://crates.io/crates/primp) HTTP client configured with `Impersonate::ChromeV144`. This generates TLS connections with a **JA3 fingerprint identical to Chrome 144**, causing Cloudflare's bot detection to classify requests as legitimate browser traffic. No headless browser is needed — requests operate at raw HTTP speed with persistent cookie storage.

```rust
let client = Client::builder()
    .impersonate(Impersonate::ChromeV144)
    .cookie_store(true)
    .timeout(std::time::Duration::from_secs(30))
    .build()
    .expect("failed to build primp client");
```

### SQLite Caching

All data is stored at `~/.local/share/sage/sage.db` with three tables:

- **`novels`** — metadata (title, author, cover, source URL, description)
- **`chapters`** — content + download status, foreign-keyed to novels with `ON DELETE CASCADE`
- **`progress`** — per-novel reading position (chapter ID + scroll offset)

WAL journal mode is enabled for concurrent read performance. Downloaded chapter text is stored directly in the `content` column, enabling fully offline reading.

---

## Keybindings

### Global

| Key | Action |
|---|---|
| `Tab` | Navigate between UI panes |
| `Esc` | Go back or exit current view |
| `t` | Cycle global UI theme (Tokyo Night → Dracula → Catppuccin → Solarized) |
| `Ctrl-C` | Force quit |
| `q` | Quit application |

### Library View

| Key | Action |
|---|---|
| `j` / `k` | Navigate novels |
| `/` | Search for novels online |
| `Enter` | Open selected novel's chapter list |
| `d` / `D` | Download all chapters for offline reading |
| `e` / `E` | Export novel to EPUB |
| `m` / `M` | Open Storage Manager |
| `Del` | Remove novel from library |

### Reading View

| Key | Action |
|---|---|
| `j` / `k` | Scroll line by line |
| `d` / `u` | Scroll page down / up |
| `g` / `G` | Jump to top / bottom |
| `p` / `n` | Next / previous chapter |
| `S` / `s` | Toggle reader settings panel |

### Reader Settings (press `S` or `s` while reading)

| Key | Setting | Values |
|---|---|---|
| `w` | Text Width | Narrow (60) → Medium (80) → Wide (100) → Full |
| `m` | Margins | Compact → Normal → Wide |
| `l` | Line Spacing | Single → Relaxed → Double |
| `p` | Paragraph Spacing | Compact → Normal → Relaxed |
| `c` | Color Scheme | Default → Sepia → Paper → Soft Dark |
| `a` | Alignment | Left → Center |

### Storage Manager

| Key | Action |
|---|---|
| `j` / `k` | Navigate downloaded novels |
| `Del` | Clear downloaded content to reclaim disk space |
| `c` / `C` | Configure custom export directory path |
| `m` / `M` / `Esc` | Return to Library |

---

## Supported Sources

| Source | Base URL | Catalog Fetch | Status |
|---|---|---|---|
| **NovelBuddy** | `novelbuddy.com` | REST API (`api.novelbuddy.com`) | ✅ Active |
| **NovelFire** | `novelfire.net` | HTML scraping (CSS selectors) | ✅ Active |

Additional providers can be added by implementing the [`NovelProvider`](src/scraper.rs) async trait:

```rust
#[async_trait]
pub trait NovelProvider: Send + Sync {
    fn source_id(&self) -> &'static str;
    fn base_url(&self) -> &'static str;
    async fn search(&self, query: &str) -> Result<Vec<Novel>, SageError>;
    async fn fetch_chapters(&self, novel_url: &str) -> Result<Vec<Chapter>, SageError>;
    async fn fetch_chapter_content(&self, chapter_url: &str) -> Result<String, SageError>;
}
```

---

## Contributing

Contributions, feature requests, and bug reports are welcome!

- [Open an issue](https://github.com/musprodev/sage/issues) for bugs or feature requests
- Submit a Pull Request to add novel providers, improve the UI, or fix bugs
- Run tests before submitting: `cargo test`

---

## License

This project is licensed under the [MIT License](LICENSE).
