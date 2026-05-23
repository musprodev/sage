use std::fs;
use std::fs::File;
use std::path::PathBuf;

use epub_builder::{EpubBuilder, EpubContent, ReferenceType, ZipLibrary};

use crate::db::Database;
use crate::error::SageError;

/// Compiles a novel and its downloaded chapters into an EPUB file in the user's Downloads directory.
pub fn export_to_epub(db: &Database, novel_id: &str, export_dir: &std::path::Path) -> Result<PathBuf, SageError> {
    let novel = db.get_novel(novel_id)?
        .ok_or_else(|| SageError::ScrapingError(format!("Novel '{}' not found.", novel_id)))?;

    let chapters = db.get_novel_chapters(novel_id)?;
    let downloaded_chapters: Vec<_> = chapters.into_iter().filter(|c| c.is_downloaded).collect();

    if downloaded_chapters.is_empty() {
        return Err(SageError::ScrapingError("No downloaded chapters to export.".into()));
    }

    let mut epub = EpubBuilder::new(ZipLibrary::new().map_err(|e| SageError::ScrapingError(e.to_string()))?)
        .map_err(|e| SageError::ScrapingError(e.to_string()))?;

    epub.metadata("title", novel.title.clone())
        .map_err(|e| SageError::ScrapingError(e.to_string()))?;
    epub.metadata("author", novel.author.clone())
        .map_err(|e| SageError::ScrapingError(e.to_string()))?;

    // Optionally add a cover image here if we fetch it.

    for (i, ch) in downloaded_chapters.into_iter().enumerate() {
        let title = ch.title.clone();
        let content = ch.content.unwrap_or_default();
        
        let mut html = format!("<h1>{}</h1>\n", title);
        for p in content.split("\n\n") {
            let p = p.trim();
            if !p.is_empty() {
                html.push_str(&format!("<p>{}</p>\n", p));
            }
        }

        let xhtml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <html xmlns=\"http://www.w3.org/1999/xhtml\">\n\
             <head><title>{}</title></head>\n\
             <body>\n{}\n</body>\n\
             </html>",
            title, html
        );

        let filename = format!("chapter_{:04}.xhtml", i);
        epub.add_content(
            EpubContent::new(&filename, xhtml.as_bytes())
                .title(title)
                .reftype(ReferenceType::Text),
        ).map_err(|e| SageError::ScrapingError(e.to_string()))?;
    }


    
    // Sanitize title for filename
    let safe_title = novel.title.replace(|c: char| !c.is_alphanumeric() && c != ' ', "_");
    let output_path = export_dir.join(format!("{}.epub", safe_title.trim()));

    let mut out_file = File::create(&output_path).map_err(|e| SageError::ScrapingError(e.to_string()))?;
    epub.generate(&mut out_file).map_err(|e| SageError::ScrapingError(e.to_string()))?;

    Ok(output_path)
}
