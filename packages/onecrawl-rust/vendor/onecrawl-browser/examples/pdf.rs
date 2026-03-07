use onecrawl_browser::browser::{Browser, BrowserConfig};
use onecrawl_protocol::cdp::browser_protocol::page::PrintToPdfParams;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let (browser, mut handler) = Browser::launch(BrowserConfig::builder().build()?).await?;

    let handle = tokio::spawn(async move {
        loop {
            let _ = handler.next().await.unwrap();
        }
    });

    let page = browser.new_page("https://news.ycombinator.com/").await?;

    // save the page as pdf
    page.save_pdf(PrintToPdfParams::default(), "hn.pdf").await?;

    handle.await?;
    Ok(())
}
