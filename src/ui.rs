/// TUI rendering — btop-inspired aesthetic with Tokyo Night colour palette.
///
/// Layout:
/// ┌──────────────┬───────────────────────────────────────────┐
/// │  Sidebar     │  Main Content                             │
/// │  (30 %)      │  (70 %)                                   │
/// │              │                                           │
/// │  Library /   │  Chapter reader with word-wrap & scroll   │
/// │  Chapters    │                                           │
/// ├──────────────┴───────────────────────────────────────────┤
/// │  Status bar                                              │
/// └──────────────────────────────────────────────────────────┘
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Gauge, List, ListItem, Paragraph, Row, Table, Wrap,
    },
};

use crate::app::{ActivePane, App};

// ──────────────────────────── Public API ──────────────────────────────

/// Render the entire TUI frame.
pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.size();

    // Fill the whole background.
    let bg_block = Block::default().style(Style::default().bg(app.theme().bg));
    f.render_widget(bg_block, size);

    // ── Top-level vertical split: body (rest) | status bar (3 rows) ──
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(size);

    let body_area = outer[0];
    let status_area = outer[1];

    // ── Horizontal split: sidebar 30% | main 70% ────────────────────
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(35), Constraint::Min(0)])
        .split(body_area);

    let sidebar_area = columns[0];
    let main_area = columns[1];

    // ── Draw each section ───────────────────────────────────────────
    draw_sidebar(f, app, sidebar_area);
    draw_main(f, app, main_area);
    draw_status_bar(f, app, status_area);
}

// ─────────────────────────── Sidebar ─────────────────────────────────

fn draw_sidebar(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    // Split sidebar into two halves: top = novels/search, bottom = chapters.
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    draw_novel_list(f, app, sections[0]);
    draw_chapter_list(f, app, sections[1]);
}

/// Render the novel / search results list.
fn draw_novel_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let (title, novels, selected) = match app.current_pane {
        ActivePane::Search => (
            " ◈ Search Results ",
            app.search_results(),
            app.selected_novel,
        ),
        _ => (
            " ◈ Library ",
            app.library_novels.as_slice(),
            app.selected_library_novel,
        ),
    };

    let block = make_block(
        title,
        app.current_pane == ActivePane::Library || app.current_pane == ActivePane::Search,
        app.theme(),
    );

    let items: Vec<ListItem> = if novels.is_empty() {
        vec![ListItem::new(Span::styled(
            "  No novels found",
            Style::default().fg(app.theme().muted),
        ))]
    } else {
        novels
            .iter()
            .enumerate()
            .map(|(i, novel)| {
                let style = if i == selected {
                    Style::default()
                        .fg(app.theme().accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(app.theme().fg)
                };
                let marker = if i == selected { "┃ " } else { "  " };
                ListItem::new(Span::styled(format!("{marker}{}", novel.title), style))
            })
            .collect()
    };

    let list = List::new(items).block(block);
    let mut state = ratatui::widgets::ListState::default().with_selected(Some(selected));
    f.render_stateful_widget(list, area, &mut state);
}

/// Render the chapter list for the currently selected novel.
fn draw_chapter_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = make_block(
        " ◈ Chapters ",
        app.current_pane == ActivePane::ChapterList,
        app.theme(),
    );

    let items: Vec<ListItem> = if app.chapters.is_empty() {
        vec![ListItem::new(Span::styled(
            "  Select a novel first",
            Style::default().fg(app.theme().muted),
        ))]
    } else {
        app.chapters
            .iter()
            .enumerate()
            .map(|(i, ch)| {
                let (marker, color) = if i == app.selected_chapter {
                    ("┃ ", app.theme().accent)
                } else if ch.is_downloaded {
                    ("✓ ", app.theme().green)
                } else {
                    ("  ", app.theme().fg)
                };

                let style = if i == app.selected_chapter {
                    Style::default().fg(color).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(color)
                };

                ListItem::new(Span::styled(format!("{marker}{}", ch.title), style))
            })
            .collect()
    };

    let list = List::new(items).block(block);
    let mut state =
        ratatui::widgets::ListState::default().with_selected(Some(app.selected_chapter));
    f.render_stateful_widget(list, area, &mut state);
}

// ──────────────────────────── Main area ──────────────────────────────

fn draw_main(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    match app.current_pane {
        ActivePane::Reading => {
            draw_reader(f, app, area);
            if app.show_settings_panel {
                draw_settings_panel(f, app, area);
            }
        }
        ActivePane::Search => draw_search_input(f, app, area),
        ActivePane::Downloads => draw_downloads(f, app, area),
        ActivePane::StorageManager => draw_storage_manager(f, app, area),
        ActivePane::Prompt(ref text) => {
            draw_storage_manager(f, app, area);
            draw_prompt(f, app, area, text);
        }
        _ => draw_welcome(f, app, area),
    }
}

/// Render the chapter reader with word-wrap and scroll support.
fn draw_reader(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let chapter = app.chapters.get(app.selected_chapter);
    let chapter_title = chapter.map(|c| c.title.as_str()).unwrap_or("Chapter");

    let total_chapters = app.chapters.len();
    let current_idx = app.selected_chapter.saturating_add(1);

    let title = format!(" ◈ {} [{}/{}] ", chapter_title, current_idx, total_chapters);

    let raw_content = app.current_chapter_content.as_deref().unwrap_or(
        "No content loaded.

Select a chapter and press 'd' to download.",
    );

    let settings = &app.reader_settings;

    // Apply line spacing and paragraph spacing
    let mut processed_content = String::with_capacity(raw_content.len());
    let extra_line_spacing = match settings.line_spacing {
        crate::reader_settings::LineSpacing::Single => 0,
        crate::reader_settings::LineSpacing::Relaxed => 1,
        crate::reader_settings::LineSpacing::Double => 2,
    };

    let mut is_first = true;
    for line in raw_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if !is_first {
            match settings.paragraph_spacing {
                crate::reader_settings::ParagraphSpacing::Compact => {
                    processed_content.push('\n');
                    for _ in 0..extra_line_spacing {
                        processed_content.push('\n');
                    }
                }
                crate::reader_settings::ParagraphSpacing::Normal => {
                    processed_content.push('\n');
                    processed_content.push('\n');
                    for _ in 0..extra_line_spacing {
                        processed_content.push('\n');
                        processed_content.push('\n');
                    }
                }
                crate::reader_settings::ParagraphSpacing::Relaxed => {
                    processed_content.push('\n');
                    processed_content.push('\n');
                    processed_content.push('\n');
                    for _ in 0..extra_line_spacing {
                        processed_content.push('\n');
                        processed_content.push('\n');
                        processed_content.push('\n');
                    }
                }
            }
        }
        is_first = false;

        if settings.paragraph_spacing == crate::reader_settings::ParagraphSpacing::Compact {
            processed_content.push_str("  ");
        }
        processed_content.push_str(trimmed);
    }

    let (fg, bg) = settings
        .color_scheme
        .colors(app.theme().fg, app.theme().surface);

    let mut block =
        make_block(title, !app.show_settings_panel, app.theme()).style(Style::default().bg(bg));

    // Apply margins
    block = block.padding(settings.margin_preset.to_padding());

    // Apply text alignment
    let paragraph = Paragraph::new(processed_content)
        .block(block)
        .alignment(settings.alignment.to_ratatui())
        .style(Style::default().fg(fg).bg(bg))
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset as u16, 0));

    // Apply max width
    let content_area = if settings.text_width != crate::reader_settings::TextWidth::Full {
        let cols = settings.text_width.to_columns();
        if area.width > cols + 4 {
            let offset = (area.width - cols) / 2;
            ratatui::layout::Rect {
                x: area.x + offset,
                y: area.y,
                width: cols,
                height: area.height,
            }
        } else {
            area
        }
    } else {
        area
    };

    // If we narrowed the width, fill the remaining background with the reader bg color
    if content_area != area {
        let bg_block = Block::default().style(Style::default().bg(bg));
        f.render_widget(bg_block, area);
    }

    f.render_widget(paragraph, content_area);
}

/// Render the search input pane and results table.
fn draw_search_input(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // 1. Search Input Box
    let input_block = make_block(" ◈ Search ", app.search_input_focused, app.theme());
    let cursor = if app.search_input_focused { "▏" } else { "" };
    let input_style = if app.search_input_focused {
        Style::default()
            .fg(app.theme().accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(app.theme().muted)
    };

    let input_lines = vec![Line::from(vec![
        Span::styled("Query: ", Style::default().fg(app.theme().muted)),
        Span::styled(&app.search_query, input_style),
        Span::styled(cursor, Style::default().fg(app.theme().accent)),
    ])];

    let input_paragraph = Paragraph::new(input_lines)
        .block(input_block)
        .style(Style::default().bg(app.theme().surface));
    f.render_widget(input_paragraph, chunks[0]);

    // 2. Search Results Table
    match &app.search_state {
        crate::app::SearchState::Idle => {
            let empty = Paragraph::new("Type a query and press Enter to search.")
                .style(Style::default().fg(app.theme().muted))
                .alignment(Alignment::Center)
                .block(make_block(
                    " Results ",
                    !app.search_input_focused,
                    app.theme(),
                ));
            f.render_widget(empty, chunks[1]);
        }
        crate::app::SearchState::Searching => {
            let searching = Paragraph::new("Searching...")
                .style(
                    Style::default()
                        .fg(ratatui::style::Color::Cyan)
                        .add_modifier(Modifier::RAPID_BLINK),
                )
                .alignment(Alignment::Center)
                .block(make_block(
                    " Results ",
                    !app.search_input_focused,
                    app.theme(),
                ));
            f.render_widget(searching, chunks[1]);
        }
        crate::app::SearchState::NoResults => {
            let empty = Paragraph::new("No titles found. The source catalog layout may have changed, or your client fingerprint was rejected.")
                .style(Style::default().fg(app.theme().muted))
                .alignment(Alignment::Center)
                .block(make_block(" Results ", !app.search_input_focused, app.theme()));
            f.render_widget(empty, chunks[1]);
        }
        crate::app::SearchState::Failure(err) => {
            let block = Block::default()
                .title(" Error ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(ratatui::style::Color::Red))
                .style(Style::default().bg(app.theme().bg));

            let error_msg = Paragraph::new(err.as_str())
                .style(Style::default().fg(ratatui::style::Color::Red))
                .alignment(Alignment::Center)
                .block(block);
            f.render_widget(error_msg, chunks[1]);
        }
        crate::app::SearchState::Success(novels) => {
            let table_block = make_block(" Results ", !app.search_input_focused, app.theme());
            let header_cells = ["Title", "Author", "Source"].iter().map(|h| {
                Cell::from(*h).style(
                    Style::default()
                        .fg(app.theme().cyan)
                        .add_modifier(Modifier::BOLD),
                )
            });
            let header = Row::new(header_cells)
                .style(Style::default().bg(app.theme().bg))
                .height(1)
                .bottom_margin(1);

            let rows = novels.iter().enumerate().map(|(i, novel)| {
                let style = if !app.search_input_focused && i == app.selected_novel {
                    Style::default()
                        .fg(app.theme().bg)
                        .bg(app.theme().accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(app.theme().fg)
                };

                let source_domain = novel
                    .source_url
                    .split('/')
                    .nth(2)
                    .unwrap_or(&novel.source_url);

                Row::new(vec![
                    Cell::from(novel.title.clone()),
                    Cell::from(novel.author.clone()),
                    Cell::from(source_domain.to_string()),
                ])
                .style(style)
            });

            let table = Table::new(
                rows,
                [
                    Constraint::Percentage(50),
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                ],
            )
            .header(header)
            .block(table_block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("┃ ");

            f.render_widget(table, chunks[1]);
        }
    }
}

/// Render the welcome / landing page.
fn draw_welcome(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = make_block(" ◈ Sage ", true, app.theme());

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "   ███████╗ █████╗  ██████╗ ███████╗",
            Style::default()
                .fg(app.theme().accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "   ██╔════╝██╔══██╗██╔════╝ ██╔════╝",
            Style::default()
                .fg(app.theme().accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "   ███████╗███████║██║  ███╗█████╗  ",
            Style::default()
                .fg(app.theme().accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "   ╚════██║██╔══██║██║   ██║██╔══╝  ",
            Style::default()
                .fg(app.theme().accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "   ███████║██║  ██║╚██████╔╝███████╗",
            Style::default()
                .fg(app.theme().accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "   ╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚══════╝",
            Style::default()
                .fg(app.theme().accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "   A Modern WebNovel Reader TUI",
            Style::default()
                .fg(app.theme().fg)
                .add_modifier(Modifier::ITALIC),
        )),
        Line::from(Span::styled(
            "   by @musprodev",
            Style::default()
                .fg(app.theme().muted)
                .add_modifier(Modifier::ITALIC),
        )),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "  Global Keys",
            Style::default()
                .fg(app.theme().cyan)
                .add_modifier(Modifier::BOLD),
        )),
        make_help_line("  Tab", "Switch panes", app.theme()),
        make_help_line("  t", "Toggle theme", app.theme()),
        make_help_line("  q", "Quit", app.theme()),
        Line::from(""),
        Line::from(Span::styled(
            "  Library Keys",
            Style::default()
                .fg(app.theme().cyan)
                .add_modifier(Modifier::BOLD),
        )),
        make_help_line("  ↑ k / ↓ j", "Navigate lists", app.theme()),
        make_help_line("  /", "Search online directory", app.theme()),
        make_help_line("  Enter", "Open chapter list", app.theme()),
        make_help_line("  D", "Download all chapters", app.theme()),
        make_help_line("  E", "Export to EPUB", app.theme()),
        make_help_line("  M", "Storage Manager", app.theme()),
        make_help_line("  Del", "Remove from library", app.theme()),
        Line::from(""),
        Line::from(Span::styled(
            "  Reading Keys",
            Style::default()
                .fg(app.theme().cyan)
                .add_modifier(Modifier::BOLD),
        )),
        make_help_line("  Enter", "Open chapter", app.theme()),
        make_help_line("  S", "Reader Settings", app.theme()),
        make_help_line("  ↑ k / ↓ j", "Scroll up / down", app.theme()),
        make_help_line("  n / p", "Next / Prev chapter", app.theme()),
        make_help_line("  b", "Bookmark chapter", app.theme()),
    ];

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(app.theme().surface))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

// ──────────────────────────── Status bar ──────────────────────────────

fn draw_downloads(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = make_block(" ◈ Downloads ", true, app.theme());
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    if app.downloads_progress.is_empty() {
        let empty = Paragraph::new("No active downloads.")
            .style(Style::default().fg(app.theme().muted))
            .alignment(Alignment::Center);
        f.render_widget(empty, inner_area);
        return;
    }

    let item_height = 3; // Space for title + gauge
    let max_items = (inner_area.height / item_height) as usize;
    let items_count = std::cmp::min(app.downloads_progress.len(), max_items);
    if items_count == 0 {
        return;
    }

    let constraints: Vec<Constraint> = (0..items_count)
        .map(|_| Constraint::Length(item_height))
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner_area);

    for (i, (novel_id, (current, total))) in
        app.downloads_progress.iter().take(max_items).enumerate()
    {
        let percent = if *total == 0 {
            0.0
        } else {
            (*current as f64 / *total as f64) * 100.0
        };

        // Try to find the novel title, else fallback to novel_id.
        let title = app
            .search_results()
            .iter()
            .chain(app.current_novel.iter())
            .find(|n| &n.id == novel_id)
            .map(|n| n.title.clone())
            .unwrap_or_else(|| novel_id.clone());

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .title(title)
                    .style(Style::default().fg(app.theme().fg)),
            )
            .gauge_style(
                Style::default()
                    .fg(app.theme().cyan)
                    .bg(app.theme().surface),
            )
            .percent(percent as u16)
            .label(format!("{}/{} Chapters", current, total));

        f.render_widget(gauge, chunks[i]);
    }
}

// ──────────────────────────── Status bar ──────────────────────────────

fn draw_status_bar(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let loading_indicator = if app.is_loading { " ⟳ " } else { "" };

    let status_color = if app.status_message.contains("error")
        || app.status_message.contains("failed")
        || app.status_message.contains("Error")
    {
        app.theme().red
    } else if app.is_loading {
        app.theme().yellow
    } else {
        app.theme().green
    };

    let pane_label = match app.current_pane {
        ActivePane::Library => "LIBRARY",
        ActivePane::ChapterList => "CHAPTERS",
        ActivePane::Search => "SEARCH",
        ActivePane::Reading => "READING",
        ActivePane::Downloads => "DOWNLOADS",
        ActivePane::StorageManager | ActivePane::Prompt(_) => "STORAGE MANAGER",
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(
                format!(" {pane_label} "),
                Style::default()
                    .fg(app.theme().bg)
                    .bg(app.theme().accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" │ ", Style::default().fg(app.theme().muted)),
            Span::styled(loading_indicator, Style::default().fg(app.theme().yellow)),
            Span::styled(&app.status_message, Style::default().fg(status_color)),
        ]),
        Line::from(""), // spacing
        Line::from(vec![Span::styled(
            match app.current_pane {
                ActivePane::Library => {
                    " [Enter] Read  [D] Download All  [M] Manage Storage  [E] Export EPUB  [Del] Remove "
                }
                ActivePane::Reading => {
                    " [J/K] Scroll  [N/P] Next/Prev Chapter  [B] Bookmark  [S] Settings  [Esc] Back "
                }
                ActivePane::StorageManager => {
                    " [J/K] Navigate  [C] Change Export Path  [Del] Clear Downloads  [M/Esc] Back "
                }
                ActivePane::Prompt(_) => " [Enter] Save  [Esc] Cancel ",
                _ => " [Tab] Switch Panes  [T] Theme  [Q] Quit ",
            },
            Style::default()
                .fg(app.theme().muted)
                .add_modifier(Modifier::BOLD),
        )]),
    ];

    let block = Block::default()
        .borders(Borders::TOP)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme().border))
        .style(Style::default().bg(app.theme().bg));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

// ──────────────────────────── Helpers ─────────────────────────────────

/// Build a rounded block with the Tokyo Night aesthetic.
/// `focused` controls whether the border is bright (active pane) or muted.
fn make_block<'a, T: Into<ratatui::widgets::block::Title<'a>>>(
    title: T,
    focused: bool,
    theme: &crate::theme::Theme,
) -> Block<'a> {
    let border_color = if focused { theme.border } else { theme.muted };

    Block::default()
        .title(title)
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.surface).fg(theme.fg))
}

/// Format a single help-line: `key` in accent, `desc` in muted.
fn make_help_line<'a>(key: &'a str, desc: &'a str, theme: &crate::theme::Theme) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("{key:<14}"),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(desc, Style::default().fg(theme.muted)),
    ])
}

/// Render the settings popup
fn draw_settings_panel(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let popup_area = centered_rect(50, 40, area); // 50% width, 40% height
    f.render_widget(ratatui::widgets::Clear, popup_area);

    let block = make_block(" Reader Settings (Arrows to adjust) ", true, app.theme());

    let settings = &app.reader_settings;

    let w_text = match settings.text_width {
        crate::reader_settings::TextWidth::Narrow => "Narrow (60)",
        crate::reader_settings::TextWidth::Medium => "Medium (80)",
        crate::reader_settings::TextWidth::Wide => "Wide (100)",
        crate::reader_settings::TextWidth::Full => "Full",
    };

    let m_text = match settings.margin_preset {
        crate::reader_settings::MarginPreset::Compact => "Compact",
        crate::reader_settings::MarginPreset::Normal => "Normal",
        crate::reader_settings::MarginPreset::Wide => "Wide",
    };

    let s_text = match settings.line_spacing {
        crate::reader_settings::LineSpacing::Single => "Single",
        crate::reader_settings::LineSpacing::Relaxed => "Relaxed",
        crate::reader_settings::LineSpacing::Double => "Double",
    };

    let c_text = match settings.color_scheme {
        crate::reader_settings::ReaderColorScheme::Default => "Default",
        crate::reader_settings::ReaderColorScheme::Sepia => "Sepia",
        crate::reader_settings::ReaderColorScheme::Paper => "Paper",
        crate::reader_settings::ReaderColorScheme::SoftDark => "Soft Dark",
    };

    let a_text = match settings.alignment {
        crate::reader_settings::TextAlignment::Left => "Left",
        crate::reader_settings::TextAlignment::Center => "Center",
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Width:      [w] "),
            Span::styled(w_text, Style::default().fg(app.theme().accent)),
        ]),
        Line::from(vec![
            Span::raw("  Margin:     [m] "),
            Span::styled(m_text, Style::default().fg(app.theme().accent)),
        ]),
        Line::from(vec![
            Span::raw("  Line Space: [l] "),
            Span::styled(s_text, Style::default().fg(app.theme().accent)),
        ]),
        Line::from(vec![
            Span::raw("  Para Space: [p] "),
            Span::styled(
                match settings.paragraph_spacing {
                    crate::reader_settings::ParagraphSpacing::Compact => "Compact",
                    crate::reader_settings::ParagraphSpacing::Normal => "Normal",
                    crate::reader_settings::ParagraphSpacing::Relaxed => "Relaxed",
                },
                Style::default().fg(app.theme().accent),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Colors:     [c] "),
            Span::styled(c_text, Style::default().fg(app.theme().accent)),
        ]),
        Line::from(vec![
            Span::raw("  Alignment:  [a] "),
            Span::styled(a_text, Style::default().fg(app.theme().accent)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  [Esc] Close Settings",
            Style::default().fg(app.theme().muted),
        )),
    ];

    let p = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(app.theme().surface));

    f.render_widget(p, popup_area);
}

fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// ──────────────────────────── Storage Manager ──────────────────────────────
fn format_size(bytes: usize) -> String {
    let mb = bytes as f64 / 1_048_576.0;
    if mb < 1.0 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", mb)
    }
}

fn draw_storage_manager(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = make_block(" ◈ Storage Manager ", true, app.theme());

    if app.storage_items.is_empty() {
        let empty =
            Paragraph::new("No downloaded chapters found.\nPress 'M' to return to library.")
                .style(Style::default().fg(app.theme().muted))
                .alignment(Alignment::Center)
                .block(block);
        f.render_widget(empty, area);
        return;
    }

    // Find max size to calculate percentage bars
    let max_bytes = app
        .storage_items
        .iter()
        .map(|item| item.size_bytes)
        .max()
        .unwrap_or(1);
    let total_bytes: usize = app.storage_items.iter().map(|item| item.size_bytes).sum();
    let total_label = format!(
        " Export Path: {} │ Total Used: {} ",
        app.config.get_export_dir().to_string_lossy(),
        format_size(total_bytes)
    );

    let list_block = block.title_bottom(
        ratatui::text::Line::from(total_label)
            .alignment(Alignment::Right)
            .style(Style::default().fg(app.theme().accent)),
    );

    let items: Vec<ListItem> = app
        .storage_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == app.storage_selected;

            let style = if is_selected {
                Style::default()
                    .bg(app.theme().surface)
                    .fg(app.theme().fg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme().fg)
            };

            // Like ncdu: [Size] [# Chapters] [Bar] Title
            let size_str = format_size(item.size_bytes);
            let size_pad = format!("{:>9}", size_str);

            let ch_str = format!("{} chs", item.downloaded_chapters);
            let ch_pad = format!("{:>8}", ch_str);

            let percentage = (item.size_bytes as f64 / max_bytes as f64) * 10.0;
            let bar_len = percentage.round() as usize;
            let bar = "#".repeat(bar_len) + &".".repeat(10 - bar_len);

            let text = format!(" {} │ {} │ [{}] {}", size_pad, ch_pad, bar, item.title);

            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items).block(list_block);
    let mut state =
        ratatui::widgets::ListState::default().with_selected(Some(app.storage_selected));
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_prompt(f: &mut Frame, app: &App, area: ratatui::layout::Rect, text: &str) {
    let block = make_block(" ◈ Export Directory ", true, app.theme());

    let paragraph = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(app.theme().accent))
        .alignment(Alignment::Left);

    // Draw in the middle of the screen
    let area = centered_rect(60, 20, area);
    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(paragraph, area);
}
