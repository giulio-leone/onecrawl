use colored::Colorize;
use onecrawl_cdp::skills::{SkillManifest, SkillRegistry};

pub fn skills_list() {
    let builtins = SkillRegistry::builtins();

    println!("{}", "Built-in Skills".bold().cyan());
    println!("{}", "─".repeat(50));
    for skill in &builtins {
        print_skill_row(skill, "built-in");
    }
    println!();
    println!(
        "{}",
        "Use `onecrawl skills discover <path>` to find additional skill packages."
            .dimmed()
    );
}

pub fn skills_info(name: &str) {
    // Search built-ins first
    let builtins = SkillRegistry::builtins();
    if let Some(skill) = builtins.iter().find(|s| s.name == name) {
        print_skill_detail(skill, "built-in");
        return;
    }

    eprintln!("{} skill not found: {name}", "✗".red());
    eprintln!(
        "{}",
        "Tip: use `onecrawl skills list` to see available skills.".dimmed()
    );
    std::process::exit(1);
}

pub fn skills_discover(path: &str) {
    let dir = std::path::Path::new(path);
    if !dir.is_dir() {
        eprintln!("{} not a directory: {path}", "✗".red());
        std::process::exit(1);
    }

    match SkillRegistry::discover(dir) {
        Ok(registry) => {
            let skills = registry.list();
            if skills.is_empty() {
                println!(
                    "{} No skill packages found in {path}",
                    "⚠".yellow()
                );
                println!(
                    "{}",
                    "A skill package needs a skill.json file in its directory.".dimmed()
                );
                return;
            }

            println!(
                "{} {} skill(s) in {}",
                "✓".green(),
                skills.len(),
                path
            );
            println!("{}", "─".repeat(50));
            for loaded in skills {
                print_skill_row(&loaded.manifest, &loaded.path.display().to_string());
            }
        }
        Err(e) => {
            eprintln!("{} discovery failed: {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

fn print_skill_row(skill: &SkillManifest, source: &str) {
    println!(
        "  {} {} {} {}",
        skill.name.bold(),
        format!("v{}", skill.version).dimmed(),
        format!("({})", source).dimmed(),
        format!("— {}", skill.description).white(),
    );
    if !skill.tools.is_empty() {
        let tool_names: Vec<&str> = skill.tools.iter().map(|t| t.name.as_str()).collect();
        println!(
            "    {} {}",
            "tools:".dimmed(),
            tool_names.join(", ").dimmed()
        );
    }
}

fn print_skill_detail(skill: &SkillManifest, source: &str) {
    println!("{} {}", "Skill:".bold(), skill.name.bold().cyan());
    println!("{} {}", "Version:".bold(), skill.version);
    println!("{} {}", "Source:".bold(), source);
    println!("{} {}", "Description:".bold(), skill.description);

    if let Some(author) = &skill.author {
        println!("{} {author}", "Author:".bold());
    }

    if !skill.requires.is_empty() {
        println!("{} {}", "Requires:".bold(), skill.requires.join(", "));
    }

    if !skill.tools.is_empty() {
        println!();
        println!("{}", "Tools:".bold());
        for tool in &skill.tools {
            println!("  {} — {}", tool.name.green(), tool.description);
            if let Some(cmd) = &tool.command {
                println!("    {} {cmd}", "command:".dimmed());
            }
            if tool.script.is_some() {
                println!("    {} (inline JS)", "script:".dimmed());
            }
        }
    }
}
