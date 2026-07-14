use async_trait::async_trait;
use htmlescape::decode_html;
use primp::{Client, Impersonate};
use regex::Regex;
use scraper::{Html, Selector};
use std::sync::LazyLock;

use crate::error::SageError;
use crate::models::{Chapter, Novel};

// ─────────────────────── Compiled regexes ──────────────────────────────
// Using LazyLock (stable since Rust 1.80) so each pattern is compiled once.

static RE_SCRIPT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap());
static RE_IFRAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<iframe[^>]*>.*?</iframe>").unwrap());
static RE_STYLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap());

// ────────────────────────────── Constants ──────────────────────────────

const NOVELBUDDY_BASE: &str = "https://novelbuddy.me";
const NOVELFIRE_BASE: &str = "https://novelfire.net";

// ──────────────────────────── Provider trait ───────────────────────────

/// A trait representing an online novel source.
#[async_trait]
pub trait NovelProvider: Send + Sync {
    fn source_id(&self) -> &'static str;
    fn base_url(&self) -> &'static str;
    async fn search(&self, query: &str) -> Result<Vec<Novel>, SageError>;
    async fn fetch_chapters(&self, novel_url: &str) -> Result<Vec<Chapter>, SageError>;
    async fn fetch_chapter_content(&self, chapter_url: &str) -> Result<String, SageError>;
}

// ──────────────────────── Selector helpers ─────────────────────────────

fn selector(s: &str) -> Result<Selector, SageError> {
    Selector::parse(s).map_err(|e| SageError::ScrapingError(format!("Invalid selector '{s}': {e}")))
}

pub fn detect_cloudflare(html: &str, url: &str) -> Result<(), SageError> {
    if html.contains("cf-browser-verification")
        || html.contains("cf_chl_opt")
        || html.contains("Just a moment...") && html.contains("cloudflare")
    {
        return Err(SageError::CloudflareBlocked {
            url: url.to_string(),
        });
    }
    Ok(())
}

// ────────────────────────── NovelBuddy provider ───────────────────────

pub struct NovelBuddy {
    client: Client,
}

impl NovelBuddy {
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
            .map_err(SageError::Request)?;

        let status = response.status().as_u16();
        if status == 403 || status == 503 {
            return Err(SageError::CloudflareBlocked {
                url: url.to_string(),
            });
        }

        let body = response.text().await.map_err(SageError::Request)?;
        detect_cloudflare(&body, url)?;
        Ok(body)
    }

    fn novel_id_from_url(url: &str) -> String {
        url.trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("unknown")
            .to_string()
    }

    fn chapter_id_from_url(novel_id: &str, url: &str) -> String {
        let slug = url
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("unknown");
        format!("{novel_id}-{slug}")
    }

    fn resolve_url(href: &str) -> String {
        if href.starts_with("http://") || href.starts_with("https://") {
            href.to_string()
        } else if href.starts_with('/') {
            format!("{NOVELBUDDY_BASE}{href}")
        } else {
            format!("{NOVELBUDDY_BASE}/{href}")
        }
    }
}

#[async_trait]
impl NovelProvider for NovelBuddy {
    fn source_id(&self) -> &'static str {
        "novelbuddy"
    }

    fn base_url(&self) -> &'static str {
        NOVELBUDDY_BASE
    }

    async fn search(&self, query: &str) -> Result<Vec<Novel>, SageError> {
        let url = format!("{NOVELBUDDY_BASE}/search?q={}", urlencoded(query));
        let html = self.fetch_page(&url).await?;
        let document = Html::parse_document(&html);

        let next_data_sel = selector("script#__NEXT_DATA__")?;
        let mut novels = Vec::new();

        if let Some(script_el) = document.select(&next_data_sel).next() {
            let json_text = script_el.inner_html();
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&json_text)
                && let Some(items) = data
                    .pointer("/props/pageProps/ssrItems")
                    .and_then(|v| v.as_array())
                {
                    for item in items {
                        let title = item
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        if title.is_empty() {
                            continue;
                        }

                        let url_path = item.get("url").and_then(|v| v.as_str()).unwrap_or("");
                        let source_url = Self::resolve_url(url_path);
                        let id = Self::novel_id_from_url(&source_url);

                        let author = item
                            .get("author")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        let cover_url = item
                            .get("cover")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let description = item
                            .get("synopsis")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        novels.push(Novel {
                            id,
                            title,
                            author,
                            cover_url,
                            source_url,
                            description,
                        });
                    }
                }
        }

        Ok(novels)
    }

    async fn fetch_chapters(&self, novel_url: &str) -> Result<Vec<Chapter>, SageError> {
        let novel_id = Self::novel_id_from_url(novel_url);

        // Normalize the URL to use the current domain.
        let normalized_url = if novel_url.contains("novelbuddy.com") {
            novel_url.replace("novelbuddy.com", "novelbuddy.me")
        } else {
            novel_url.to_string()
        };

        let html = self.fetch_page(&normalized_url).await?;

        let mut chapters = Vec::new();
        let mut fallback_number = 0.0;
        let mut manga_hsid = None;
        let mut is_404 = false;

        {
            let document = Html::parse_document(&html);
            let next_data_sel = selector("script#__NEXT_DATA__")?;
            if let Some(script_el) = document.select(&next_data_sel).next() {
                let json_text = script_el.inner_html();
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&json_text) {
                    // Check if the page returned a 404 error.
                    if let Some(status) = data
                        .pointer("/props/pageProps/httpError/status")
                        .and_then(|v| v.as_u64())
                    {
                        if status == 404 {
                            is_404 = true;
                        }
                    }

                    // Primary path: /props/pageProps/mangaHsid
                    if let Some(hsid) = data
                        .pointer("/props/pageProps/mangaHsid")
                        .and_then(|v| v.as_str())
                    {
                        manga_hsid = Some(hsid.to_string());
                    }
                    // Fallback path: /props/pageProps/initialManga/id
                    else if let Some(hsid) = data
                        .pointer("/props/pageProps/initialManga/id")
                        .and_then(|v| v.as_str())
                    {
                        manga_hsid = Some(hsid.to_string());
                    }
                }
            }
        }

        // If we got a 404 and no hsid, try the /novel/ prefixed URL.
        if is_404 && manga_hsid.is_none() {
            let slug = novel_id.clone();
            let retry_url = format!("{NOVELBUDDY_BASE}/novel/{slug}");
            if let Ok(retry_html) = self.fetch_page(&retry_url).await {
                let document = Html::parse_document(&retry_html);
                let next_data_sel = selector("script#__NEXT_DATA__")?;
                if let Some(script_el) = document.select(&next_data_sel).next() {
                    let json_text = script_el.inner_html();
                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(&json_text) {
                        if let Some(hsid) = data
                            .pointer("/props/pageProps/mangaHsid")
                            .and_then(|v| v.as_str())
                            .or_else(|| {
                                data.pointer("/props/pageProps/initialManga/id")
                                    .and_then(|v| v.as_str())
                            })
                        {
                            manga_hsid = Some(hsid.to_string());
                        }
                    }
                }
            }
        }

        // Fetch chapters from the API using the hsid.
        if let Some(hsid) = manga_hsid {
            let api_url = format!("https://api.novelbuddy.me/titles/{}/chapters", hsid);
            if let Ok(api_res) = self.client.get(&api_url).send().await {
                if api_res.status().is_success() {
                    if let Ok(api_body) = api_res.text().await {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&api_body) {
                            if let Some(chapter_list) =
                                data.pointer("/data/chapters").and_then(|v| v.as_array())
                            {
                                for item in chapter_list {
                                    if let (Some(title), Some(url)) = (
                                        item.get("name").and_then(|v| v.as_str()),
                                        item.get("url").and_then(|v| v.as_str()),
                                    ) {
                                        let title = clean_text(std::iter::once(title));
                                        let url = Self::resolve_url(url);
                                        let id =
                                            Self::chapter_id_from_url(&novel_id, &url);
                                        let chapter_number =
                                            extract_chapter_number(&title).unwrap_or_else(|| {
                                                fallback_number += 1.0;
                                                fallback_number
                                            });
                                        chapters.push(Chapter {
                                            id,
                                            novel_id: novel_id.clone(),
                                            title,
                                            url,
                                            chapter_number,
                                            content: None,
                                            is_downloaded: false,
                                        });
                                    }
                                }
                                chapters.reverse();
                            }
                        }
                    }
                }
            }
        }

        // Fallback: parse chapters directly from the HTML chapter list.
        if chapters.is_empty() {
            let document = Html::parse_document(&html);
            if let Ok(li_sel) = selector("ul.divide-y li a[href]") {
                for link in document.select(&li_sel) {
                    if let Some(href) = link.value().attr("href") {
                        // Only include links that look like chapter URLs.
                        if href.contains(&novel_id) && href.contains("chapter") {
                            let url = Self::resolve_url(href);
                            let title_text = link.text().collect::<Vec<_>>().join(" ");
                            let title = clean_text(std::iter::once(title_text.as_str()));
                            if title.is_empty() {
                                continue;
                            }
                            let id = Self::chapter_id_from_url(&novel_id, &url);
                            let chapter_number =
                                extract_chapter_number(&title).unwrap_or_else(|| {
                                    fallback_number += 1.0;
                                    fallback_number
                                });
                            // Avoid duplicate chapters.
                            if !chapters.iter().any(|c: &Chapter| c.id == id) {
                                chapters.push(Chapter {
                                    id,
                                    novel_id: novel_id.clone(),
                                    title,
                                    url,
                                    chapter_number,
                                    content: None,
                                    is_downloaded: false,
                                });
                            }
                        }
                    }
                }
            }
        }

        if chapters.is_empty() {
            return Err(SageError::ElementNotFound {
                selector: "NovelBuddy chapters API fetch failed".into(),
            });
        }

        Ok(chapters)
    }

    async fn fetch_chapter_content(&self, chapter_url: &str) -> Result<String, SageError> {
        // Normalize the URL to use the current domain.
        let normalized_url = if chapter_url.contains("novelbuddy.com") {
            chapter_url.replace("novelbuddy.com", "novelbuddy.me")
        } else {
            chapter_url.to_string()
        };
        let mut html = self.fetch_page(&normalized_url).await?;

        html = RE_SCRIPT.replace_all(&html, "").into_owned();
        html = RE_IFRAME.replace_all(&html, "").into_owned();
        html = RE_STYLE.replace_all(&html, "").into_owned();

        let document = Html::parse_document(&html);

        let selectors = [
            "div#chapter-content p",
            "div.chapter-body p",
            "#chapter-content p",
            ".chapter-body p",
            ".chapter-content p",
            "div#chapter-container p",
            "div#chapter-container",
            "div#chapter-content",
            "div.chapter-content",
            "div.novel-tts-content p",
        ];

        for sel_str in &selectors {
            if let Ok(sel) = selector(sel_str) {
                let paragraphs: Vec<String> = document
                    .select(&sel)
                    .map(|el| {
                        let text = clean_text(el.text());
                        decode_html(&text).unwrap_or(text)
                    })
                    .filter(|text| {
                        !text.is_empty() && !text.to_lowercase().contains("thanks for reading on")
                    })
                    .collect();

                if !paragraphs.is_empty() {
                    return Ok(paragraphs.join("\n\n"));
                }
            }
        }

        Err(SageError::ElementNotFound {
            selector: "Chapter content container not found".into(),
        })
    }
}

// ────────────────────────── NovelFire provider ────────────────────────

pub struct NovelFire {
    client: Client,
}

impl NovelFire {
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
            .map_err(SageError::Request)?;

        let status = response.status().as_u16();
        if status == 403 || status == 503 {
            return Err(SageError::CloudflareBlocked {
                url: url.to_string(),
            });
        }
        let body = response.text().await.map_err(SageError::Request)?;
        detect_cloudflare(&body, url)?;
        Ok(body)
    }

    fn novel_id_from_url(url: &str) -> String {
        url.trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("unknown")
            .to_string()
    }

    fn chapter_id_from_url(novel_id: &str, url: &str) -> String {
        let slug = url
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("unknown");
        format!("{novel_id}-{slug}")
    }

    fn resolve_url(href: &str) -> String {
        if href.starts_with("http://") || href.starts_with("https://") {
            href.to_string()
        } else if href.starts_with('/') {
            format!("{NOVELFIRE_BASE}{href}")
        } else {
            format!("{NOVELFIRE_BASE}/{href}")
        }
    }
}

#[async_trait]
impl NovelProvider for NovelFire {
    fn source_id(&self) -> &'static str {
        "novelfire"
    }

    fn base_url(&self) -> &'static str {
        NOVELFIRE_BASE
    }

    async fn search(&self, query: &str) -> Result<Vec<Novel>, SageError> {
        let url = format!("{NOVELFIRE_BASE}/search?keyword={}", urlencoded(query));
        let html = self.fetch_page(&url).await?;
        let document = Html::parse_document(&html);

        let novel_item_sel = selector(".novel-item")?;
        let title_sel = selector(".novel-title")?;
        let link_sel = selector("a")?;
        let img_sel = selector("img")?;

        let mut novels = Vec::new();

        for element in document.select(&novel_item_sel) {
            let inner = Html::parse_fragment(&element.html());

            let link_anchor = match inner.select(&link_sel).next() {
                Some(el) => el,
                None => continue,
            };

            let title = inner
                .select(&title_sel)
                .next()
                .map(|el| clean_text(el.text()))
                .unwrap_or_default();
            if title.is_empty() {
                continue;
            }

            let href = link_anchor.value().attr("href").unwrap_or_default();
            let source_url = Self::resolve_url(href);
            let id = Self::novel_id_from_url(&source_url);

            let author = "Unknown".to_string(); // Author not available in search results

            let cover_url = inner
                .select(&img_sel)
                .next()
                .and_then(|el| {
                    el.value()
                        .attr("data-src")
                        .or_else(|| el.value().attr("src"))
                })
                .unwrap_or_default()
                .to_string();

            let description = "".to_string(); // Description not available in search results

            novels.push(Novel {
                id,
                title,
                author,
                cover_url,
                source_url,
                description,
            });
        }

        Ok(novels)
    }

    async fn fetch_chapters(&self, novel_url: &str) -> Result<Vec<Chapter>, SageError> {
        let chapters_url = format!("{}/chapters", novel_url);
        let html = self.fetch_page(&chapters_url).await?;
        let novel_id = Self::novel_id_from_url(novel_url);

        let document = Html::parse_document(&html);
        let chapter_sel = selector("li > a")?;

        let mut chapters = Vec::new();
        let mut fallback_number: f32 = 0.0;

        for el in document.select(&chapter_sel) {
            let href = match el.value().attr("href") {
                Some(h) if h.contains("/chapter-") => h,
                _ => continue,
            };

            let title = clean_text(el.text());
            let url = Self::resolve_url(href);
            let id = Self::chapter_id_from_url(&novel_id, &url);
            let chapter_number = extract_chapter_number(&title).unwrap_or_else(|| {
                fallback_number += 1.0;
                fallback_number
            });
            chapters.push(Chapter {
                id,
                novel_id: novel_id.clone(),
                title,
                url,
                chapter_number,
                content: None,
                is_downloaded: false,
            });
        }

        if chapters.is_empty() {
            return Err(SageError::ElementNotFound {
                selector: "chapter links".into(),
            });
        }

        Ok(chapters)
    }

    async fn fetch_chapter_content(&self, chapter_url: &str) -> Result<String, SageError> {
        let mut html = self.fetch_page(chapter_url).await?;

        html = RE_SCRIPT.replace_all(&html, "").into_owned();
        html = RE_IFRAME.replace_all(&html, "").into_owned();
        html = RE_STYLE.replace_all(&html, "").into_owned();

        let document = Html::parse_document(&html);

        let selectors = [
            "div#chapter-content p",
            "div.chapter-body p",
            "#chapter-content p",
            ".chapter-body p",
            ".chapter-content p",
            "div#chapter-container p",
            "div#chapter-container",
            "div#chapter-content",
            "div.chapter-content",
            "div.novel-tts-content p",
        ];

        for sel_str in &selectors {
            if let Ok(sel) = selector(sel_str) {
                let paragraphs: Vec<String> = document
                    .select(&sel)
                    .map(|el| {
                        let text = clean_text(el.text());
                        decode_html(&text).unwrap_or(text)
                    })
                    .filter(|text| {
                        !text.is_empty() && !text.to_lowercase().contains("thanks for reading on")
                    })
                    .collect();

                if !paragraphs.is_empty() {
                    return Ok(paragraphs.join("\n\n"));
                }
            }
        }

        Err(SageError::ElementNotFound {
            selector: "Chapter content container not found".into(),
        })
    }
}

// ──────────────────────────── Utilities ────────────────────────────────

fn urlencoded(input: &str) -> String {
    let mut result = String::with_capacity(input.len() * 3);
    for c in input.chars() {
        match c {
            ' ' => result.push('+'),
            c if c.is_ascii_alphanumeric() || "-._~".contains(c) => result.push(c),
            c => {
                // Encode each UTF-8 byte individually for correct percent-encoding.
                let mut buf = [0u8; 4];
                let encoded = c.encode_utf8(&mut buf);
                for byte in encoded.bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    result
}

fn clean_text<'a>(text_iter: impl Iterator<Item = &'a str>) -> String {
    text_iter
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_chapter_number(title: &str) -> Option<f32> {
    let lower = title.to_lowercase();
    for prefix in &["chapter ", "ch. ", "ch "] {
        if let Some(rest) = lower.strip_prefix(prefix)
            && let Some(num) = parse_leading_number(rest) {
                return Some(num);
            }
    }
    if let Some(idx) = lower.find("chapter") {
        let after = &lower[idx + "chapter".len()..];
        let after = after.trim_start_matches([' ', '-', '_', '.', ':']);
        if let Some(num) = parse_leading_number(after) {
            return Some(num);
        }
    }
    None
}

fn parse_leading_number(s: &str) -> Option<f32> {
    let numeric: String = s
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if numeric.is_empty() {
        return None;
    }
    numeric.parse::<f32>().ok()
}
