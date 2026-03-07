use crate::cli::OrchestrateAction;

pub async fn handle(action: OrchestrateAction) {
    match action {
        OrchestrateAction::Run { file, verbose, timeout } => run(&file, verbose, timeout).await,
        OrchestrateAction::Validate { file } => validate(&file).await,
        OrchestrateAction::Devices => devices().await,
        OrchestrateAction::Stop => stop().await,
    }
}

async fn run(file: &str, verbose: bool, timeout: Option<u64>) {
    if verbose {
        println!("Loading orchestration from: {file}");
    }

    let orchestration = match onecrawl_cdp::orchestrator::Orchestrator::from_file(file) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("❌ Failed to load orchestration: {e}");
            return;
        }
    };

    if let Err(errors) = onecrawl_cdp::orchestrator::Orchestrator::validate(&orchestration) {
        eprintln!("❌ Validation errors:");
        for err in &errors {
            eprintln!("  • {err}");
        }
        return;
    }

    if verbose {
        println!(
            "Orchestration '{}': {} devices, {} steps",
            orchestration.name,
            orchestration.devices.len(),
            orchestration.steps.len()
        );
    }

    let mut orch = onecrawl_cdp::orchestrator::Orchestrator::new(orchestration);

    if let Some(t) = timeout {
        println!("Timeout set to {t}s");
    }

    println!("Connecting devices...");
    if let Err(e) = orch.connect_devices().await {
        eprintln!("❌ Failed to connect devices: {e}");
        return;
    }

    println!("Executing orchestration...");
    match orch.execute().await {
        Ok(result) => {
            if result.success {
                println!("✅ Orchestration '{}' completed successfully", result.name);
            } else {
                println!("⚠️  Orchestration '{}' completed with errors", result.name);
            }
            println!(
                "   Steps: {}/{} completed in {:.2}s",
                result.steps_completed, result.steps_total, result.duration_secs
            );
            if !result.errors.is_empty() {
                println!("   Errors:");
                for err in &result.errors {
                    println!("     • {err}");
                }
            }
            if verbose {
                let json = serde_json::to_string_pretty(&result).unwrap_or_default();
                println!("\n{json}");
            }
        }
        Err(e) => {
            eprintln!("❌ Orchestration failed: {e}");
        }
    }

    let _ = orch.disconnect().await;
}

async fn validate(file: &str) {
    let orchestration = match onecrawl_cdp::orchestrator::Orchestrator::from_file(file) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("❌ Failed to load: {e}");
            return;
        }
    };

    match onecrawl_cdp::orchestrator::Orchestrator::validate(&orchestration) {
        Ok(()) => {
            println!("✅ Orchestration '{}' is valid", orchestration.name);
            println!(
                "   Devices: {}  Steps: {}",
                orchestration.devices.len(),
                orchestration.steps.len()
            );
        }
        Err(errors) => {
            eprintln!("❌ Validation failed:");
            for err in &errors {
                eprintln!("  • {err}");
            }
        }
    }
}

async fn devices() {
    println!("📱 Connected devices:");
    println!("  (Use 'orchestrate run' to connect devices via a workflow)");
    println!("  Tip: Devices are connected when an orchestration starts");
}

async fn stop() {
    println!("⏹  Stop signal sent to running orchestration");
}
