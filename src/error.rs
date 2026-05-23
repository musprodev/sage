/// Centralized error types for the Sage application.
use thiserror::Error;

/// The top-level error type for all Sage operations.
#[derive(Debug, Error)]
pub enum SageError {
    /// An HTTP request failed (network issues, timeouts, bad status codes).
    #[error("HTTP request failed: {0}")]
    Request(#[from] primp::Error),

    /// A database operation failed.
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// A filesystem I/O operation failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A required HTML element was not found during scraping.
    #[error("Element not found: {selector}")]
    ElementNotFound {
        /// The CSS selector that failed to match.
        selector: String,
    },

    /// The target page is protected by Cloudflare's anti-bot challenge.
    #[error("Cloudflare protection detected on: {url}")]
    CloudflareBlocked {
        /// The URL that returned a Cloudflare challenge page.
        url: String,
    },

    /// A catch-all for other scraping failures (malformed HTML, missing attributes, etc.).
    #[error("Scraping error: {0}")]
    ScrapingError(String),
}

/// Convenience type alias used throughout the application.
pub type Result<T> = std::result::Result<T, SageError>;
