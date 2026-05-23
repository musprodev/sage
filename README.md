<div align="center">

# 📖 Sage

**A blazingly fast, highly customizable terminal-based web novel reader.**

[![Release](https://img.shields.io/github/v/release/musprodev/sage?style=for-the-badge&color=success)](https://github.com/musprodev/sage/releases/latest)
[![Rust](https://img.shields.io/badge/rust-1.80%2B-blue.svg?style=for-the-badge)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-linux%20%7C%20macOS%20%7C%20windows-lightgrey?style=for-the-badge)](#)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=for-the-badge)](LICENSE)

<!-- Replace the link below with the actual URL to your recorded GIF of Sage in action -->
![Sage in Action](./assets/demo.gif)

</div>

Sage is a high-performance, terminal-based user interface (TUI) web novel reader written in Rust. It utilizes the powerful `ratatui` crate for rendering and features an advanced asynchronous scraping engine designed to bypass Cloudflare and directly fetch full chapter catalogs from sources like NovelBuddy.

## ✨ Features

- **Blazing Fast TUI:** Built with Rust and `ratatui`, offering an incredibly responsive, keyboard-driven interface inspired by `btop`.
- **Advanced Scraping:** Uses `primp` with Chrome V144 impersonation to seamlessly bypass Cloudflare and interact with hidden API endpoints, fetching entire novel chapter lists (1000+ chapters) instantly.
- **True Offline Reading:** Background download your favorite novels directly into your local database. Sage will intelligently load texts from your hard drive if available, allowing you to read seamlessly without an internet connection.
- **Local SQLite Library:** Persists your novel progress, bookmarks, and downloaded chapters locally.
- **Storage Manager:** An integrated `ncdu`-style storage manager lets you visualize disk usage per novel, set custom cross-platform export directories, and instantly clear downloaded caches to free up space.
- **Premium Reader Experience:** A highly customizable reader view that supports:
  - Dynamic margins and text-width constraints (Narrow, Medium, Wide, Full).
  - Customizable line and paragraph spacing (`Compact` mode removes blank lines and indents paragraphs).
  - Reader-specific color themes (Sepia, Paper, Soft Dark).
  - Text alignment toggles.
- **EPUB Export:** Seamlessly export fully downloaded novels into well-formatted EPUB files for your e-reader (saved to your custom Export Directory).

## 🚀 Installation

### Download Pre-compiled Binaries (Recommended)

Sage automatically builds and bundles standalone binaries for Windows, macOS, and Linux on every release!

1. Head over to the [GitHub Releases page](https://github.com/musprodev/sage/releases).
2. Download the compressed archive for your operating system:
   - **Linux:** `sage-linux-x86_64.tar.gz`
   - **macOS:** `sage-macos-x86_64.tar.gz` (or `aarch64` for Apple Silicon)
   - **Windows:** `sage-windows-x86_64.zip`
3. Extract the archive and place the `sage` executable in your system's `PATH`.

### Building from Source

If you prefer to compile from source, ensure you have the [Rust toolchain](https://rustup.rs/) installed.

Clone the repository and build the release binary:

```bash
git clone https://github.com/yourusername/sage.git
cd sage
cargo build --release
```

The optimized binary will be located at `target/release/sage`. You can move this to your `PATH`:

```bash
sudo mv target/release/sage /usr/local/bin/
```

*(Note: Pre-compiled binaries for Linux, macOS, and Windows will be available in the GitHub Releases page via automated CI pipelines in the future.)*

## Usage & Keybindings

Launch Sage by running:

```bash
sage
```

### Global Keys
- `Tab`: Switch focus between the Sidebar and Main Area
- `t`: Toggle global UI theme (Dark, Light, Btop)
- `q`: Quit the application

### Library View
- `j` / `k`: Navigate your saved novels
- `/`: Search the online directory for new novels
- `Enter`: Open the selected novel's chapter list
- `D`: Download all chapters for the selected novel
- `M`: Open the Storage Manager (view and manage disk space)
- `E`: Export the novel to an EPUB file (saved to your custom Export Directory)
- `Del`: Remove the novel from your library entirely

### Reading View
- `Enter`: Open the selected chapter
- `S` or `s`: Toggle the Advanced Reader Settings panel
- `j` / `k`: Scroll down / up line-by-line
- `d` / `u`: Scroll down / up by page
- `n` / `p`: Next / Previous chapter
- `b`: Bookmark the current chapter

### Storage Manager
- `j` / `k`: Navigate downloaded novels
- `Del`: Clear downloaded chapter contents to reclaim disk space
- `M` or `Esc`: Return to the Library view

## Advanced Reader Configuration

While reading a chapter, press `S` to open the settings panel. You can dynamically adjust the layout using the following keys:
- `w`: Cycle Text Width (Narrow 60, Medium 80, Wide 100, Full)
- `m`: Cycle Margins (Compact, Normal, Wide)
- `l`: Cycle Line Spacing (Single, Relaxed, Double)
- `p`: Cycle Paragraph Spacing (Compact, Normal, Relaxed)
- `c`: Cycle Color Scheme (Default, Sepia, Paper, Soft Dark)
- `a`: Cycle Text Alignment (Left, Center)

## Contributing & Feedback

Contributions, feature requests, and bug reports are highly welcome! 
If you have an idea for an improvement or found a bug, please [open an issue](https://github.com/musprodev/sage/issues) on GitHub. We'd love to hear your suggestions to make Sage even better!

If you'd like to add support for new novel providers or improve the UI, please submit a Pull Request.
 

## License

This project is licensed under the MIT License.
