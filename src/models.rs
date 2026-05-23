//! Domain models for the Sage web novel reader.

/// Represents a web novel sourced from an online platform.
#[derive(Debug, Clone)]
pub struct Novel {
    /// Unique identifier for the novel (typically derived from the source URL).
    pub id: String,
    /// The title of the novel.
    pub title: String,
    /// The author or translator of the novel.
    pub author: String,
    /// URL pointing to the novel's cover image.
    pub cover_url: String,
    /// The canonical URL of the novel on its source website.
    pub source_url: String,
    /// A synopsis or summary of the novel.
    pub description: String,
}

/// Represents a single chapter within a novel.
#[derive(Debug, Clone)]
pub struct Chapter {
    /// Unique identifier for the chapter.
    pub id: String,
    /// The ID of the parent novel this chapter belongs to.
    pub novel_id: String,
    /// The display title of the chapter.
    pub title: String,
    /// The URL where this chapter's content can be fetched.
    pub url: String,
    /// The chapter number (f32 to support sub-chapters like 10.5).
    pub chapter_number: f32,
    /// The full text content of the chapter, if it has been downloaded.
    pub content: Option<String>,
    /// Whether the chapter content has been downloaded and cached locally.
    pub is_downloaded: bool,
}

/// Tracks the user's reading progress within a specific novel.
#[derive(Debug, Clone)]
pub struct Progress {
    /// The ID of the novel being read.
    pub novel_id: String,
    /// The ID of the chapter the user was last reading.
    pub chapter_id: String,
    /// The vertical scroll offset within the chapter content.
    pub scroll_offset: usize,
}
