use primp::{Client, Impersonate};

#[tokio::main]
async fn main() -> Result<(), primp::Error> {
    let client = Client::builder()
        .impersonate(Impersonate::ChromeV144)
        .build()?;
        
    let resp = client.get("https://novelbuddy.com/search?q=shadow").send().await?;
    println!("Status: {}", resp.status());
    let text = resp.text().await?;
    println!("Body preview (first 500 chars): {}", &text[..std::cmp::min(500, text.len())]);
    if text.contains("cf-browser-verification") || text.contains("cf_chl_opt") || text.contains("Just a moment...") {
        println!("CLOUDFLARE DETECTED");
    } else if text.contains("book-item") {
        println!("BOOK-ITEM FOUND");
    } else {
        println!("NO BOOK-ITEM FOUND");
    }
    
    Ok(())
}
