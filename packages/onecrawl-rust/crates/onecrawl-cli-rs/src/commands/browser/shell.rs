use colored::Colorize;

// ---------------------------------------------------------------------------
// Interactive Shell
// ---------------------------------------------------------------------------

pub async fn shell_repl() {
    use std::io::{self, BufRead, Write};

    let mut history = onecrawl_cdp::shell::ShellHistory::new(500);
    let commands = onecrawl_cdp::shell::available_commands();

    println!("{} OneCrawl Interactive Shell", "▶".green());
    println!(
        "  Type {} for commands, {} to quit.\n",
        "help".cyan(),
        "exit".cyan()
    );

    loop {
        print!("{} ", "onecrawl>".green());
        io::stdout().flush().ok();

        let mut line = String::new();
        if io::stdin().lock().read_line(&mut line).is_err() || line.is_empty() {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let cmd = onecrawl_cdp::shell::parse_command(trimmed);
        history.add(cmd.clone());

        match cmd.command.as_str() {
            "exit" | "quit" => {
                println!("{} Bye!", "✓".green());
                break;
            }
            "help" => {
                for (name, desc) in &commands {
                    println!("  {:<28} {}", name.cyan(), desc);
                }
            }
            "history" => {
                for (i, c) in history.commands.iter().enumerate() {
                    println!("  {:>4}  {}", i + 1, c.raw);
                }
            }
            other => {
                println!(
                    "{} Command '{}' would be dispatched to the browser session",
                    "→".yellow(),
                    other
                );
            }
        }
    }
}
