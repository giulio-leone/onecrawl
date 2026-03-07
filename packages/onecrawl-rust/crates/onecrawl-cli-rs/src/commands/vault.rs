use crate::cli::VaultAction;

fn resolve_path(path: &str) -> String {
    if path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return path.replacen("~", &home, 1);
        }
    }
    path.to_string()
}

fn read_password(prompt: &str) -> String {
    eprint!("{prompt}");
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .unwrap_or_default();
    input.trim().to_string()
}

pub fn handle(action: VaultAction) {
    match action {
        VaultAction::Create { path } => {
            let path = resolve_path(&path);
            let password = read_password("Enter master password: ");
            match onecrawl_crypto::vault::Vault::create(&path, &password) {
                Ok(vault) => {
                    println!("✅ Vault created at {path}");
                    println!("   Entries: {}", vault.len());
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        VaultAction::Open { path } => {
            let path = resolve_path(&path);
            let password = read_password("Enter master password: ");
            match onecrawl_crypto::vault::Vault::open(&path, &password) {
                Ok(vault) => {
                    println!("✅ Vault opened ({} entries)", vault.len());
                    let expired = vault.check_expired();
                    if !expired.is_empty() {
                        println!("⚠️  Expired keys: {}", expired.join(", "));
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        VaultAction::Set {
            key,
            value,
            category,
            prompt,
            path,
        } => {
            let path = resolve_path(&path);
            let password = read_password("Enter master password: ");
            let secret_value = if prompt || value.is_none() {
                read_password("Enter secret value: ")
            } else {
                value.unwrap_or_default()
            };

            let open_result = onecrawl_crypto::vault::Vault::open(&path, &password)
                .or_else(|_| onecrawl_crypto::vault::Vault::create(&path, &password));

            match open_result {
                Ok(mut vault) => match vault.set(&key, &secret_value, category.as_deref()) {
                    Ok(()) => println!("✅ Secret '{key}' stored"),
                    Err(e) => {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        VaultAction::Get { key, path } => {
            let path = resolve_path(&path);
            let password = read_password("Enter master password: ");
            match onecrawl_crypto::vault::Vault::open(&path, &password) {
                Ok(vault) => match vault.get(&key) {
                    Some(entry) => println!("{}", entry.value),
                    None => {
                        eprintln!("Key '{key}' not found");
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        VaultAction::Delete { key, path } => {
            let path = resolve_path(&path);
            let password = read_password("Enter master password: ");
            match onecrawl_crypto::vault::Vault::open(&path, &password) {
                Ok(mut vault) => match vault.delete(&key) {
                    Ok(()) => println!("✅ Secret '{key}' deleted"),
                    Err(e) => {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        VaultAction::List { category, path } => {
            let path = resolve_path(&path);
            let password = read_password("Enter master password: ");
            match onecrawl_crypto::vault::Vault::open(&path, &password) {
                Ok(vault) => {
                    let entries = match &category {
                        Some(cat) => vault.list_by_category(cat),
                        None => vault.list(),
                    };
                    if entries.is_empty() {
                        println!("Vault is empty");
                    } else {
                        println!("{:<30} {:<15} {:<24} {}", "KEY", "CATEGORY", "UPDATED", "STATUS");
                        println!("{}", "-".repeat(75));
                        for e in &entries {
                            let cat = e.category.as_deref().unwrap_or("-");
                            let status = if e.expired {
                                "EXPIRED"
                            } else if e.has_expiry {
                                "has expiry"
                            } else {
                                "ok"
                            };
                            println!("{:<30} {:<15} {:<24} {}", e.key, cat, e.updated_at, status);
                        }
                        println!("\n{} entries", entries.len());
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        VaultAction::Use { service, path } => {
            let path = resolve_path(&path);
            let password = read_password("Enter master password: ");
            match onecrawl_crypto::vault::Vault::open(&path, &password) {
                Ok(vault) => {
                    let vars = vault.export_for_workflow(&service);
                    if vars.is_empty() {
                        println!("No entries for service '{service}'");
                    } else {
                        let json = serde_json::to_string_pretty(&vars)
                            .unwrap_or_else(|_| format!("{vars:?}"));
                        println!("{json}");
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        VaultAction::ChangePassword { path } => {
            let path = resolve_path(&path);
            let old_pass = read_password("Enter current password: ");
            let new_pass = read_password("Enter new password: ");
            match onecrawl_crypto::vault::Vault::open(&path, &old_pass) {
                Ok(mut vault) => match vault.change_password(&new_pass) {
                    Ok(()) => println!("✅ Password changed"),
                    Err(e) => {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        VaultAction::ImportEnv { prefix, path } => {
            let path = resolve_path(&path);
            let password = read_password("Enter master password: ");
            let open_result = onecrawl_crypto::vault::Vault::open(&path, &password)
                .or_else(|_| onecrawl_crypto::vault::Vault::create(&path, &password));

            match open_result {
                Ok(mut vault) => match vault.import_env(&prefix) {
                    Ok(count) => println!("✅ Imported {count} entries from env (prefix: {prefix})"),
                    Err(e) => {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
    }
}
