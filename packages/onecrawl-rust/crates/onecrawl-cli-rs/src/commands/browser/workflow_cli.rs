use colored::Colorize;
use super::helpers::with_page;

pub async fn workflow_exec(file: &str) {
    let data = match std::fs::read_to_string(file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{} failed to read {file}: {e}", "✗".red());
            std::process::exit(1);
        }
    };
    let workflow = match onecrawl_cdp::workflow::parse_json(&data) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("{} invalid workflow: {e}", "✗".red());
            std::process::exit(1);
        }
    };
    let errors = onecrawl_cdp::workflow::validate(&workflow);
    if !errors.is_empty() {
        eprintln!("{} workflow validation failed:", "✗".red());
        for e in &errors {
            eprintln!("  - {e}");
        }
        std::process::exit(1);
    }
    println!(
        "{} Workflow '{}' loaded ({} steps)",
        "▶".cyan(),
        workflow.name,
        workflow.steps.len()
    );
    with_page(|page| async move {
        let result = onecrawl_cdp::workflow::execute_workflow(&page, &workflow)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn workflow_validate_cli(file: &str) {
    let data = match std::fs::read_to_string(file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{} failed to read {file}: {e}", "✗".red());
            std::process::exit(1);
        }
    };
    let workflow = match onecrawl_cdp::workflow::parse_json(&data) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("{} parse error: {e}", "✗".red());
            std::process::exit(1);
        }
    };
    let errors = onecrawl_cdp::workflow::validate(&workflow);
    if errors.is_empty() {
        println!(
            "{} Workflow '{}' is valid ({} steps, {} variables)",
            "✓".green(),
            workflow.name,
            workflow.steps.len(),
            workflow.variables.len()
        );
    } else {
        eprintln!("{} Validation errors:", "✗".red());
        for e in &errors {
            eprintln!("  - {e}");
        }
        std::process::exit(1);
    }
}
