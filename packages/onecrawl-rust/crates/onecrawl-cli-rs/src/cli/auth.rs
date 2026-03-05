use clap::Subcommand;


#[derive(Subcommand)]
pub(crate) enum CaptchaAction {
    /// Detect CAPTCHA presence on the current page
    Detect,
    /// Wait for a CAPTCHA to appear (with timeout)
    Wait {
        /// Timeout in ms
        #[arg(short, long, default_value = "30000")]
        timeout: u64,
    },
    /// Take a screenshot of the detected CAPTCHA element
    Screenshot,
    /// Inject a CAPTCHA solution token into the page
    Inject {
        /// Solution token
        solution: String,
    },
    /// Solve a CAPTCHA using browser-native methods (free, no API key)
    ///
    /// Turnstile: clicks checkbox + waits for auto-clear.
    /// reCAPTCHA v2: switches to audio challenge + local Whisper transcription.
    ///
    /// With `--api`: uses external solver (CapSolver/2captcha/AntiCaptcha).
    /// Configure in `~/.onecrawl/config.json`: `{"capsolver_key":"CAP-xxx"}` or
    /// `{"twocaptcha_key":"abc"}` or `{"anticaptcha_key":"xyz"}`.
    Solve {
        /// Timeout in ms for solving
        #[arg(short, long, default_value = "30000")]
        timeout: u64,
        /// Use external API solver (reads key from ~/.onecrawl/config.json)
        #[arg(long)]
        api: bool,
    },
    /// Run comprehensive stealth fingerprint check
    ///
    /// Tests 15 browser properties: webdriver, plugins, WebGL, UA, toString, etc.
    /// Returns a score (0-100%) and detailed findings.
    Check,
    /// List all detectable CAPTCHA types
    Types,
}


#[derive(Subcommand)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum AuthAction {
    /// Enable virtual WebAuthn authenticator
    PasskeyEnable {
        /// Protocol: ctap2 or u2f
        #[arg(long, default_value = "ctap2")]
        protocol: String,
        /// Transport: internal, usb, nfc, ble
        #[arg(long, default_value = "internal")]
        transport: String,
    },
    /// Add a passkey credential
    PasskeyAdd {
        /// Base64url-encoded credential ID
        #[arg(long)]
        credential_id: String,
        /// Relying party domain
        #[arg(long)]
        rp_id: String,
        /// Optional user handle
        #[arg(long)]
        user_handle: Option<String>,
    },
    /// List stored passkey credentials
    PasskeyList,
    /// Show passkey operation log
    PasskeyLog,
    /// Disable virtual authenticator
    PasskeyDisable,
    /// Remove a passkey credential
    PasskeyRemove {
        /// Credential ID to remove
        #[arg(long)]
        credential_id: String,
    },
    /// Enable CDP virtual authenticator, watch for passkey creation, export credential.
    ///
    /// Run this BEFORE registering a passkey on x.com (or other site):
    ///   1. Start a headed session and log in
    ///   2. Run `auth passkey-register --output /tmp/xcom-passkey.json`
    ///   3. Register the passkey in the browser (x.com Settings → Security → Passkey)
    ///   4. The credential is exported automatically when Chrome records it
    PasskeyRegister {
        /// File to write the exported passkey credentials (JSON)
        #[arg(long, default_value = "/tmp/onecrawl-passkeys.json")]
        output: String,
        /// Seconds to wait for the passkey to be registered (default: 120)
        #[arg(long, default_value_t = 120u64)]
        timeout_secs: u64,
    },
    /// Store a passkey file path in the session so CDP WebAuthn is re-injected
    /// on every connect. Use with `session start --import-passkey FILE`.
    PasskeySetFile {
        /// Path to passkey JSON file produced by `auth passkey-register`
        #[arg(long)]
        file: String,
    },

    // ── Vault commands ───────────────────────────────────────────────

    /// List all sites and credentials stored in the passkey vault (~/.onecrawl/passkeys/vault.json).
    VaultList,

    /// Save credentials from a native passkey JSON file into the vault.
    VaultSave {
        /// Path to passkey JSON file (produced by `auth passkey-register`)
        #[arg(long)]
        input: String,
    },

    /// Remove a credential from the vault by its credential_id.
    VaultRemove {
        /// Base64-encoded credential ID to remove
        #[arg(long)]
        credential_id: String,
    },

    /// Remove all credentials for a specific rp_id (site) from the vault.
    VaultClearSite {
        /// Relying party ID, e.g. `x.com`
        #[arg(long)]
        rp_id: String,
    },

    /// Export vault credentials for a site to a passkey JSON file.
    VaultExport {
        /// Relying party ID, e.g. `x.com`
        #[arg(long)]
        rp_id: String,
        /// Output file path
        #[arg(long, default_value = "/tmp/onecrawl-passkeys.json")]
        output: String,
    },

    /// Import passkeys from a Bitwarden unencrypted JSON export.
    ///
    /// Generate with: `bw export --format json --output export.json`
    /// Note: Only Bitwarden-native passkeys are importable. Apple/Windows
    /// hardware-bound passkeys cannot be exported by design.
    ImportBitwarden {
        /// Path to Bitwarden JSON export file
        #[arg(long)]
        input: String,
        /// Also save imported credentials to the vault
        #[arg(long, default_value_t = true)]
        vault: bool,
    },

    /// Import passkeys from a 1Password export (export.data JSON from a .1pux archive).
    ///
    /// Extract the .1pux ZIP first: `unzip export.1pux export.data`
    ImportOnePassword {
        /// Path to `export.data` JSON file (extracted from .1pux)
        #[arg(long)]
        input: String,
        /// Also save imported credentials to the vault
        #[arg(long, default_value_t = true)]
        vault: bool,
    },

    /// Import passkeys from a FIDO Alliance CXF (Credential Exchange Format) JSON file.
    ///
    /// CXF v1.0 (FIDO Alliance draft, Oct 2024). For encrypted CXF files,
    /// decrypt first — HPKE-encrypted CXF is not yet supported.
    ImportCxf {
        /// Path to CXF JSON file (`cxf.json`)
        #[arg(long)]
        input: String,
        /// Also save imported credentials to the vault
        #[arg(long, default_value_t = true)]
        vault: bool,
    },
}


#[derive(Subcommand)]
pub(crate) enum StealthAction {
    /// Inject stealth anti-detection patches
    Inject,
}


#[derive(Subcommand)]
pub(crate) enum AntibotAction {
    /// Inject full anti-bot stealth patches
    Inject {
        /// Level: basic, standard, aggressive
        #[arg(short, long, default_value = "aggressive")]
        level: String,
    },
    /// Run bot detection test on the current page
    Test,
    /// List available stealth profiles
    Profiles,
}


#[derive(Subcommand)]
pub(crate) enum AuthStateAction {
    /// Save current auth state (cookies + localStorage) to a file
    Save {
        /// Output file path
        path: String,
    },
    /// Load auth state from a file
    Load {
        /// Input file path
        path: String,
    },
    /// List saved state files
    List,
    /// Show summary of a state file
    Show {
        /// Path to state file
        path: String,
    },
    /// Rename a state file
    Rename {
        /// Old file name
        old: String,
        /// New file name
        new: String,
    },
    /// Clear state files
    Clear {
        /// Clear all saved states
        #[arg(long)]
        all: bool,
        /// Specific state name to clear
        name: Option<String>,
    },
    /// Delete old state files
    Clean {
        /// Delete states older than N days
        #[arg(long, default_value = "30")]
        older_than: u64,
    },
}

