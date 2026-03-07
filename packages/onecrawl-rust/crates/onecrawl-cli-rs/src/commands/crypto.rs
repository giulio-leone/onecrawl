use clap::Subcommand;

#[derive(Subcommand)]
pub enum CryptoAction {
    /// Encrypt data
    Encrypt {
        /// Data to encrypt
        data: String,
        /// Passphrase
        #[arg(short, long)]
        passphrase: String,
    },
    /// Decrypt data
    Decrypt {
        /// Encrypted payload (JSON)
        payload: String,
        /// Passphrase
        #[arg(short, long)]
        passphrase: String,
    },
    /// Generate PKCE challenge
    Pkce,
    /// Generate TOTP code
    Totp {
        /// Base32-encoded secret
        secret: String,
    },
    /// Generate random TOTP secret
    GenerateSecret,
    /// Derive key from passphrase
    DeriveKey {
        /// Passphrase
        passphrase: String,
        /// Salt (hex)
        salt: String,
    },
}

pub fn handle(action: CryptoAction) {
    match action {
        CryptoAction::Encrypt { data, passphrase } => {
            match onecrawl_crypto::encrypt(data.as_bytes(), &passphrase) {
                Ok(payload) => {
                    println!("{}", serde_json::to_string_pretty(&payload).unwrap());
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        CryptoAction::Decrypt {
            payload,
            passphrase,
        } => {
            let parsed: onecrawl_core::EncryptedPayload = match serde_json::from_str(&payload) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Invalid JSON payload: {e}");
                    std::process::exit(1);
                }
            };
            match onecrawl_crypto::decrypt(&parsed, &passphrase) {
                Ok(data) => {
                    println!("{}", String::from_utf8_lossy(&data));
                }
                Err(e) => {
                    eprintln!("Decryption failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        CryptoAction::Pkce => match onecrawl_crypto::generate_pkce_challenge() {
            Ok(challenge) => {
                println!("code_verifier:  {}", challenge.code_verifier);
                println!("code_challenge: {}", challenge.code_challenge);
            }
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        },
        CryptoAction::Totp { secret } => {
            let config = onecrawl_core::TotpConfig {
                secret,
                ..Default::default()
            };
            match onecrawl_crypto::generate_totp(&config) {
                Ok(code) => println!("{code}"),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        CryptoAction::GenerateSecret => {
            println!("{}", onecrawl_crypto::totp::generate_secret());
        }
        CryptoAction::DeriveKey { passphrase, salt } => {
            let salt_bytes = match hex::decode(&salt) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("Error: invalid hex salt: {e}");
                    std::process::exit(1);
                }
            };
            match onecrawl_crypto::derive_key(&passphrase, &salt_bytes) {
                Ok(key) => println!("{}", hex::encode(key)),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
    }
}
