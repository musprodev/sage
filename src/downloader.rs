use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Semaphore, mpsc};
use tokio::time::interval;

use crate::app::AppEvent;
use crate::db::Database;
use crate::models::Chapter;
use crate::scraper::NovelProvider;

#[derive(Debug)]
pub enum DownloadCommand {
    QueueNovel(String, Vec<Chapter>),
    Pause,
    Resume,
}

pub struct DownloadManager {
    pub cmd_tx: mpsc::UnboundedSender<DownloadCommand>,
}

impl DownloadManager {
    /// Starts the background download task and returns the `DownloadManager`
    /// handle which can be used to send commands to the queue.
    pub fn start<P: NovelProvider + Send + Sync + 'static>(
        event_tx: mpsc::UnboundedSender<AppEvent>,
        provider: Arc<P>,
    ) -> Self {
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<DownloadCommand>();

        tokio::spawn(async move {
            let db = match Database::new() {
                Ok(db) => db,
                Err(e) => {
                    let _ = event_tx.send(AppEvent::Error(format!("Downloader DB error: {e}")));
                    return;
                }
            };

            let mut queue: VecDeque<Chapter> = VecDeque::new();
            let mut is_paused = false;

            // Limit to 5 concurrent connections.
            let semaphore = Arc::new(Semaphore::new(5));
            let (done_tx, mut done_rx) = mpsc::unbounded_channel::<Chapter>();

            // Novel ID -> (current_downloaded, total_chapters)
            let mut progress: std::collections::HashMap<String, (usize, usize)> =
                std::collections::HashMap::new();

            // Rate limiter: at most 1 request spawned every 500ms
            let mut rate_limiter = interval(Duration::from_millis(500));

            loop {
                tokio::select! {
                    // 1. Check for incoming commands (Queue, Pause, Resume).
                    Some(cmd) = cmd_rx.recv() => {
                        match cmd {
                            DownloadCommand::QueueNovel(novel_id, chapters) => {
                                let total = chapters.len();
                                progress.insert(novel_id, (0, total));
                                queue.extend(chapters);
                            }
                            DownloadCommand::Pause => {
                                is_paused = true;
                            }
                            DownloadCommand::Resume => {
                                is_paused = false;
                            }
                        }
                    }

                    // 2. Check for completed downloads to save to DB.
                    Some(chapter) = done_rx.recv() => {
                        if let Err(e) = db.upsert_chapter(&chapter) {
                            let _ = event_tx.send(AppEvent::Error(format!(
                                "Failed to save chapter {}: {e}",
                                chapter.id
                            )));
                        } else {
                            if let Some(p) = progress.get_mut(&chapter.novel_id) {
                                p.0 += 1;
                                let _ = event_tx.send(AppEvent::DownloadProgress(
                                    chapter.novel_id.clone(),
                                    p.0,
                                    p.1,
                                ));
                            }
                        }
                    }

                    // 3. Process the queue, respecting rate limits and concurrency limits.
                    // This branch only executes if not paused, queue is not empty, and the interval has ticked.
                    _ = rate_limiter.tick(), if !is_paused && !queue.is_empty() => {
                        // try_acquire_owned prevents blocking inside the select! if we're at the limit.
                        if let Ok(permit) = semaphore.clone().try_acquire_owned() {
                            let chapter = queue.pop_front().unwrap();
                            let provider = provider.clone();
                            let done_tx = done_tx.clone();
                            let event_tx = event_tx.clone();

                            tokio::spawn(async move {
                                match provider.fetch_chapter_content(&chapter.url).await {
                                    Ok(text) => {
                                        let mut finished = chapter;
                                        finished.content = Some(text);
                                        finished.is_downloaded = true;
                                        let _ = done_tx.send(finished);
                                    }
                                    Err(e) => {
                                        let _ = event_tx.send(AppEvent::Error(format!(
                                            "Failed to download chapter {}: {e}",
                                            chapter.id
                                        )));
                                    }
                                }
                                drop(permit);
                            });
                        }
                    }

                    else => {
                        // All channels closed, shutdown the task.
                        break;
                    }
                }
            }
        });

        Self { cmd_tx }
    }
}
