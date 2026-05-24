# Contributing to Sage

Thank you for your interest in contributing to Sage! We welcome contributions of all kinds — bug reports, feature requests, documentation improvements, and code changes.

## Getting Started

### Prerequisites

- **Rust toolchain** 1.80+ (nightly recommended for edition 2024 features)
- **System dependencies** (Fedora/RHEL): `sqlite-devel openssl-devel gcc pkg-config`
- **System dependencies** (Debian/Ubuntu): `libsqlite3-dev libssl-dev gcc pkg-config`
- **System dependencies** (macOS): `brew install sqlite openssl pkg-config`

### Setup

```bash
git clone https://github.com/musprodev/sage.git
cd sage
cargo build
cargo test
```

## How to Contribute

### Reporting Bugs

1. Check [existing issues](https://github.com/musprodev/sage/issues) to avoid duplicates.
2. Open a new issue with:
   - Steps to reproduce
   - Expected vs. actual behavior
   - Your OS and Rust version (`rustc --version`)

### Suggesting Features

Open an issue with the `enhancement` label describing:
- The problem you're trying to solve
- Your proposed solution
- Any alternatives you've considered

### Adding a Novel Provider

Sage uses an extensible `NovelProvider` trait. To add a new source:

1. Create a new struct in `src/scraper.rs`
2. Implement the `NovelProvider` trait:

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

3. Wire it into `App::new()` in `src/app.rs`
4. Add tests

### Submitting Pull Requests

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make your changes
4. Run the full test suite: `cargo test`
5. Run the linter: `cargo clippy`
6. Format your code: `cargo fmt`
7. Commit with a clear message
8. Push and open a Pull Request

## Code Style

- Follow standard Rust conventions (`cargo fmt`)
- Run `cargo clippy` and resolve all warnings
- Add doc comments for public APIs
- Write tests for new functionality

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
