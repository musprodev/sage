# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 1.0.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability in Sage, please report it responsibly.

**Do not open a public GitHub issue for security vulnerabilities.**

Instead, please email: **musprodev@users.noreply.github.com**

You should receive a response within 72 hours. We will work with you to understand the scope of the issue and develop a fix before any public disclosure.

## Scope

Sage is a client-side TUI application that makes HTTP requests to third-party novel hosting websites. Security concerns may include:

- **Credential exposure** — Sage does not handle user credentials, but cookie storage is used for session persistence with novel sources.
- **Local data integrity** — Chapter content and reading progress are stored in a local SQLite database (`~/.local/share/sage/sage.db`).
- **Network security** — All outbound requests use HTTPS via the `primp` HTTP client.
- **Dependency vulnerabilities** — Please report if you discover a vulnerability in any of Sage's Rust dependencies.
