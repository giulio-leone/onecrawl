//! Interactive scraping REPL shell — command parser and history.
//!
//! Provides [`ShellCommand`] parsing and [`ShellHistory`] persistence for the
//! OneCrawl interactive CLI experience.  The actual REPL loop lives in the CLI
//! crate; this module is the parsing/storage back-end.

use onecrawl_core::Result;
use serde::{Deserialize, Serialize};

/// A parsed shell command with its raw input, command name, arguments and timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellCommand {
    pub raw: String,
    pub command: String,
    pub args: Vec<String>,
    pub timestamp: f64,
}

/// Ring-buffer style command history with a configurable maximum size.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellHistory {
    pub commands: Vec<ShellCommand>,
    pub max_size: usize,
}

impl ShellHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            commands: Vec::new(),
            max_size,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn add(&mut self, cmd: ShellCommand) {
        if self.max_size == 0 {
            return;
        }
        if self.commands.len() >= self.max_size {
            self.commands.remove(0);
        }
        self.commands.push(cmd);
    }

    pub fn last(&self) -> Option<&ShellCommand> {
        self.commands.last()
    }

    pub fn search(&self, pattern: &str) -> Vec<&ShellCommand> {
        self.commands
            .iter()
            .filter(|c| c.raw.contains(pattern))
            .collect()
    }
}

/// Parse a shell command string into parts.
pub fn parse_command(input: &str) -> ShellCommand {
    let trimmed = input.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let command = parts.first().map(|s| s.to_string()).unwrap_or_default();
    let args = parts.iter().skip(1).map(|s| s.to_string()).collect();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1000.0;

    ShellCommand {
        raw: trimmed.to_string(),
        command,
        args,
        timestamp,
    }
}

/// Get available shell commands and their descriptions.
pub fn available_commands() -> Vec<(String, String)> {
    vec![
        ("goto <url>".to_string(), "Navigate to URL".to_string()),
        (
            "select <css>".to_string(),
            "Query selector, return matches".to_string(),
        ),
        ("xpath <expr>".to_string(), "XPath query".to_string()),
        (
            "text [selector]".to_string(),
            "Extract text content".to_string(),
        ),
        (
            "html [selector]".to_string(),
            "Extract innerHTML".to_string(),
        ),
        (
            "attr <selector> <name>".to_string(),
            "Get attribute value".to_string(),
        ),
        ("click <selector>".to_string(), "Click element".to_string()),
        (
            "type <selector> <text>".to_string(),
            "Type into input".to_string(),
        ),
        (
            "screenshot [path]".to_string(),
            "Take screenshot".to_string(),
        ),
        ("eval <js>".to_string(), "Evaluate JavaScript".to_string()),
        ("cookies".to_string(), "Show current cookies".to_string()),
        ("links".to_string(), "List all links on page".to_string()),
        ("title".to_string(), "Show page title".to_string()),
        ("url".to_string(), "Show current URL".to_string()),
        ("back".to_string(), "Go back in history".to_string()),
        ("forward".to_string(), "Go forward in history".to_string()),
        ("reload".to_string(), "Reload current page".to_string()),
        (
            "wait <ms>".to_string(),
            "Wait specified milliseconds".to_string(),
        ),
        ("stealth".to_string(), "Apply stealth patches".to_string()),
        ("history".to_string(), "Show command history".to_string()),
        (
            "export <path>".to_string(),
            "Export page content to file".to_string(),
        ),
        ("help".to_string(), "Show available commands".to_string()),
        ("exit".to_string(), "Exit shell".to_string()),
    ]
}

/// Save shell history to a JSON file.
pub fn save_history(history: &ShellHistory, path: &std::path::Path) -> Result<()> {
    let json = serde_json::to_string_pretty(history)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load shell history from a JSON file.
pub fn load_history(path: &std::path::Path) -> Result<ShellHistory> {
    let json = std::fs::read_to_string(path)?;
    let history: ShellHistory = serde_json::from_str(&json)?;
    Ok(history)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_basic() {
        let cmd = parse_command("goto");
        assert_eq!(cmd.command, "goto");
        assert!(cmd.args.is_empty());
        assert_eq!(cmd.raw, "goto");
    }

    #[test]
    fn test_parse_command_with_args() {
        let cmd = parse_command("goto https://example.com --verbose");
        assert_eq!(cmd.command, "goto");
        assert_eq!(cmd.args, vec!["https://example.com", "--verbose"]);
    }

    #[test]
    fn test_parse_command_trims_whitespace() {
        let cmd = parse_command("  select  .my-class  ");
        assert_eq!(cmd.command, "select");
        assert_eq!(cmd.args, vec![".my-class"]);
        assert_eq!(cmd.raw, "select  .my-class");
    }

    #[test]
    fn test_parse_command_empty() {
        let cmd = parse_command("");
        assert_eq!(cmd.command, "");
        assert!(cmd.args.is_empty());
    }

    #[test]
    fn test_shell_history_add_and_search() {
        let mut history = ShellHistory::new(100);
        assert!(history.is_empty());

        history.add(parse_command("goto https://a.com"));
        history.add(parse_command("select div"));
        history.add(parse_command("goto https://b.com"));

        assert_eq!(history.commands.len(), 3);
        assert_eq!(history.last().unwrap().command, "goto");

        let results = history.search("goto");
        assert_eq!(results.len(), 2);

        let results = history.search("select");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_shell_history_eviction() {
        let mut history = ShellHistory::new(2);
        history.add(parse_command("first"));
        history.add(parse_command("second"));
        history.add(parse_command("third"));
        assert_eq!(history.commands.len(), 2);
        assert_eq!(history.commands[0].command, "second");
        assert_eq!(history.commands[1].command, "third");
    }

    #[test]
    fn test_available_commands_non_empty() {
        let cmds = available_commands();
        assert!(cmds.len() >= 10);
        let names: Vec<&str> = cmds.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.iter().any(|n| n.starts_with("goto")));
        assert!(names.iter().any(|n| n.starts_with("help")));
        assert!(names.iter().any(|n| n.starts_with("exit")));
    }
}
