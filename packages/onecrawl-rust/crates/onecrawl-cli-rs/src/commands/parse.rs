use clap::Subcommand;

#[derive(Subcommand)]
pub enum ParseAction {
    /// Parse HTML and get accessibility tree
    A11y {
        /// HTML file path or stdin
        #[arg(short, long)]
        file: Option<String>,
        /// Interactive elements only
        #[arg(short, long)]
        interactive: bool,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
    /// Query HTML with CSS selector
    Query {
        /// CSS selector
        selector: String,
        /// HTML file path
        #[arg(short, long)]
        file: Option<String>,
    },
    /// Extract text from HTML
    Text {
        /// CSS selector
        selector: String,
        /// HTML file path
        #[arg(short, long)]
        file: Option<String>,
    },
    /// Extract links from HTML
    Links {
        /// HTML file path
        #[arg(short, long)]
        file: Option<String>,
    },
}

pub fn handle(action: ParseAction) {
    match action {
        ParseAction::A11y {
            file,
            interactive,
            json,
        } => {
            let html = read_input(file);
            match onecrawl_parser::get_accessibility_tree(&html) {
                Ok(tree) => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&tree).unwrap());
                    } else {
                        print!(
                            "{}",
                            onecrawl_parser::accessibility::render_tree(&tree, 0, interactive)
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        ParseAction::Query { selector, file } => {
            let html = read_input(file);
            match onecrawl_parser::query_selector(&html, &selector) {
                Ok(elements) => {
                    println!("{}", serde_json::to_string_pretty(&elements).unwrap());
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        ParseAction::Text { selector, file } => {
            let html = read_input(file);
            match onecrawl_parser::extract_text(&html, &selector) {
                Ok(texts) => {
                    for text in texts {
                        println!("{text}");
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        ParseAction::Links { file } => {
            let html = read_input(file);
            match onecrawl_parser::extract::extract_links(&html) {
                Ok(links) => {
                    for (href, text) in links {
                        println!("{href}\t{text}");
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
    }
}

fn read_input(file: Option<String>) -> String {
    match file {
        Some(path) => std::fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("Cannot read {path}: {e}");
            std::process::exit(1);
        }),
        None => {
            use std::io::Read;
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input).unwrap();
            input
        }
    }
}
