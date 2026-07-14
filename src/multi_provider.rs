use async_trait::async_trait;

use crate::error::SageError;
use crate::models::{Chapter, Novel};
use crate::scraper::NovelProvider;
use std::sync::Arc;

pub struct MultiProvider {
    providers: Vec<Arc<dyn NovelProvider>>,
}

impl MultiProvider {
    pub fn new(providers: Vec<Arc<dyn NovelProvider>>) -> Self {
        Self { providers }
    }

    fn get_provider_for_url(&self, url: &str) -> Option<Arc<dyn NovelProvider>> {
        self.providers
            .iter()
            .find(|p| url.starts_with(p.base_url()) || (p.source_id() == "novelbuddy" && url.contains("novelbuddy.com")))
            .cloned()
    }
}

#[async_trait]
impl NovelProvider for MultiProvider {
    fn source_id(&self) -> &'static str {
        "multi"
    }

    fn base_url(&self) -> &'static str {
        ""
    }

    async fn search(&self, query: &str) -> Result<Vec<Novel>, SageError> {
        let mut all_novels = Vec::new();
        for provider in &self.providers {
            if let Ok(mut novels) = provider.search(query).await {
                all_novels.append(&mut novels);
            }
        }
        if all_novels.is_empty() {
            Err(SageError::ElementNotFound {
                selector: "No novels found in any source".into(),
            })
        } else {
            Ok(all_novels)
        }
    }

    async fn fetch_chapters(&self, novel_url: &str) -> Result<Vec<Chapter>, SageError> {
        if let Some(provider) = self.get_provider_for_url(novel_url) {
            provider.fetch_chapters(novel_url).await
        } else {
            Err(SageError::ScrapingError("No provider found for URL".into()))
        }
    }

    async fn fetch_chapter_content(&self, chapter_url: &str) -> Result<String, SageError> {
        if let Some(provider) = self.get_provider_for_url(chapter_url) {
            provider.fetch_chapter_content(chapter_url).await
        } else {
            Err(SageError::ScrapingError("No provider found for URL".into()))
        }
    }
}
