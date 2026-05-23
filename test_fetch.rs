use reqwest_impersonate as reqwest;
use scraper::{Html, Selector};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .impersonate(reqwest::impersonate::Impersonate::Chrome114)
        .build()?;
    let res = client.get("https://novelbuddy.com/pursuit-of-the-truth/chapter-1432-i-am-xu-hui").send().await?;
    let text = res.text().await?;
    std::fs::write("chapter.html", &text)?;
    println!("Saved to chapter.html");
    Ok(())
}
