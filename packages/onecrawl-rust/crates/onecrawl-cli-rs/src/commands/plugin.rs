use crate::cli::PluginCliAction;

pub async fn handle(action: PluginCliAction) {
    match action {
        PluginCliAction::Install { path } => install(&path).await,
        PluginCliAction::Uninstall { name } => uninstall(&name).await,
        PluginCliAction::Enable { name } => enable(&name).await,
        PluginCliAction::Disable { name } => disable(&name).await,
        PluginCliAction::List => list().await,
        PluginCliAction::Info { name } => info(&name).await,
        PluginCliAction::Create { name, path } => create(&name, path.as_deref()).await,
        PluginCliAction::Run {
            plugin_name,
            action_name,
            params,
        } => run(&plugin_name, &action_name, params.as_deref()).await,
        PluginCliAction::Config { name, set } => config(&name, set.as_deref()).await,
    }
}

fn plugins_dir() -> String {
    onecrawl_cdp::default_plugins_dir()
        .to_string_lossy()
        .into_owned()
}

async fn install(path: &str) {
    let mut registry = match onecrawl_cdp::PluginRegistry::new(&plugins_dir()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Failed to load plugin registry: {e}");
            return;
        }
    };

    match registry.install_local(path) {
        Ok(plugin) => {
            println!("✅ Plugin '{}' v{} installed", plugin.manifest.name, plugin.manifest.version);
            println!("   Description: {}", plugin.manifest.description);
            println!("   Commands: {}  Actions: {}  Hooks: {}",
                plugin.manifest.commands.len(),
                plugin.manifest.actions.len(),
                plugin.manifest.hooks.len(),
            );
            println!();
            println!("   Run 'onecrawl plugin enable {}' to activate", plugin.manifest.name);
        }
        Err(e) => eprintln!("❌ Failed to install plugin: {e}"),
    }
}

async fn uninstall(name: &str) {
    let mut registry = match onecrawl_cdp::PluginRegistry::new(&plugins_dir()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Failed to load plugin registry: {e}");
            return;
        }
    };

    match registry.uninstall(name) {
        Ok(()) => println!("✅ Plugin '{name}' uninstalled"),
        Err(e) => eprintln!("❌ Failed to uninstall: {e}"),
    }
}

async fn enable(name: &str) {
    let mut registry = match onecrawl_cdp::PluginRegistry::new(&plugins_dir()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Failed to load plugin registry: {e}");
            return;
        }
    };

    match registry.enable(name) {
        Ok(()) => println!("✅ Plugin '{name}' enabled"),
        Err(e) => eprintln!("❌ Failed to enable: {e}"),
    }
}

async fn disable(name: &str) {
    let mut registry = match onecrawl_cdp::PluginRegistry::new(&plugins_dir()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Failed to load plugin registry: {e}");
            return;
        }
    };

    match registry.disable(name) {
        Ok(()) => println!("✅ Plugin '{name}' disabled"),
        Err(e) => eprintln!("❌ Failed to disable: {e}"),
    }
}

async fn list() {
    let registry = match onecrawl_cdp::PluginRegistry::new(&plugins_dir()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Failed to load plugin registry: {e}");
            return;
        }
    };

    let plugins = registry.list();
    if plugins.is_empty() {
        println!("No plugins installed.");
        println!();
        println!("Install a plugin:  onecrawl plugin install <path>");
        println!("Create a plugin:   onecrawl plugin create <name>");
        return;
    }

    println!("Installed plugins ({}):", plugins.len());
    println!();
    for p in &plugins {
        let status = match &p.status {
            onecrawl_cdp::PluginStatus::Installed => "installed",
            onecrawl_cdp::PluginStatus::Active => "active",
            onecrawl_cdp::PluginStatus::Disabled => "disabled",
            onecrawl_cdp::PluginStatus::Error(e) => e.as_str(),
        };
        println!(
            "  {} v{} [{}]",
            p.manifest.name, p.manifest.version, status
        );
        println!("    {}", p.manifest.description);
    }
}

async fn info(name: &str) {
    let registry = match onecrawl_cdp::PluginRegistry::new(&plugins_dir()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Failed to load plugin registry: {e}");
            return;
        }
    };

    match registry.get(name) {
        Some(p) => {
            let json = serde_json::to_string_pretty(&p).unwrap_or_default();
            println!("{json}");
        }
        None => eprintln!("❌ Plugin '{name}' not found"),
    }
}

async fn create(name: &str, path: Option<&str>) {
    let registry = match onecrawl_cdp::PluginRegistry::new(&plugins_dir()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Failed to load plugin registry: {e}");
            return;
        }
    };

    let target = path.unwrap_or(name);
    match registry.create_scaffold(name, target) {
        Ok(()) => {
            println!("✅ Plugin scaffold created: {target}/");
            println!("   plugin.json         — manifest");
            println!("   handlers/hello.json — example handler");
            println!("   README.md           — documentation");
            println!();
            println!("Next steps:");
            println!("  1. Edit plugin.json to define commands/actions");
            println!("  2. Create handler JSON workflows in handlers/");
            println!("  3. Install: onecrawl plugin install {target}");
        }
        Err(e) => eprintln!("❌ Failed to create scaffold: {e}"),
    }
}

async fn run(plugin_name: &str, action_name: &str, params: Option<&str>) {
    let registry = match onecrawl_cdp::PluginRegistry::new(&plugins_dir()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Failed to load plugin registry: {e}");
            return;
        }
    };

    let params_value: serde_json::Value = match params {
        Some(json_str) => match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("❌ Invalid JSON params: {e}");
                return;
            }
        },
        None => serde_json::json!({}),
    };

    match registry.execute_action(plugin_name, action_name, params_value).await {
        Ok(result) => {
            let json = serde_json::to_string_pretty(&result).unwrap_or_default();
            println!("{json}");
        }
        Err(e) => eprintln!("❌ Failed to execute action: {e}"),
    }
}

async fn config(name: &str, set: Option<&str>) {
    let mut registry = match onecrawl_cdp::PluginRegistry::new(&plugins_dir()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Failed to load plugin registry: {e}");
            return;
        }
    };

    match set {
        Some(config_str) => {
            // Try to parse as JSON first
            let config: serde_json::Value = match serde_json::from_str(config_str) {
                Ok(v) => v,
                Err(_) => {
                    // Try key=value format
                    if let Some((key, value)) = config_str.split_once('=') {
                        serde_json::json!({ key: value })
                    } else {
                        eprintln!("❌ Invalid config format. Use JSON or key=value");
                        return;
                    }
                }
            };

            match registry.configure(name, config) {
                Ok(()) => println!("✅ Plugin '{name}' configured"),
                Err(e) => eprintln!("❌ Failed to configure: {e}"),
            }
        }
        None => {
            // Show current config
            match registry.get(name) {
                Some(p) => {
                    match &p.config {
                        Some(c) => {
                            let json = serde_json::to_string_pretty(c).unwrap_or_default();
                            println!("{json}");
                        }
                        None => println!("No configuration set for plugin '{name}'"),
                    }
                }
                None => eprintln!("❌ Plugin '{name}' not found"),
            }
        }
    }
}
