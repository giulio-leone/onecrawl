//! Plugin system — manifest-based extensibility for OneCrawl.
//!
//! Plugins are directories containing a `plugin.json` manifest and handler files.
//! The registry manages plugin lifecycle: install, enable, disable, uninstall.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ════════════════════════════════════════════════════════════════════
//  Manifest types
// ════════════════════════════════════════════════════════════════════

/// Plugin manifest (`plugin.json` in each plugin directory).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub onecrawl_version: Option<String>,
    #[serde(default)]
    pub commands: Vec<PluginCommand>,
    #[serde(default)]
    pub actions: Vec<PluginAction>,
    #[serde(default)]
    pub hooks: Vec<PluginHook>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub config_schema: Option<serde_json::Value>,
}

/// A CLI command the plugin adds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommand {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub args: Vec<PluginArg>,
    pub handler: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginArg {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_arg_type")]
    pub arg_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
}

fn default_arg_type() -> String {
    "string".into()
}

/// An MCP action the plugin adds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAction {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub params: Vec<PluginParam>,
    pub handler: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginParam {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_arg_type")]
    pub param_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
}

/// Event hooks the plugin registers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHook {
    pub event: String,
    pub handler: String,
    #[serde(default)]
    pub filter: Option<String>,
}

// ════════════════════════════════════════════════════════════════════
//  Plugin status / installed info
// ════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginStatus {
    Installed,
    Active,
    Disabled,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub manifest: PluginManifest,
    pub path: PathBuf,
    pub status: PluginStatus,
    pub installed_at: String,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
}

// ════════════════════════════════════════════════════════════════════
//  Registry persistence format
// ════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RegistryState {
    plugins: HashMap<String, InstalledPlugin>,
}

// ════════════════════════════════════════════════════════════════════
//  Plugin registry
// ════════════════════════════════════════════════════════════════════

pub struct PluginRegistry {
    plugins_dir: PathBuf,
    plugins: HashMap<String, InstalledPlugin>,
}

impl PluginRegistry {
    /// Create or load the plugin registry from `plugins_dir`.
    pub fn new(plugins_dir: &str) -> Result<Self, String> {
        let dir = PathBuf::from(plugins_dir);
        if !dir.exists() {
            std::fs::create_dir_all(&dir)
                .map_err(|e| format!("failed to create plugins dir: {e}"))?;
        }

        let mut registry = Self {
            plugins_dir: dir.clone(),
            plugins: HashMap::new(),
        };

        // Load persisted registry state if it exists
        let registry_file = dir.join("registry.json");
        if registry_file.exists() {
            let data = std::fs::read_to_string(&registry_file)
                .map_err(|e| format!("failed to read registry.json: {e}"))?;
            let state: RegistryState = serde_json::from_str(&data)
                .map_err(|e| format!("failed to parse registry.json: {e}"))?;
            registry.plugins = state.plugins;
        }

        // Also scan for any plugin dirs not yet in the registry
        registry.scan_plugins()?;

        Ok(registry)
    }

    /// Scan for plugin directories that contain a `plugin.json` but aren't yet registered.
    fn scan_plugins(&mut self) -> Result<(), String> {
        let entries = std::fs::read_dir(&self.plugins_dir)
            .map_err(|e| format!("failed to read plugins dir: {e}"))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let manifest_path = path.join("plugin.json");
            if !manifest_path.exists() {
                continue;
            }
            let dir_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if self.plugins.contains_key(&dir_name) {
                continue;
            }
            // Try to load the manifest
            match load_manifest(&manifest_path) {
                Ok(manifest) => {
                    self.plugins.insert(dir_name, InstalledPlugin {
                        manifest,
                        path,
                        status: PluginStatus::Installed,
                        installed_at: now_iso(),
                        config: None,
                    });
                }
                Err(_) => continue,
            }
        }
        Ok(())
    }

    /// Install a plugin from a local directory or archive.
    pub fn install_local(&mut self, source_path: &str) -> Result<InstalledPlugin, String> {
        let src = PathBuf::from(source_path);
        if !src.exists() {
            return Err(format!("source path does not exist: {source_path}"));
        }

        let manifest_path = if src.is_dir() {
            src.join("plugin.json")
        } else {
            return Err("source must be a directory containing plugin.json".into());
        };

        if !manifest_path.exists() {
            return Err("plugin.json not found in source directory".into());
        }

        let manifest = load_manifest(&manifest_path)?;
        Self::validate_manifest(&manifest)?;

        let name = manifest.name.clone();
        validate_plugin_name(&name)?;

        let dest = self.plugins_dir.join(&name);
        if dest.exists() {
            return Err(format!("plugin '{name}' is already installed"));
        }

        copy_dir_recursive(&src, &dest)?;

        let plugin = InstalledPlugin {
            manifest,
            path: dest,
            status: PluginStatus::Installed,
            installed_at: now_iso(),
            config: None,
        };

        self.plugins.insert(name, plugin.clone());
        self.save()?;
        Ok(plugin)
    }

    /// Install from a remote URL (not yet supported).
    pub async fn install_remote(&mut self, _url: &str) -> Result<InstalledPlugin, String> {
        Err("remote plugin installation is not yet supported — use install_local instead".into())
    }

    /// Create a plugin scaffold.
    pub fn create_scaffold(&self, name: &str, path: &str) -> Result<(), String> {
        validate_plugin_name(name)?;
        let dir = PathBuf::from(path);
        if dir.exists() {
            return Err(format!("directory already exists: {path}"));
        }
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("failed to create directory: {e}"))?;

        let manifest = PluginManifest {
            name: name.to_string(),
            version: "0.1.0".into(),
            description: format!("{name} plugin for OneCrawl"),
            author: None,
            license: Some("MIT".into()),
            homepage: None,
            onecrawl_version: None,
            commands: vec![PluginCommand {
                name: format!("{name} hello"),
                description: "Example command".into(),
                args: vec![PluginArg {
                    name: "message".into(),
                    description: Some("Greeting message".into()),
                    arg_type: "string".into(),
                    required: false,
                    default: Some("Hello from plugin!".into()),
                }],
                handler: "handlers/hello.json".into(),
            }],
            actions: vec![PluginAction {
                name: format!("{name}_hello"),
                description: "Example action".into(),
                params: vec![PluginParam {
                    name: "message".into(),
                    description: Some("Greeting message".into()),
                    param_type: "string".into(),
                    required: false,
                    default: Some(serde_json::Value::String("Hello from plugin!".into())),
                }],
                handler: "handlers/hello.json".into(),
            }],
            hooks: vec![],
            dependencies: vec![],
            config_schema: None,
        };

        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| format!("failed to serialize manifest: {e}"))?;
        std::fs::write(dir.join("plugin.json"), manifest_json)
            .map_err(|e| format!("failed to write plugin.json: {e}"))?;

        // Create handlers directory with example handler
        let handlers_dir = dir.join("handlers");
        std::fs::create_dir_all(&handlers_dir)
            .map_err(|e| format!("failed to create handlers dir: {e}"))?;

        let example_handler = serde_json::json!({
            "name": "hello",
            "description": "Example handler",
            "steps": [
                {
                    "action": "log",
                    "params": {
                        "message": "Hello from the plugin handler!"
                    }
                }
            ]
        });
        let handler_json = serde_json::to_string_pretty(&example_handler)
            .map_err(|e| format!("failed to serialize handler: {e}"))?;
        std::fs::write(handlers_dir.join("hello.json"), handler_json)
            .map_err(|e| format!("failed to write handler: {e}"))?;

        // Create README
        let readme = format!(
            "# {name}\n\nOneCrawl plugin.\n\n## Installation\n\n```\nonecrawl plugin install .\n```\n"
        );
        std::fs::write(dir.join("README.md"), readme)
            .map_err(|e| format!("failed to write README: {e}"))?;

        Ok(())
    }

    /// Uninstall a plugin.
    pub fn uninstall(&mut self, name: &str) -> Result<(), String> {
        validate_plugin_name(name)?;
        if !self.plugins.contains_key(name) {
            return Err(format!("plugin '{name}' is not installed"));
        }

        let plugin_dir = self.plugins_dir.join(name);
        // Safety: ensure the path is within plugins_dir
        if !is_within(&plugin_dir, &self.plugins_dir) {
            return Err("path traversal detected".into());
        }

        if plugin_dir.exists() {
            std::fs::remove_dir_all(&plugin_dir)
                .map_err(|e| format!("failed to remove plugin directory: {e}"))?;
        }

        self.plugins.remove(name);
        self.save()?;
        Ok(())
    }

    /// Enable a plugin.
    pub fn enable(&mut self, name: &str) -> Result<(), String> {
        let plugin = self.plugins.get_mut(name)
            .ok_or_else(|| format!("plugin '{name}' is not installed"))?;
        plugin.status = PluginStatus::Active;
        self.save()
    }

    /// Disable a plugin.
    pub fn disable(&mut self, name: &str) -> Result<(), String> {
        let plugin = self.plugins.get_mut(name)
            .ok_or_else(|| format!("plugin '{name}' is not installed"))?;
        plugin.status = PluginStatus::Disabled;
        self.save()
    }

    /// List all installed plugins.
    pub fn list(&self) -> Vec<InstalledPlugin> {
        self.plugins.values().cloned().collect()
    }

    /// Get plugin info.
    pub fn get(&self, name: &str) -> Option<&InstalledPlugin> {
        self.plugins.get(name)
    }

    /// Get all commands from active plugins.
    pub fn active_commands(&self) -> Vec<(String, PluginCommand)> {
        self.plugins
            .iter()
            .filter(|(_, p)| p.status == PluginStatus::Active)
            .flat_map(|(name, p)| {
                p.manifest.commands.iter().map(move |c| (name.clone(), c.clone()))
            })
            .collect()
    }

    /// Get all actions from active plugins.
    pub fn active_actions(&self) -> Vec<(String, PluginAction)> {
        self.plugins
            .iter()
            .filter(|(_, p)| p.status == PluginStatus::Active)
            .flat_map(|(name, p)| {
                p.manifest.actions.iter().map(move |a| (name.clone(), a.clone()))
            })
            .collect()
    }

    /// Get all hooks from active plugins.
    pub fn active_hooks(&self) -> Vec<(String, PluginHook)> {
        self.plugins
            .iter()
            .filter(|(_, p)| p.status == PluginStatus::Active)
            .flat_map(|(name, p)| {
                p.manifest.hooks.iter().map(move |h| (name.clone(), h.clone()))
            })
            .collect()
    }

    /// Validate a plugin manifest.
    fn validate_manifest(manifest: &PluginManifest) -> Result<(), String> {
        if manifest.name.is_empty() {
            return Err("plugin name is required".into());
        }
        if manifest.version.is_empty() {
            return Err("plugin version is required".into());
        }
        if !manifest.name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err("plugin name must contain only alphanumeric characters, hyphens, and underscores".into());
        }
        if manifest.description.is_empty() {
            return Err("plugin description is required".into());
        }
        // Validate handler paths don't escape plugin directory
        for cmd in &manifest.commands {
            validate_handler_path(&cmd.handler)?;
        }
        for action in &manifest.actions {
            validate_handler_path(&action.handler)?;
        }
        for hook in &manifest.hooks {
            validate_handler_path(&hook.handler)?;
        }
        Ok(())
    }

    /// Set plugin configuration.
    pub fn configure(&mut self, name: &str, config: serde_json::Value) -> Result<(), String> {
        let plugin = self.plugins.get_mut(name)
            .ok_or_else(|| format!("plugin '{name}' is not installed"))?;
        plugin.config = Some(config);
        self.save()
    }

    /// Save registry state to disk.
    pub fn save(&self) -> Result<(), String> {
        let state = RegistryState {
            plugins: self.plugins.clone(),
        };
        let json = serde_json::to_string_pretty(&state)
            .map_err(|e| format!("failed to serialize registry: {e}"))?;
        let registry_file = self.plugins_dir.join("registry.json");
        std::fs::write(&registry_file, json)
            .map_err(|e| format!("failed to write registry.json: {e}"))?;
        Ok(())
    }

    /// Execute a plugin action handler.
    ///
    /// For MVP, handlers are JSON workflow files. This reads the handler
    /// and returns its content along with the provided parameters for
    /// external execution by the workflow engine.
    pub async fn execute_action(
        &self,
        plugin_name: &str,
        action_name: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let plugin = self.plugins.get(plugin_name)
            .ok_or_else(|| format!("plugin '{plugin_name}' not found"))?;

        if plugin.status != PluginStatus::Active {
            return Err(format!("plugin '{plugin_name}' is not active"));
        }

        let action = plugin.manifest.actions.iter()
            .find(|a| a.name == action_name)
            .ok_or_else(|| format!(
                "action '{action_name}' not found in plugin '{plugin_name}'"
            ))?;

        let handler_path = plugin.path.join(&action.handler);

        // Safety: ensure handler path stays within plugin directory
        let canon_plugin = plugin.path.canonicalize()
            .map_err(|e| format!("failed to resolve plugin path: {e}"))?;
        let canon_handler = handler_path.canonicalize()
            .map_err(|e| format!("handler file not found: {e}"))?;
        if !canon_handler.starts_with(&canon_plugin) {
            return Err("handler path escapes plugin directory".into());
        }

        let handler_content = std::fs::read_to_string(&handler_path)
            .map_err(|e| format!("failed to read handler file: {e}"))?;

        let handler_json: serde_json::Value = serde_json::from_str(&handler_content)
            .map_err(|e| format!("failed to parse handler JSON: {e}"))?;

        Ok(serde_json::json!({
            "plugin": plugin_name,
            "action": action_name,
            "handler": handler_json,
            "params": params,
            "status": "ready",
        }))
    }
}

// ════════════════════════════════════════════════════════════════════
//  Built-in templates
// ════════════════════════════════════════════════════════════════════

/// Built-in plugin templates for common use cases.
pub fn builtin_templates() -> Vec<(&'static str, &'static str)> {
    vec![
        ("captcha-solver", "CAPTCHA solving plugin template"),
        ("auth-flow", "Authentication flow plugin template"),
        ("data-extractor", "Data extraction plugin template"),
        ("notification", "Notification plugin template"),
    ]
}

/// Return the default plugins directory (`~/.onecrawl/plugins/`).
pub fn default_plugins_dir() -> PathBuf {
    dirs_fallback().join("plugins")
}

// ════════════════════════════════════════════════════════════════════
//  Helpers
// ════════════════════════════════════════════════════════════════════

fn dirs_fallback() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".onecrawl")
}

fn now_iso() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}Z", d.as_secs())
}

fn load_manifest(path: &Path) -> Result<PluginManifest, String> {
    let data = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    serde_json::from_str(&data)
        .map_err(|e| format!("failed to parse {}: {e}", path.display()))
}

/// Validate that a plugin name contains no path traversal characters.
fn validate_plugin_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("plugin name cannot be empty".into());
    }
    if name.contains('.') || name.contains('/') || name.contains('\\') {
        return Err("plugin name must not contain '.', '/', or '\\'".into());
    }
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("plugin name must contain only alphanumeric characters, hyphens, and underscores".into());
    }
    Ok(())
}

/// Validate that a handler path doesn't try to escape the plugin directory.
fn validate_handler_path(handler: &str) -> Result<(), String> {
    if handler.contains("..") || handler.starts_with('/') || handler.starts_with('\\') {
        return Err(format!("invalid handler path '{handler}': must be relative and not contain '..'"));
    }
    Ok(())
}

/// Check that `child` is within `parent`.
fn is_within(child: &Path, parent: &Path) -> bool {
    match (child.canonicalize(), parent.canonicalize()) {
        (Ok(c), Ok(p)) => c.starts_with(p),
        _ => {
            // Fallback for paths that don't exist yet
            child.starts_with(parent)
        }
    }
}

/// Recursively copy a directory.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst)
        .map_err(|e| format!("failed to create {}: {e}", dst.display()))?;

    for entry in std::fs::read_dir(src)
        .map_err(|e| format!("failed to read {}: {e}", src.display()))?
    {
        let entry = entry.map_err(|e| format!("directory entry error: {e}"))?;
        let src_path = entry.path();
        let dst_path = dst.join(
            entry.file_name(),
        );

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("failed to copy {}: {e}", src_path.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_good_names() {
        assert!(validate_plugin_name("my-plugin").is_ok());
        assert!(validate_plugin_name("plugin_v2").is_ok());
        assert!(validate_plugin_name("CoolPlugin").is_ok());
    }

    #[test]
    fn reject_bad_names() {
        assert!(validate_plugin_name("").is_err());
        assert!(validate_plugin_name("../escape").is_err());
        assert!(validate_plugin_name("foo/bar").is_err());
        assert!(validate_plugin_name("foo\\bar").is_err());
        assert!(validate_plugin_name("has.dot").is_err());
    }

    #[test]
    fn reject_bad_handler_paths() {
        assert!(validate_handler_path("../escape.json").is_err());
        assert!(validate_handler_path("/absolute/path.json").is_err());
        assert!(validate_handler_path("handlers/ok.json").is_ok());
    }

    #[test]
    fn manifest_roundtrip() {
        let manifest = PluginManifest {
            name: "test-plugin".into(),
            version: "1.0.0".into(),
            description: "Test".into(),
            author: None,
            license: None,
            homepage: None,
            onecrawl_version: None,
            commands: vec![],
            actions: vec![],
            hooks: vec![],
            dependencies: vec![],
            config_schema: None,
        };
        let json = serde_json::to_string(&manifest).expect("serialize");
        let parsed: PluginManifest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.name, "test-plugin");
        assert_eq!(parsed.version, "1.0.0");
    }

    #[test]
    fn builtin_templates_not_empty() {
        assert!(!builtin_templates().is_empty());
    }

    #[test]
    fn validate_manifest_rejects_empty_name() {
        let m = PluginManifest {
            name: "".into(),
            version: "1.0.0".into(),
            description: "Test".into(),
            author: None,
            license: None,
            homepage: None,
            onecrawl_version: None,
            commands: vec![],
            actions: vec![],
            hooks: vec![],
            dependencies: vec![],
            config_schema: None,
        };
        assert!(PluginRegistry::validate_manifest(&m).is_err());
    }

    #[test]
    fn scaffold_and_install() {
        let tmp = std::env::temp_dir().join("onecrawl-plugin-test-scaffold");
        let _ = std::fs::remove_dir_all(&tmp);

        let plugins_dir = tmp.join("plugins");
        std::fs::create_dir_all(&plugins_dir).expect("create plugins dir");
        let registry = PluginRegistry::new(plugins_dir.to_str().expect("path"))
            .expect("create registry");

        let scaffold_dir = tmp.join("my-test-plugin");
        registry
            .create_scaffold("my-test-plugin", scaffold_dir.to_str().expect("path"))
            .expect("scaffold");

        assert!(scaffold_dir.join("plugin.json").exists());
        assert!(scaffold_dir.join("handlers/hello.json").exists());
        assert!(scaffold_dir.join("README.md").exists());

        // Verify manifest is valid JSON
        let manifest_data = std::fs::read_to_string(scaffold_dir.join("plugin.json")).expect("read");
        let _: PluginManifest = serde_json::from_str(&manifest_data).expect("parse manifest");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
