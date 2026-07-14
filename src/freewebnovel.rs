use async_trait::async_trait;
use primp::{Client, Impersonate};
use scraper::{Html, Selector};

use crate::error::SageError;
use crate::models::{Chapter, Novel};
use crate::scraper::NovelProvider;

const FREEWEBNOVEL_BASE: &str = "https://freewebnovel.com";

pub struct FreeWebNovel {
    client: Client,
}

impl FreeWebNovel {
    pub fn new() -> Self {
        let client = Client::builder()
            .impersonate(Impersonate::ChromeV144)
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build primp client");
        Self { client }
    }

    async fn fetch_page(&self, url: &str) -> Result<String, SageError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| SageError::ScrapingError(e.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            return Err(SageError::ScrapingError(format!("HTTP error {}", status)));
        }
        let html = response
            .text()
            .await
            .map_err(|e| SageError::ScrapingError(e.to_string()))?;
        
        crate::scraper::detect_cloudflare(&html, url)?;
        Ok(html)
    }

    fn resolve_url(href: &str) -> String {
        if href.starts_with("http://") || href.starts_with("https://") {
            href.to_string()
        } else if href.starts_with('/') {
            format!("{}{}", FREEWEBNOVEL_BASE, href)
        } else {
            format!("{}/{}", FREEWEBNOVEL_BASE, href)
        }
    }
}

#[async_trait]
impl NovelProvider for FreeWebNovel {
    fn source_id(&self) -> &'static str {
        "freewebnovel"
    }

    fn base_url(&self) -> &'static str {
        FREEWEBNOVEL_BASE
    }

    async fn search(&self, query: &str) -> Result<Vec<Novel>, SageError> {
        let url = format!("{}/search?searchkey={}", FREEWEBNOVEL_BASE, urlencoding::encode(query));
        let html = self.fetch_page(&url).await?;
        let document = Html::parse_document(&html);
        
        let sel_tit = Selector::parse(".tit > a").unwrap();
        
        let mut novels = Vec::new();
        for node in document.select(&sel_tit) {
            if let Some(href) = node.value().attr("href") {
                let title = node.text().collect::<Vec<_>>().join("");
                let url = Self::resolve_url(href);
                novels.push(Novel {
                    id: url.clone(),
                    title,
                    author: "Unknown".to_string(),
                    cover_url: String::new(),
                    source_url: url,
                    description: "".to_string(),
                });
            }
        }
        
        Ok(novels)
    }

    async fn fetch_chapters(&self, novel_url: &str) -> Result<Vec<Chapter>, SageError> {
        let html = self.fetch_page(novel_url).await?;
        let document = Html::parse_document(&html);
        
        // Chapters are inside `li > a` that have titles containing Chapter.
        let sel_a = Selector::parse("li > a").unwrap();
        
        let mut chapters = Vec::new();
        let mut i = 1;
        for node in document.select(&sel_a) {
            if let (Some(href), Some(title)) = (node.value().attr("href"), node.value().attr("title")) {
                if href.contains("/chapter-") || href.contains(".html") {
                    let url = Self::resolve_url(href);
                    let title = title.trim().to_string();
                    if title.is_empty() {
                        continue;
                    }
                    chapters.push(Chapter {
                        id: url.clone(),
                        novel_id: novel_url.to_string(),
                        title,
                        url,
                        chapter_number: i as f32,
                        content: None,
                        is_downloaded: false,
                    });
                    i += 1;
                }
            }
        }
        
        if chapters.is_empty() {
            return Err(SageError::ElementNotFound { selector: "FreeWebNovel chapters".into() });
        }
        
        Ok(chapters)
    }

    async fn fetch_chapter_content(&self, chapter_url: &str) -> Result<String, SageError> {
        let html = self.fetch_page(chapter_url).await?;
        let document = Html::parse_document(&html);
        
        let sel_article = Selector::parse("#article").unwrap();
        if let Some(article) = document.select(&sel_article).next() {
            let mut text = String::new();
            for p in article.select(&Selector::parse("p").unwrap()) {
                text.push_str(&p.text().collect::<Vec<_>>().join(""));
                text.push('\n');
                text.push('\n');
            }
            if text.is_empty() {
                text = article.text().collect::<Vec<_>>().join("\n");
            }
            Ok(text.trim().to_string())
        } else {
            Err(SageError::ElementNotFound { selector: "#article".into() })
        }
    }
}
