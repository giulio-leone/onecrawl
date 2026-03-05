use colored::Colorize;
use onecrawl_cdp::Page;

/// Run a browser command that needs the BrowserSession (e.g. tab management).

pub async fn with_page<F, Fut>(f: F)
where
    F: FnOnce(Page) -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    let (_session, page) = match super::super::session::connect_to_session().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    };
    if let Err(e) = f(page).await {
        eprintln!("{} {e}", "✗".red());
        std::process::exit(1);
    }
}

pub async fn with_session<F, Fut>(f: F)
where
    F: FnOnce(onecrawl_cdp::BrowserSession, Page) -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    let (session, page) = match super::super::session::connect_to_session().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    };
    if let Err(e) = f(session, page).await {
        eprintln!("{} {e}", "✗".red());
        std::process::exit(1);
    }
}
