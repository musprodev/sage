use primp::{Client, Impersonate};

#[tokio::main]
async fn main() -> Result<(), primp::Error> {
    let client = Client::builder()
        .impersonate(Impersonate::ChromeV144)
        .build()?;
        
    let resp = client.get("https://novelbuddy.com/api/manga/VYPGVZ8z/chapters").send().await?;
    let text = resp.text().await?;
    println!("Resp len: {}", text.len());
    Ok(())
}
