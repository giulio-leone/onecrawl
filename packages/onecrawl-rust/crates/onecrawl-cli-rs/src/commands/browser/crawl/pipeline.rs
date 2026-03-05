use colored::Colorize;

// Rate Limiter (standalone — no Page required)
// Retry Queue (standalone — no Page required)
// Task Scheduler (standalone — no Page required)
// Session Pool (standalone — no Page required)

pub fn pipeline_run(pipeline_path: &str, data_path: &str, output: Option<&str>, format: &str) {
    let pipeline =
        match onecrawl_cdp::data_pipeline::load_pipeline(std::path::Path::new(pipeline_path)) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{} Failed to load pipeline: {e}", "✗".red());
                std::process::exit(1);
            }
        };

    let data_str = match std::fs::read_to_string(data_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{} Failed to read data: {e}", "✗".red());
            std::process::exit(1);
        }
    };

    let items: Vec<std::collections::HashMap<String, String>> =
        match serde_json::from_str(&data_str) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{} Invalid data JSON: {e}", "✗".red());
                std::process::exit(1);
            }
        };

    let result = onecrawl_cdp::data_pipeline::execute_pipeline(&pipeline, items);
    println!(
        "{} Pipeline '{}': {} → {} items ({} filtered, {} deduplicated)",
        "✓".green(),
        pipeline.name,
        result.input_count,
        result.output_count,
        result.filtered_count,
        result.deduplicated_count,
    );
    for err in &result.errors {
        eprintln!("  {} {err}", "⚠".yellow());
    }

    if let Some(out) = output {
        match onecrawl_cdp::data_pipeline::export_processed(
            &result,
            std::path::Path::new(out),
            format,
        ) {
            Ok(n) => println!("{} Exported {n} items to {}", "✓".green(), out.cyan()),
            Err(e) => {
                eprintln!("{} Export failed: {e}", "✗".red());
                std::process::exit(1);
            }
        }
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    }
}

pub fn pipeline_validate(pipeline_path: &str) {
    let pipeline =
        match onecrawl_cdp::data_pipeline::load_pipeline(std::path::Path::new(pipeline_path)) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{} Failed to load pipeline: {e}", "✗".red());
                std::process::exit(1);
            }
        };

    let errors = onecrawl_cdp::data_pipeline::validate_pipeline(&pipeline);
    if errors.is_empty() {
        println!("{} Pipeline '{}' is valid", "✓".green(), pipeline.name);
    } else {
        eprintln!(
            "{} Pipeline '{}' has {} error(s):",
            "✗".red(),
            pipeline.name,
            errors.len()
        );
        for err in &errors {
            eprintln!("  - {err}");
        }
        std::process::exit(1);
    }
}

pub fn pipeline_save_file(pipeline_json: &str, path: &str) {
    let pipeline: onecrawl_cdp::Pipeline = match serde_json::from_str(pipeline_json) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} Invalid pipeline JSON: {e}", "✗".red());
            std::process::exit(1);
        }
    };
    match onecrawl_cdp::data_pipeline::save_pipeline(&pipeline, std::path::Path::new(path)) {
        Ok(()) => println!("{} Pipeline saved to {}", "✓".green(), path.cyan()),
        Err(e) => {
            eprintln!("{} Save failed: {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

pub fn pipeline_load_file(path: &str) {
    match onecrawl_cdp::data_pipeline::load_pipeline(std::path::Path::new(path)) {
        Ok(pipeline) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&pipeline).unwrap_or_default()
            );
        }
        Err(e) => {
            eprintln!("{} Failed to load pipeline: {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

