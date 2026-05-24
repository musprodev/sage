%global crate_name sage

Name:           %{crate_name}
Version:        1.0.1
Release:        1%{?dist}
Summary:        A blazingly fast terminal-based web novel reader (TUI)

License:        MIT
URL:            https://github.com/musprodev/sage
Source0:        %{url}/archive/v%{version}/%{crate_name}-%{version}.tar.gz

BuildRequires:  rust >= 1.80.0
BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  openssl-devel
BuildRequires:  sqlite-devel
BuildRequires:  pkg-config
BuildRequires:  perl-FindBin

# Sage is a TUI application — it requires a terminal to run
Requires:       sqlite-libs
Requires:       openssl-libs

ExclusiveArch:  x86_64 aarch64

%description
Sage is a high-performance, asynchronous terminal user interface (TUI) web
novel reader written in Rust. It scrapes novel sources (NovelBuddy, NovelFire)
with JA3 Cloudflare bypass via Chrome V144 impersonation, caches chapter
content in a local SQLite database for offline reading, and provides a premium
customizable reader with EPUB export. Built with ratatui and crossterm.

Features:
- Cloudflare bypass using primp with JA3 fingerprint impersonation
- True offline reading with local SQLite caching
- Multi-source support (NovelBuddy, NovelFire) via extensible provider trait
- EPUB export for e-readers
- Integrated ncdu-style storage manager
- Customizable reader (text width, margins, spacing, color schemes, alignment)
- Vim-style keyboard navigation

%prep
%autosetup -n %{crate_name}-%{version}

%build
export CARGO_HOME="%{_builddir}/.cargo"
cargo build --release --locked

%check
export CARGO_HOME="%{_builddir}/.cargo"
cargo test --release --locked

%install
install -Dpm 0755 target/release/%{crate_name} %{buildroot}%{_bindir}/%{crate_name}

%files
%license LICENSE
%doc README.md
%{_bindir}/%{crate_name}

%changelog
* Sat May 24 2026 musprodev <musprodev@users.noreply.github.com> - 1.0.1-1
- Initial RPM package for Sage v1.0.1
- Terminal-based web novel reader with Cloudflare bypass
- SQLite offline caching and EPUB export support
