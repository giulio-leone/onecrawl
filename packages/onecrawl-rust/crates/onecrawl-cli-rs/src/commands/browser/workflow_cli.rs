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


pub async fn workflow_resume_cli(file: &str, resume_from: usize, choice: &str, reasoning: Option<&str>) {
    use crate::commands::browser::helpers::with_page;
    with_page(|page| async move {
        let data = std::fs::read_to_string(file)
            .map_err(|e| format!("failed to read {file}: {e}"))?;
        let workflow = onecrawl_cdp::workflow::parse_json(&data)
            .map_err(|e| format!("parse error: {e}"))?;

        if resume_from >= workflow.steps.len() {
            return Err(format!("resume_from {} out of range (workflow has {} steps)",
                resume_from, workflow.steps.len()));
        }

        let mut variables = workflow.variables.clone();
        variables.insert("__agent_choice".into(), serde_json::json!(choice));
        if let Some(r) = reasoning {
            variables.insert("__agent_reasoning".into(), serde_json::json!(r));
        }

        println!("{} Resuming workflow \'{}\' from step {}", "▶".green(), workflow.name, resume_from + 1);
        println!("  Agent chose: {}", choice);

        let result = onecrawl_cdp::workflow::execute_workflow(&page, &workflow).await
            .map_err(|e| format!("workflow error: {e}"))?;
        println!("{} Workflow completed: {:?}", "✓".green(), result.status);
        println!("  Steps: {} succeeded, {} failed, {} skipped",
            result.steps_succeeded, result.steps_failed, result.steps_skipped);
        let json = serde_json::to_string_pretty(&result).unwrap_or_default();
        println!("{json}");
        Ok(())
    }).await;
}

pub async fn agent_decide_cli(prompt: &str, options: Option<&str>) {
    println!("{} Agent Decision Required", "🤖".yellow());
    println!("  Prompt: {}", prompt);
    if let Some(opts) = options {
        println!("  Options:");
        for (i, opt) in opts.split(',').enumerate() {
            println!("    {}. {}", i + 1, opt.trim());
        }
    }
    let response = serde_json::json!({
        "prompt": prompt,
        "options": options.map(|o| o.split(',').map(|s| s.trim()).collect::<Vec<_>>()),
        "awaiting_decision": true,
    });
    println!("{}", serde_json::to_string_pretty(&response).unwrap_or_default());
}
