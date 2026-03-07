use colored::Colorize;
use super::helpers::with_page;

pub async fn agent_loop(goal: &str, max_steps: usize, verify_js: Option<&str>) {
    let goal = goal.to_string();
    let verify = verify_js.map(String::from);
    with_page(|page| async move {
        match onecrawl_cdp::agent::agent_loop(&page, &goal, max_steps, verify.as_deref()).await {
            Ok(result) => {
                let status = result["status"].as_str().unwrap_or("unknown");
                let steps = result["total_steps"].as_u64().unwrap_or(0);
                if status == "goal_achieved" {
                    println!("{} Goal achieved in {} steps", "✓".green(), steps.to_string().cyan());
                } else {
                    println!("{} Max steps reached ({})", "⚠".yellow(), steps);
                }
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }).await;
}

pub async fn goal_assert(assertion_type: &str, value: &str) {
    let at = assertion_type.to_string();
    let v = value.to_string();
    with_page(|page| async move {
        let assertions = vec![(at.as_str(), v.as_str())];
        match onecrawl_cdp::agent::goal_assert(&page, &assertions).await {
            Ok(result) => {
                let passed = result["all_passed"].as_bool().unwrap_or(false);
                if passed {
                    println!("{} Assertion passed", "✓".green());
                } else {
                    println!("{} Assertion failed", "✗".red());
                }
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }).await;
}

pub async fn annotated_observe() {
    with_page(|page| async move {
        match onecrawl_cdp::agent::annotated_observe(&page).await {
            Ok(result) => {
                let count = result["element_count"].as_u64().unwrap_or(0);
                println!("{} {} interactive elements found", "✓".green(), count.to_string().cyan());
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }).await;
}

pub async fn session_context(command: &str, key: Option<&str>, value: Option<&str>) {
    let cmd = command.to_string();
    let k = key.map(String::from);
    let v = value.map(String::from);
    with_page(|page| async move {
        match onecrawl_cdp::agent::session_context(&page, &cmd, k.as_deref(), v.as_deref()).await {
            Ok(result) => {
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }).await;
}

pub async fn auto_chain(actions: &[String], on_error: &str, max_retries: usize) {
    let acts = actions.to_vec();
    let oe = on_error.to_string();
    with_page(|page| async move {
        match onecrawl_cdp::agent::auto_chain(&page, &acts, &oe, max_retries).await {
            Ok(result) => {
                let status = result["status"].as_str().unwrap_or("unknown");
                println!("{} Chain: {}", if status == "all_success" { "✓".green() } else { "⚠".yellow() }, status);
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }).await;
}

pub async fn think() {
    with_page(|page| async move {
        match onecrawl_cdp::agent::think(&page).await {
            Ok(result) => {
                let page_type = result["page_type"].as_str().unwrap_or("unknown");
                println!("🧠 Page type: {}", page_type.cyan());
                if let Some(recs) = result["recommendations"].as_array() {
                    for rec in recs {
                        let action = rec["action"].as_str().unwrap_or("");
                        let priority = rec["priority"].as_str().unwrap_or("");
                        let reason = rec["reason"].as_str().unwrap_or("");
                        println!("  {} [{}] {} — {}", "→".dimmed(), priority, action.cyan(), reason);
                    }
                }
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }).await;
}

pub async fn annotated_screenshot(output: &str) {
    let out = output.to_string();
    with_page(|page| async move {
        match onecrawl_cdp::annotated::annotated_screenshot(&page).await {
            Ok(result) => {
                if let Some(b64) = result["screenshot_base64"].as_str() {
                    use base64::Engine;
                    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(b64) {
                        if std::fs::write(&out, &bytes).is_ok() {
                            let count = result["element_map"]["count"].as_u64().unwrap_or(0);
                            println!("{} Annotated screenshot saved to {} ({} elements)", "✓".green(), out.cyan(), count);
                            return Ok(());
                        }
                    }
                }
                Err("Failed to decode or save screenshot".to_string())
            }
            Err(e) => Err(e.to_string()),
        }
    }).await;
}

pub async fn adaptive_retry(action: &str, alternatives: &[String], max_retries: usize) {
    let act = action.to_string();
    let alts = alternatives.to_vec();
    with_page(|page| async move {
        match onecrawl_cdp::annotated::adaptive_retry(&page, &act, max_retries, &alts).await {
            Ok(result) => {
                let status = result["status"].as_str().unwrap_or("unknown");
                println!("{} Retry result: {}", if status == "success" { "✓".green() } else { "✗".red() }, status);
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }).await;
}

pub async fn click_at_coords(x: f64, y: f64) {
    with_page(|page| async move {
        match onecrawl_cdp::agent::click_at_coords(&page, x, y).await {
            Ok(result) => {
                let clicked = result["clicked"].as_bool().unwrap_or(false);
                if clicked {
                    let tag = result["element"]["tag"].as_str().unwrap_or("?");
                    println!("{} Clicked <{}> at ({}, {})", "✓".green(), tag.cyan(), x, y);
                } else {
                    println!("{} No element at ({}, {})", "✗".red(), x, y);
                }
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }).await;
}

pub async fn multi_page_sync() {
    with_page(|page| async move {
        let js = r#"JSON.stringify({ url: location.href, title: document.title, ready: document.readyState })"#.to_string();
        match page.evaluate(js).await {
            Ok(val) => {
                let s: String = val.into_value().unwrap_or_default();
                println!("{}", s);
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }).await;
}

pub async fn input_replay_file(events_file: &str) {
    let content = match std::fs::read_to_string(events_file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{} Cannot read {}: {}", "✗".red(), events_file, e);
            return;
        }
    };
    let events: Vec<serde_json::Value> = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{} Invalid JSON: {}", "✗".red(), e);
            return;
        }
    };
    with_page(|page| async move {
        match onecrawl_cdp::agent::input_replay(&page, &events).await {
            Ok(result) => {
                let total = result["total_events"].as_u64().unwrap_or(0);
                println!("{} Replayed {} events", "✓".green(), total.to_string().cyan());
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }).await;
}
