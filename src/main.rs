#![allow(dead_code, unused_variables, unused_imports, unreachable_patterns)]
mod app;
pub mod config;
mod db;
mod downloader;
mod error;
mod exporter;
mod models;
mod reader_settings;
mod scraper;
pub mod theme;
mod ui;

use std::io;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::sync::mpsc;

use app::{ActivePane, App, AppEvent};

/// Entry point — sets up the terminal, runs the async event loop,
/// and ensures clean shutdown regardless of how the app exits.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── Terminal setup ──────────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // ── Application + event channel ─────────────────────────────────
    let (mut app, mut event_rx) = App::new()?;

    // ── Tick timer — fires AppEvent::Tick every 250ms ───────────────
    let tick_tx = app.event_sender();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(250));
        loop {
            interval.tick().await;
            if tick_tx.send(AppEvent::Tick).is_err() {
                // Receiver dropped — app is shutting down.
                break;
            }
        }
    });

    // ── Main event loop ─────────────────────────────────────────────
    let result = run_loop(&mut terminal, &mut app, &mut event_rx).await;

    // ── Terminal teardown (always runs) ─────────────────────────────
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Sage exited with error: {e}");
    }

    Ok(())
}

/// The core select!-based event loop.
///
/// One branch handles keyboard / mouse events from crossterm's async
/// `EventStream`, the other handles `AppEvent`s arriving from
/// background scraper tasks and the tick timer.
async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    event_rx: &mut mpsc::UnboundedReceiver<AppEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut crossterm_events = event::EventStream::new();

    loop {
        // ── Render ──────────────────────────────────────────────────
        terminal.draw(|f| ui::draw(f, app))?;

        // ── Wait for the next event ─────────────────────────────────
        tokio::select! {
            // Branch 1: terminal / keyboard events.
            maybe_event = crossterm_events.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                        handle_key(app, key.code, key.modifiers);
                    }
                    Some(Err(e)) => {
                        app.status_message = format!("Input error: {e}");
                    }
                    // None → stream ended (terminal closed).
                    None => break,
                    // Ignore mouse, resize, etc. for now.
                    _ => {}
                }
            }

            // Branch 2: application events from background tasks.
            Some(app_event) = event_rx.recv() => {
                app.handle_event(app_event);
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

// ──────────────────────── Keyboard handling ──────────────────────────

/// Map a key press to an application action.
fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // ── Global keys (work in any pane) ──────────────────────────────
    match code {
        // Ctrl-C always quits.
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
            return;
        }
        KeyCode::Char('t')
            if !(app.current_pane == ActivePane::Search && app.search_input_focused) =>
        {
            app.next_theme();
            return;
        }
        _ => {}
    }

    // ── Pane-specific keys ──────────────────────────────────────────
    match app.current_pane {
        ActivePane::Search => handle_search_keys(app, code),
        ActivePane::Library => handle_library_keys(app, code),
        ActivePane::ChapterList => handle_chapterlist_keys(app, code),
        ActivePane::Reading => handle_reading_keys(app, code),
        ActivePane::Downloads => handle_library_keys(app, code),
        ActivePane::StorageManager => handle_storage_keys(app, code),
        ActivePane::Prompt(_) => handle_prompt_keys(app, code),
    }
}

fn handle_search_keys(app: &mut App, code: KeyCode) {
    if app.search_input_focused {
        match code {
            KeyCode::Esc => {
                app.current_pane = ActivePane::Library;
            }
            KeyCode::Tab => {
                app.search_input_focused = false;
            }
            KeyCode::Enter
                if !app.search_query.is_empty() => {
                    let query = app.search_query.clone();
                    let _ = app
                        .event_sender()
                        .send(crate::app::AppEvent::SearchQuery(query));
                }
            KeyCode::Backspace => {
                app.search_query.pop();
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
            }
            _ => {}
        }
    } else {
        match code {
            KeyCode::Esc => {
                app.current_pane = ActivePane::Library;
            }
            KeyCode::Tab => {
                app.search_input_focused = true;
            }
            KeyCode::Char('j') | KeyCode::Down
                if !app.search_results().is_empty() => {
                    app.selected_novel =
                        (app.selected_novel + 1).min(app.search_results().len() - 1);
                }
            KeyCode::Char('k') | KeyCode::Up => {
                app.selected_novel = app.selected_novel.saturating_sub(1);
            }
            KeyCode::Enter => {
                if let Some(novel) = app.search_results().get(app.selected_novel).cloned() {
                    if let Err(e) = app.save_novel_to_library(&novel) {
                        app.status_message = format!("Failed to save novel: {e}");
                    } else {
                        app.current_pane = ActivePane::Library;
                        // Select this novel in the library
                        if let Some(idx) = app.library_novels.iter().position(|n| n.id == novel.id)
                        {
                            app.selected_library_novel = idx;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn handle_library_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('/') => {
            app.current_pane = ActivePane::Search;
            app.search_query.clear();
        }
        KeyCode::Char('m') | KeyCode::Char('M') => {
            if let Ok(items) = app.db().get_storage_items() {
                app.storage_items = items;
                app.storage_selected = 0;
                app.current_pane = ActivePane::StorageManager;
            } else {
                app.status_message = "Failed to load storage items".into();
            }
        }
        KeyCode::Delete => {
            if !app.library_novels.is_empty()
                && let Some(novel) = app.library_novels.get(app.selected_library_novel).cloned() {
                    let _ = app.db().delete_novel(&novel.id);
                    app.library_novels.remove(app.selected_library_novel);
                    if app.selected_library_novel >= app.library_novels.len() {
                        app.selected_library_novel = app.library_novels.len().saturating_sub(1);
                    }
                    app.status_message = format!("Removed '{}'", novel.title);
                }
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if let Some(novel) = app.library_novels.get(app.selected_library_novel).cloned()
                && let Ok(chapters) = app.db().get_novel_chapters(&novel.id) {
                    let _ = app
                        .download_tx
                        .send(crate::downloader::DownloadCommand::QueueNovel(
                            novel.id, chapters,
                        ));
                    app.status_message = format!("Queued '{}' for download", novel.title);
                }
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            if let Some(novel) = app.library_novels.get(app.selected_library_novel).cloned() {
                app.status_message = format!("Exporting '{}' to EPUB...", novel.title);
                let novel_id = novel.id.clone();
                let tx = app.event_sender();
                let export_dir = app.config.get_export_dir();
                std::thread::spawn(move || {
                    let db = match crate::db::Database::new() {
                        Ok(db) => db,
                        Err(e) => {
                            let _ =
                                tx.send(crate::app::AppEvent::Error(format!("DB Error: {}", e)));
                            return;
                        }
                    };

                    match crate::exporter::export_to_epub(&db, &novel_id, &export_dir) {
                        Ok(path) => {
                            let _ = tx.send(crate::app::AppEvent::ExportCompleted(format!(
                                "EPUB saved to {:?}",
                                path
                            )));
                        }
                        Err(e) => {
                            let _ = tx
                                .send(crate::app::AppEvent::Error(format!("Export failed: {}", e)));
                        }
                    }
                });
            }
        }
        KeyCode::Tab | KeyCode::Char('l') | KeyCode::Right
            if !app.chapters.is_empty() => {
                app.current_pane = ActivePane::ChapterList;
            }

        // Navigate novel list.
        KeyCode::Char('j') | KeyCode::Down
            if !app.library_novels.is_empty() => {
                app.selected_library_novel =
                    (app.selected_library_novel + 1).min(app.library_novels.len() - 1);
            }
        KeyCode::Char('k') | KeyCode::Up => {
            app.selected_library_novel = app.selected_library_novel.saturating_sub(1);
        }

        // Select a novel → fetch its chapters and focus chapter list.
        KeyCode::Enter => {
            if let Some(novel) = app.library_novels.get(app.selected_library_novel).cloned() {
                let url = novel.source_url.clone();
                app.current_novel = Some(novel);
                app.trigger_fetch_chapters(url);
                app.current_pane = ActivePane::ChapterList;
            }
        }

        _ => {}
    }
}

fn handle_chapterlist_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left => {
            app.current_pane = ActivePane::Library;
        }
        KeyCode::Tab => {
            app.current_pane = ActivePane::Library;
        }

        // Navigate chapter list.
        KeyCode::Char('j') | KeyCode::Down
            if !app.chapters.is_empty() => {
                app.selected_chapter = (app.selected_chapter + 1).min(app.chapters.len() - 1);
            }
        KeyCode::Char('k') | KeyCode::Up => {
            app.selected_chapter = app.selected_chapter.saturating_sub(1);
        }

        // Download selected chapter.
        KeyCode::Char('d') => {
            if let Some(ch) = app.chapters.get(app.selected_chapter).cloned() {
                app.trigger_download_chapter(ch.id, ch.url);
            }
        }

        // Select a chapter → Read
        KeyCode::Enter => {
            if let Some(ch) = app.chapters.get(app.selected_chapter).cloned() {
                app.trigger_download_chapter(ch.id, ch.url);
                app.current_pane = ActivePane::Reading;
            }
        }
        _ => {}
    }
}

fn handle_reading_keys(app: &mut App, code: KeyCode) {
    if app.show_settings_panel {
        match code {
            KeyCode::Esc | KeyCode::Char('S') | KeyCode::Char('s') => {
                app.show_settings_panel = false;
            }
            _ => {}
        }

        // Handle precise settings toggles:
        if let KeyCode::Char(c) = code {
            match c {
                'w' => app.reader_settings.text_width = app.reader_settings.text_width.next(),
                'm' => app.reader_settings.margin_preset = app.reader_settings.margin_preset.next(),
                'l' => app.reader_settings.line_spacing = app.reader_settings.line_spacing.next(),
                'p' => {
                    app.reader_settings.paragraph_spacing =
                        app.reader_settings.paragraph_spacing.next()
                }
                'c' => app.reader_settings.color_scheme = app.reader_settings.color_scheme.next(),
                'a' => app.reader_settings.alignment = app.reader_settings.alignment.next(),
                _ => {}
            }
        }
        return;
    }

    match code {
        KeyCode::Char('S') | KeyCode::Char('s') => {
            app.show_settings_panel = true;
        }
        KeyCode::Char('q') => {
            app.try_save_progress();
            app.should_quit = true;
        }
        KeyCode::Tab | KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left => {
            app.try_save_progress();
            app.current_pane = ActivePane::ChapterList;
        }
        KeyCode::Char('/') => {
            app.try_save_progress();
            app.current_pane = ActivePane::Search;
            app.search_query.clear();
        }

        // Scroll the reader.
        KeyCode::Char('j') | KeyCode::Down => {
            app.scroll_offset = app.scroll_offset.saturating_add(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.scroll_offset = app.scroll_offset.saturating_sub(1);
        }
        // Page-based scrolling.
        KeyCode::Char('d') => {
            app.scroll_offset = app.scroll_offset.saturating_add(20);
        }
        KeyCode::Char('u') => {
            app.scroll_offset = app.scroll_offset.saturating_sub(20);
        }
        // Home / End.
        KeyCode::Char('g') => {
            app.scroll_offset = 0;
        }
        KeyCode::Char('G') => {
            // Ideally we'd measure wrapped lines, but a large number works as End for now.
            app.scroll_offset = usize::MAX;
        }

        // Next / Previous chapter.
        KeyCode::Char('n') => {
            app.try_save_progress();
            if app.selected_chapter > 0 {
                app.selected_chapter -= 1;
                app.load_selected_chapter();
            }
        }
        KeyCode::Char('p') => {
            app.try_save_progress();
            if app.selected_chapter + 1 < app.chapters.len() {
                app.selected_chapter += 1;
                app.load_selected_chapter();
            }
        }
        KeyCode::Char('t') => {
            app.theme_index = (app.theme_index + 1) % 4;
        }
        _ => {}
    }
}

fn handle_storage_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('M') | KeyCode::Char('m') | KeyCode::Tab => {
            app.current_pane = ActivePane::Library;
        }
        KeyCode::Char('j') | KeyCode::Down
            if app.storage_selected + 1 < app.storage_items.len() => {
                app.storage_selected += 1;
            }
        KeyCode::Char('k') | KeyCode::Up => {
            app.storage_selected = app.storage_selected.saturating_sub(1);
        }
        KeyCode::Delete => {
            if let Some(item) = app.storage_items.get(app.storage_selected).cloned() {
                if app.db().clear_novel_downloads(&item.novel_id).is_ok() {
                    app.status_message = format!("Cleared downloads for '{}'", item.title);
                    if let Ok(items) = app.db().get_storage_items() {
                        app.storage_items = items;
                        if app.storage_selected >= app.storage_items.len() {
                            app.storage_selected = app.storage_items.len().saturating_sub(1);
                        }
                    }
                } else {
                    app.status_message = "Failed to clear downloads".into();
                }
            }
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.current_pane =
                ActivePane::Prompt(app.config.get_export_dir().to_string_lossy().into_owned());
        }
        _ => {}
    }
}

fn handle_prompt_keys(app: &mut App, code: KeyCode) {
    if let ActivePane::Prompt(ref mut text) = app.current_pane {
        match code {
            KeyCode::Esc => {
                app.current_pane = ActivePane::StorageManager;
            }
            KeyCode::Enter => {
                let path = std::path::PathBuf::from(text.clone());
                app.config.export_dir = Some(path);
                app.config.save();
                app.status_message = "Export path updated".into();
                app.current_pane = ActivePane::StorageManager;
            }
            KeyCode::Backspace => {
                text.pop();
            }
            KeyCode::Char(c) => {
                text.push(c);
            }
            _ => {}
        }
    }
}
