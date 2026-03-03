use clap::Subcommand;

#[derive(Subcommand)]
pub enum StorageAction {
    /// Get a value
    Get {
        /// Key
        key: String,
        /// Store path
        #[arg(short, long, default_value = ".onecrawl/store")]
        path: String,
        /// Passphrase
        #[arg(short = 'P', long, env = "ONECRAWL_PASSPHRASE")]
        passphrase: String,
    },
    /// Set a value
    Set {
        /// Key
        key: String,
        /// Value
        value: String,
        /// Store path
        #[arg(short, long, default_value = ".onecrawl/store")]
        path: String,
        /// Passphrase
        #[arg(short = 'P', long, env = "ONECRAWL_PASSPHRASE")]
        passphrase: String,
    },
    /// Delete a key
    Delete {
        /// Key
        key: String,
        /// Store path
        #[arg(short, long, default_value = ".onecrawl/store")]
        path: String,
        /// Passphrase
        #[arg(short = 'P', long, env = "ONECRAWL_PASSPHRASE")]
        passphrase: String,
    },
    /// List keys
    List {
        /// Key prefix
        #[arg(default_value = "")]
        prefix: String,
        /// Store path
        #[arg(short, long, default_value = ".onecrawl/store")]
        path: String,
        /// Passphrase
        #[arg(short = 'P', long, env = "ONECRAWL_PASSPHRASE")]
        passphrase: String,
    },
}

pub async fn handle(action: StorageAction) {
    match action {
        StorageAction::Get {
            key,
            path,
            passphrase,
        } => {
            let store =
                onecrawl_storage::EncryptedStore::open(std::path::Path::new(&path), &passphrase)
                    .unwrap_or_else(|e| {
                        eprintln!("Cannot open store: {e}");
                        std::process::exit(1);
                    });
            match store.get(&key) {
                Ok(Some(data)) => println!("{}", String::from_utf8_lossy(&data)),
                Ok(None) => {
                    eprintln!("Key not found: {key}");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        StorageAction::Set {
            key,
            value,
            path,
            passphrase,
        } => {
            let store =
                onecrawl_storage::EncryptedStore::open(std::path::Path::new(&path), &passphrase)
                    .unwrap_or_else(|e| {
                        eprintln!("Cannot open store: {e}");
                        std::process::exit(1);
                    });
            match store.set(&key, value.as_bytes()) {
                Ok(()) => println!("OK"),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        StorageAction::Delete {
            key,
            path,
            passphrase,
        } => {
            let store =
                onecrawl_storage::EncryptedStore::open(std::path::Path::new(&path), &passphrase)
                    .unwrap_or_else(|e| {
                        eprintln!("Cannot open store: {e}");
                        std::process::exit(1);
                    });
            match store.delete(&key) {
                Ok(true) => println!("Deleted"),
                Ok(false) => {
                    eprintln!("Key not found: {key}");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        StorageAction::List {
            prefix,
            path,
            passphrase,
        } => {
            let store =
                onecrawl_storage::EncryptedStore::open(std::path::Path::new(&path), &passphrase)
                    .unwrap_or_else(|e| {
                        eprintln!("Cannot open store: {e}");
                        std::process::exit(1);
                    });
            match store.list(&prefix) {
                Ok(keys) => {
                    for key in keys {
                        println!("{key}");
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
    }
}
