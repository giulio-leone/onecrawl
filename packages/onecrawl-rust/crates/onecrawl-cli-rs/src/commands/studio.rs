use crate::cli::StudioAction;
use onecrawl_cdp::studio::StudioWorkspace;

const WORKSPACE_DIR: &str = "~/.onecrawl/studio";

pub async fn handle(action: StudioAction) {
    match action {
        StudioAction::Open { port } => open(port).await,
        StudioAction::Templates => templates(),
        StudioAction::Projects => projects(),
        StudioAction::Export { project_id, output } => export(&project_id, output.as_deref()),
        StudioAction::Import { file, name } => import(&file, name.as_deref()),
        StudioAction::Validate { file } => validate(&file),
    }
}

async fn open(port: u16) {
    println!("🕷️  Starting OneCrawl Studio on port {port}...");
    println!("   Open http://localhost:{port}/studio in your browser");
    println!();

    // Try to open browser
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg(format!("http://localhost:{port}/studio"))
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(format!("http://localhost:{port}/studio"))
            .spawn();
    }

    if let Err(e) = onecrawl_server::serve::start_server(port).await {
        eprintln!("❌ Server error: {e}");
    }
}

fn templates() {
    let templates = StudioWorkspace::templates();
    println!("Available templates ({}):", templates.len());
    println!();
    for t in &templates {
        println!("  {} — {}", t.name, t.description);
        println!("    ID: {}  Category: {}  Tags: {}",
            t.id, t.category, t.tags.join(", "));
        println!();
    }
}

fn projects() {
    let ws = match StudioWorkspace::new(WORKSPACE_DIR) {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("❌ Failed to open workspace: {e}");
            return;
        }
    };

    match ws.list_projects() {
        Ok(projects) => {
            if projects.is_empty() {
                println!("No projects yet.");
                println!();
                println!("Create a project:  onecrawl studio open");
                println!("Import a workflow: onecrawl studio import <file.json>");
                return;
            }
            println!("Saved projects ({}):", projects.len());
            println!();
            for p in &projects {
                println!("  {} — {}", p.name, p.id);
                if let Some(desc) = &p.description {
                    println!("    {desc}");
                }
                println!("    Updated: {}  Runs: {}", p.updated_at, p.run_count);
                println!();
            }
        }
        Err(e) => eprintln!("❌ Failed to list projects: {e}"),
    }
}

fn export(id: &str, output: Option<&str>) {
    let ws = match StudioWorkspace::new(WORKSPACE_DIR) {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("❌ Failed to open workspace: {e}");
            return;
        }
    };

    match ws.export_workflow(id) {
        Ok(json) => {
            if let Some(path) = output {
                match std::fs::write(path, &json) {
                    Ok(()) => println!("✅ Exported to {path}"),
                    Err(e) => eprintln!("❌ Failed to write file: {e}"),
                }
            } else {
                println!("{json}");
            }
        }
        Err(e) => eprintln!("❌ Failed to export: {e}"),
    }
}

fn import(file: &str, name: Option<&str>) {
    let ws = match StudioWorkspace::new(WORKSPACE_DIR) {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("❌ Failed to open workspace: {e}");
            return;
        }
    };

    let json = match std::fs::read_to_string(file) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("❌ Failed to read file: {e}");
            return;
        }
    };

    let project_name = name.unwrap_or_else(|| {
        std::path::Path::new(file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("imported")
    });

    match ws.import_workflow(project_name, &json) {
        Ok(project) => {
            println!("✅ Imported as '{}' (ID: {})", project.name, project.id);
        }
        Err(e) => eprintln!("❌ Failed to import: {e}"),
    }
}

fn validate(file: &str) {
    let json = match std::fs::read_to_string(file) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("❌ Failed to read file: {e}");
            return;
        }
    };

    let workflow: serde_json::Value = match serde_json::from_str(&json) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("❌ Invalid JSON: {e}");
            return;
        }
    };

    match StudioWorkspace::validate_workflow(&workflow) {
        Ok(warnings) => {
            if warnings.is_empty() {
                println!("✅ Workflow is valid");
            } else {
                println!("⚠️  Workflow has warnings:");
                for w in &warnings {
                    println!("   - {w}");
                }
            }
        }
        Err(e) => eprintln!("❌ Validation error: {e}"),
    }
}
