/// Application state, async event channels, and task spawning logic.
///
/// This module ties together the UI state, database, and scraper behind a
/// single `App` struct. Background work (network requests) is dispatched via
/// `tokio::spawn` and communicates results back through an unbounded MPSC
/// channel, keeping the main TUI loop non-blocking.
use std::sync::Arc;

use tokio::sync::mpsc;

use crate::db::Database;
use crate::error::SageError;
use crate::models::{Chapter, Novel};
use crate::reader_settings::ReaderSettings;
use crate::scraper::{NovelBuddy, NovelProvider};

// ──────────────────────────── Event types ──────────────────────────────

/// Asynchronous events sent from background tasks back to the main TUI loop.
#[derive(Debug)]
pub enum AppEvent {
    /// A periodic tick for UI refresh (cursor blink, animations, etc.).
    Tick,
    /// A request to search for a query.
    SearchQuery(String),
    /// A search completed successfully with a list of matching novels.
    SearchResults(Vec<Novel>),
    /// The chapter listing for a novel has been fetched.
    ChaptersFetched(Vec<Chapter>),
    /// A single chapter's content was downloaded.
    /// Contains `(chapter_id, content)`.
    ChapterDownloaded(String, String),
    /// Background download progress update.
    /// Contains `(novel_id, current_downloaded, total_chapters)`.
    DownloadProgress(String, usize, usize),
    /// EPUB Export completed with a message.
    ExportCompleted(String),
    /// An error occurred in a background task.
    Error(String),
}

#[derive(Debug, Default)]
pub enum SearchState {
    #[default]
    Idle,
    Searching,
    Success(Vec<Novel>),
    NoResults,
    Failure(String),
}

// ────────────────────────────── UI state ───────────────────────────────

/// Which top-level pane the user is currently viewing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivePane {
    /// The user's personal library of saved novels.
    Library,
    /// The search results and input view.
    Search,
    /// The chapter list view (bottom left).
    ChapterList,
    /// The chapter reader view.
    Reading,
    /// The downloads / progress view.
    Downloads,
    StorageManager,
    Prompt(String),
}

// ────────────────────────── Application core ──────────────────────────

/// Central application state.
///
/// Owns the database connection, the scraper provider, and all mutable UI
/// state. Spawns `tokio` tasks for network-bound operations and routes their
/// results back through `event_tx`.

#[derive(Debug, Clone)]
pub struct StorageItem {
    pub novel_id: String,
    pub title: String,
    pub downloaded_chapters: usize,
    pub size_bytes: usize,
}

pub struct App {
    // ── UI state ────────────────────────────────────────────────────
    /// The currently active pane.
    pub current_pane: ActivePane,
    /// The current search query typed by the user.
    pub search_query: String,
    /// Whether the search input is focused (if false, the results list is focused).
    pub search_input_focused: bool,
    /// Index of the currently highlighted novel in a list.
    pub selected_novel: usize,
    /// Index of the currently highlighted novel in the library.
    pub selected_library_novel: usize,
    /// Index of the currently highlighted chapter in a list.
    pub selected_chapter: usize,
    /// Vertical scroll offset in the reading pane.
    pub scroll_offset: usize,
    /// Whether the application should exit on the next loop iteration.
    pub should_quit: bool,
    /// The current theme index
    pub theme_index: usize,
    /// Settings for the reader pane
    pub reader_settings: ReaderSettings,
    pub config: crate::config::AppConfig,
    pub storage_items: Vec<StorageItem>,
    pub storage_selected: usize,
    /// Whether the reader settings panel is open
    pub show_settings_panel: bool,

    // ── Search / chapter results ────────────────────────────────────
    /// The explicit state of the current search operation.
    pub search_state: SearchState,
    /// Saved novels from the local library.
    pub library_novels: Vec<Novel>,
    /// Chapters for the currently selected novel.
    pub chapters: Vec<Chapter>,
    /// The novel currently being viewed / read.
    pub current_novel: Option<Novel>,
    /// The chapter content currently being read.
    pub current_chapter_content: Option<String>,

    // ── Status line ─────────────────────────────────────────────────
    /// A transient status message displayed in the UI footer.
    pub status_message: String,
    /// Whether a background operation is in-flight.
    pub is_loading: bool,
    /// Download progress state: Novel ID -> (Downloaded, Total)
    pub downloads_progress: std::collections::HashMap<String, (usize, usize)>,

    // ── Core components ─────────────────────────────────────────────
    /// Persistence layer.
    db: Database,
    /// Web scraper, wrapped in `Arc` so it can be shared with spawned tasks.
    provider: Arc<NovelBuddy>,
    /// Sender half of the event channel; cloned into each spawned task.
    event_tx: mpsc::UnboundedSender<AppEvent>,
    /// Sender for queueing downloads in the background manager.
    pub download_tx: mpsc::UnboundedSender<crate::downloader::DownloadCommand>,
}

impl App {
    /// Create a new `App`, initialising the database and scraper.
    ///
    /// Returns the `App` together with the receiving half of the event
    /// channel, which the caller (main loop) should poll.
    pub fn new() -> Result<(Self, mpsc::UnboundedReceiver<AppEvent>), SageError> {
        let db = Database::new()?;
        db.init_schema()?;

        let provider = Arc::new(NovelBuddy::new());
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        let download_manager = crate::downloader::DownloadManager::start(event_tx.clone(), provider.clone());

        let mut app = Self {
            // UI state
            current_pane: ActivePane::Library,
            search_query: String::new(),
            search_input_focused: true,
            selected_novel: 0,
            selected_library_novel: 0,
            selected_chapter: 0,
            scroll_offset: 0,
            should_quit: false,
            theme_index: 0,
            reader_settings: ReaderSettings::default(),
            config: crate::config::AppConfig::load(),
            storage_items: Vec::new(),
            storage_selected: 0,
            show_settings_panel: false,

            // Results
            search_state: SearchState::Idle,
            library_novels: Vec::new(),
            chapters: Vec::new(),
            current_novel: None,
            current_chapter_content: None,

            // Status
            status_message: String::from("Welcome to Sage"),
            is_loading: false,
            downloads_progress: std::collections::HashMap::new(),

            // Core
            db,
            provider,
            event_tx,
            download_tx: download_manager.cmd_tx,
        };

        app.load_library();
        Ok((app, event_rx))
    }

    pub fn load_library(&mut self) {
        match self.db.get_all_novels() {
            Ok(novels) => {
                self.library_novels = novels;
            }
            Err(e) => {
                self.status_message = format!("Failed to load library: {}", e);
            }
        }
        if let Ok(items) = self.db.get_storage_items() {
            self.storage_items = items;
        }
    }

    // ────────────────── Channel / event helpers ─────────────────────

    pub fn search_results(&self) -> &[Novel] {
        if let SearchState::Success(ref novels) = self.search_state {
            novels
        } else {
            &[]
        }
    }

    /// Return a clone of the event sender (useful for the tick timer).
    pub fn event_sender(&self) -> mpsc::UnboundedSender<AppEvent> {
        self.event_tx.clone()
    }

    // ────────────────── Task-spawning functions ─────────────────────

    /// Spawn a background search task.
    ///
    /// The scraper runs on a separate Tokio task and sends back
    /// `AppEvent::SearchCompleted` (or `AppEvent::Error`) through the
    /// channel, so the main loop never blocks.
    pub fn trigger_search(&mut self, query: String) {
        self.search_state = SearchState::Searching;
        self.is_loading = true;
        self.status_message = format!("Searching for \"{query}\"…");

        let provider = Arc::clone(&self.provider);
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            let event = match provider.search(&query).await {
                Ok(novels) => AppEvent::SearchResults(novels),
                Err(e) => AppEvent::Error(format!("Search failed: {e}")),
            };
            // If the receiver is dropped the app is shutting down — ignore the error.
            let _ = tx.send(event);
        });
    }

    /// Spawn a background task to fetch chapters for a novel.
    pub fn trigger_fetch_chapters(&mut self, novel_url: String) {
        self.is_loading = true;

        // Load from DB first — if we already have chapters, use them and skip
        // the network request entirely. This is essential for offline reading.
        if let Some(novel) = &self.current_novel
            && let Ok(db_chapters) = self.db().get_novel_chapters(&novel.id)
                && !db_chapters.is_empty() {
                    self.chapters = db_chapters;
                    self.is_loading = false;
                    self.status_message = format!("Loaded {} chapter(s) from library.", self.chapters.len());

                    // Restore reading progress for this novel.
                    let novel_id = novel.id.clone();
                    let _ = self.restore_progress(&novel_id);
                    return;
                }

        self.status_message = String::from("Fetching chapter list from web...");

        let provider = Arc::clone(&self.provider);
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            let event = match provider.fetch_chapters(&novel_url).await {
                Ok(chapters) => AppEvent::ChaptersFetched(chapters),
                Err(e) => AppEvent::Error(format!("Failed to fetch chapters: {e}")),
            };
            let _ = tx.send(event);
        });
    }
    pub fn load_selected_chapter(&mut self) {
        if let Some(chapter) = self.chapters.get(self.selected_chapter).cloned() {
            // First check if DB has the content
            if let Ok(db_chapters) = self.db().get_novel_chapters(&chapter.novel_id)
                && let Some(db_chap) = db_chapters.into_iter().find(|c| c.id == chapter.id)
                    && let Some(content) = db_chap.content {
                        self.current_chapter_content = Some(content);
                        self.scroll_offset = 0;
                        return; // Found locally!
                    }

            // Fallback to in-memory content or trigger web fetch
            if let Some(content) = chapter.content {
                self.current_chapter_content = Some(content);
                self.scroll_offset = 0;
            } else {
                self.current_chapter_content = None;
                self.trigger_download_chapter(chapter.id, chapter.url);
            }
        }
    }

    pub fn trigger_download_chapter(&mut self, chapter_id: String, chapter_url: String) {
        self.is_loading = true;
        self.status_message = "Downloading chapter…".to_string();

        let provider = Arc::clone(&self.provider);
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            let event = match provider.fetch_chapter_content(&chapter_url).await {
                Ok(content) => AppEvent::ChapterDownloaded(chapter_id, content),
                Err(e) => AppEvent::Error(format!("Download failed: {e}")),
            };
            let _ = tx.send(event);
        });
    }

    // ──────────────────── Event handling ────────────────────────────

    /// Process an incoming `AppEvent`, mutating the application state
    /// accordingly.
    ///
    /// This is intended to be called from the main TUI loop each time
    /// an event is received from the channel.
    pub fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Tick => {
                // Nothing to do — the UI will re-render on the next frame.
            }

            AppEvent::SearchQuery(query) => {
                self.search_query = query.clone();
                self.trigger_search(query);
            }

            AppEvent::SearchResults(novels) => {
                self.is_loading = false;
                if novels.is_empty() {
                    self.search_state = SearchState::NoResults;
                    self.status_message = String::from("No novels found.");
                } else {
                    self.status_message = format!("Found {} novel(s)", novels.len());
                    self.search_state = SearchState::Success(novels);
                    self.selected_novel = 0;
                }
            }

            AppEvent::ChaptersFetched(mut chapters) => {
                self.is_loading = false;
                self.status_message = format!("Loaded {} chapter(s)", chapters.len());

                // Merge with existing DB data to preserve downloaded content.
                // Web-fetched chapters arrive with content=None and is_downloaded=false,
                // so we must not blindly overwrite the DB rows.
                if let Some(novel) = &self.current_novel {
                    if let Ok(db_chapters) = self.db.get_novel_chapters(&novel.id) {
                        let db_map: std::collections::HashMap<String, _> = db_chapters
                            .into_iter()
                            .map(|c| (c.id.clone(), c))
                            .collect();
                        for ch in chapters.iter_mut() {
                            if let Some(db_ch) = db_map.get(&ch.id) {
                                // Preserve downloaded content from the database.
                                if db_ch.is_downloaded {
                                    ch.content = db_ch.content.clone();
                                    ch.is_downloaded = true;
                                }
                            }
                        }
                    }
                }

                // Persist chapters to the database (now with preserved content).
                for chapter in &chapters {
                    if let Err(e) = self.db.upsert_chapter(chapter) {
                        self.status_message = format!("DB error: {e}");
                    }
                }

                self.chapters = chapters;
                self.selected_chapter = 0;

                if let Some(novel_id) = self.current_novel.as_ref().map(|n| n.id.clone()) {
                    let _ = self.restore_progress(&novel_id);
                }
            }

            AppEvent::ChapterDownloaded(chapter_id, content) => {
                self.is_loading = false;
                self.status_message = String::from("Chapter downloaded");

                // Update the chapter record in the database.
                if let Some(ch) = self.chapters.iter_mut().find(|c| c.id == chapter_id) {
                    ch.content = Some(content.clone());
                    ch.is_downloaded = true;
                    if let Err(e) = self.db.upsert_chapter(ch) {
                        self.status_message = format!("DB error: {e}");
                    }
                }

                self.current_chapter_content = Some(content);
                self.scroll_offset = 0;
            }

            AppEvent::DownloadProgress(novel_id, current, total) => {
                self.downloads_progress.insert(novel_id.clone(), (current, total));
                // Look up the human-readable title from the library (fall back to novel_id).
                let title = self
                    .library_novels
                    .iter()
                    .find(|n| n.id == novel_id)
                    .map(|n| n.title.as_str())
                    .unwrap_or(&novel_id)
                    .to_owned();
                self.status_message =
                    format!("Downloading '{}': {}/{} chapters", title, current, total);
                self.is_loading = current != total;
            }

            AppEvent::ExportCompleted(msg) => {
                self.is_loading = false;
                self.status_message = msg;
            }

            AppEvent::Error(msg) => {
                self.is_loading = false;
                if matches!(self.search_state, SearchState::Searching) {
                    self.search_state = SearchState::Failure(msg.clone());
                }
                self.status_message = msg;
            }
        }
    }

    // ──────────────── Persistence convenience methods ───────────────

    /// Save the currently selected novel to the local library.
    pub fn save_novel_to_library(&mut self, novel: &Novel) -> Result<(), SageError> {
        self.db.upsert_novel(novel)?;
        self.load_library();
        self.status_message = format!("Saved \"{}\" to library", novel.title);
        Ok(())
    }

    /// Persist the current reading progress for a novel.
    pub fn save_reading_progress(
        &mut self,
        novel_id: &str,
        chapter_id: &str,
    ) -> Result<(), SageError> {
        self.db
            .save_progress(novel_id, chapter_id, self.scroll_offset)?;
        Ok(())
    }

    /// Try saving the reading progress if we are in Reading mode.
    pub fn try_save_progress(&mut self) {
        if self.current_pane == ActivePane::Reading
            && let Some(novel) = &self.current_novel
                && let Some(chapter) = self.chapters.get(self.selected_chapter) {
                    let _ = self.save_reading_progress(&novel.id.clone(), &chapter.id.clone());
                }
    }

    /// Load chapters for a novel from the local database (offline).
    pub fn load_chapters_from_db(&mut self, novel_id: &str) -> Result<(), SageError> {
        self.chapters = self.db.get_novel_chapters(novel_id)?;
        self.selected_chapter = 0;
        Ok(())
    }

    /// Restore reading progress for a novel from the database.
    pub fn restore_progress(&mut self, novel_id: &str) -> Result<(), SageError> {
        if let Some(progress) = self.db.get_progress(novel_id)? {
            self.scroll_offset = progress.scroll_offset;

            // Try to select the chapter that was last read.
            if let Some(idx) = self.chapters.iter().position(|c| c.id == progress.chapter_id) {
                self.selected_chapter = idx;
            }
        }
        Ok(())
    }

    /// Provide read-only access to the database (for library listing, etc.).
    pub fn theme(&self) -> &crate::theme::Theme {
        &crate::theme::THEMES[self.theme_index]
    }

    pub fn next_theme(&mut self) {
        self.theme_index = (self.theme_index + 1) % crate::theme::THEMES.len();
    }

    pub fn db(&self) -> &Database {
        &self.db
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_search_completed_updates_state() {
        // Build a minimal App with an in-memory DB.
        let (mut app, _rx) = make_test_app();

        let novels = vec![
            Novel {
                id: "n1".into(),
                title: "Novel One".into(),
                author: "A".into(),
                cover_url: String::new(),
                source_url: String::new(),
                description: String::new(),
            },
            Novel {
                id: "n2".into(),
                title: "Novel Two".into(),
                author: "B".into(),
                cover_url: String::new(),
                source_url: String::new(),
                description: String::new(),
            },
        ];

        app.is_loading = true;
        app.handle_event(AppEvent::SearchResults(novels));

        assert!(!app.is_loading);
        assert_eq!(app.search_results().len(), 2);
        assert_eq!(app.search_results()[0].title, "Novel One");
        assert_eq!(app.search_results()[1].title, "Novel Two");
        assert_eq!(app.selected_novel, 0);
        assert!(app.status_message.contains("2"));
    }

    #[test]
    fn handle_error_clears_loading() {
        let (mut app, _rx) = make_test_app();
        app.is_loading = true;

        app.handle_event(AppEvent::Error("something broke".into()));

        assert!(!app.is_loading);
        assert_eq!(app.status_message, "something broke");
    }

    #[test]
    fn handle_tick_is_noop() {
        let (mut app, _rx) = make_test_app();
        let msg_before = app.status_message.clone();

        app.handle_event(AppEvent::Tick);

        assert_eq!(app.status_message, msg_before);
    }

    #[test]
    fn active_pane_defaults_to_library() {
        let (app, _rx) = make_test_app();
        assert_eq!(app.current_pane, ActivePane::Library);
    }

    /// Helper: build an `App` backed by an in-memory SQLite database
    /// so tests don't touch the filesystem.
    fn make_test_app() -> (App, mpsc::UnboundedReceiver<AppEvent>) {
        let conn =
            rusqlite::Connection::open_in_memory().expect("failed to open in-memory db");
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        let db = Database::from_connection(conn);
        db.init_schema().expect("failed to init schema");

        let provider = Arc::new(NovelBuddy::new());
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let (download_tx, _) = mpsc::unbounded_channel();
        let app = App {
            current_pane: ActivePane::Library,
            search_query: String::new(),
            search_input_focused: true,
            selected_novel: 0,
            selected_library_novel: 0,
            selected_chapter: 0,
            scroll_offset: 0,
            should_quit: false,
            theme_index: 0,
            reader_settings: crate::reader_settings::ReaderSettings::default(),
            config: crate::config::AppConfig::load(),
            storage_items: Vec::new(),
            storage_selected: 0,
            show_settings_panel: false,

            search_state: SearchState::Idle,
            library_novels: Vec::new(),
            chapters: Vec::new(),
            current_novel: None,
            current_chapter_content: None,

            status_message: String::from("Welcome to Sage"),
            is_loading: false,

            download_tx,
            downloads_progress: std::collections::HashMap::new(),

            db,
            provider,
            event_tx,
        };

        (app, event_rx)
    }
}
