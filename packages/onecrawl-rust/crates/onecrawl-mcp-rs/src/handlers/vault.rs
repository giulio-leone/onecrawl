//! Handler implementations for the `vault` super-tool.

use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{json_ok, text_ok, McpResult};
use crate::OneCrawlMcp;

fn vault_path_or_default(path: &Option<String>) -> String {
    path.as_deref()
        .map(String::from)
        .unwrap_or_else(|| {
            onecrawl_crypto::vault::default_vault_path()
                .to_string_lossy()
                .into_owned()
        })
}

impl OneCrawlMcp {
    // ════════════════════════════════════════════════════════════════
    //  Vault handlers
    // ════════════════════════════════════════════════════════════════

    pub(crate) fn vault_create(
        &self,
        p: VaultCreateParams,
    ) -> Result<CallToolResult, McpError> {
        let path = vault_path_or_default(&p.path);
        let vault = onecrawl_crypto::vault::Vault::create(&path, &p.password).mcp()?;

        json_ok(&serde_json::json!({
            "action": "vault_create",
            "path": path,
            "entries": vault.len(),
            "status": "created",
        }))
    }

    pub(crate) fn vault_open(
        &self,
        p: VaultOpenParams,
    ) -> Result<CallToolResult, McpError> {
        let path = vault_path_or_default(&p.path);
        let vault = onecrawl_crypto::vault::Vault::open(&path, &p.password).mcp()?;

        json_ok(&serde_json::json!({
            "action": "vault_open",
            "path": path,
            "entries": vault.len(),
            "status": "opened",
        }))
    }

    pub(crate) fn vault_set(
        &self,
        p: VaultSetParams,
    ) -> Result<CallToolResult, McpError> {
        let path = vault_path_or_default(&p.path);
        let mut vault = open_or_create(&path, &p.password).mcp()?;
        vault.set(&p.key, &p.value, p.category.as_deref()).mcp()?;

        json_ok(&serde_json::json!({
            "action": "vault_set",
            "key": p.key,
            "category": p.category,
            "entries": vault.len(),
            "status": "stored",
        }))
    }

    pub(crate) fn vault_get(
        &self,
        p: VaultGetParams,
    ) -> Result<CallToolResult, McpError> {
        let path = vault_path_or_default(&p.path);
        let vault = onecrawl_crypto::vault::Vault::open(&path, &p.password).mcp()?;

        match vault.get(&p.key) {
            Some(entry) => json_ok(&serde_json::json!({
                "action": "vault_get",
                "key": entry.key,
                "value": entry.value,
                "category": entry.category,
                "created_at": entry.created_at,
                "updated_at": entry.updated_at,
            })),
            None => text_ok(format!("key '{}' not found in vault", p.key)),
        }
    }

    pub(crate) fn vault_delete(
        &self,
        p: VaultDeleteParams,
    ) -> Result<CallToolResult, McpError> {
        let path = vault_path_or_default(&p.path);
        let mut vault = onecrawl_crypto::vault::Vault::open(&path, &p.password).mcp()?;
        vault.delete(&p.key).mcp()?;

        json_ok(&serde_json::json!({
            "action": "vault_delete",
            "key": p.key,
            "entries": vault.len(),
            "status": "deleted",
        }))
    }

    pub(crate) fn vault_list(
        &self,
        p: VaultListParams,
    ) -> Result<CallToolResult, McpError> {
        let path = vault_path_or_default(&p.path);
        let vault = onecrawl_crypto::vault::Vault::open(&path, &p.password).mcp()?;

        let entries = match &p.category {
            Some(cat) => vault.list_by_category(cat),
            None => vault.list(),
        };

        let expired = vault.check_expired();

        json_ok(&serde_json::json!({
            "action": "vault_list",
            "entries": entries,
            "count": entries.len(),
            "expired_keys": expired,
        }))
    }

    pub(crate) fn vault_use(
        &self,
        p: VaultUseParams,
    ) -> Result<CallToolResult, McpError> {
        let path = vault_path_or_default(&p.path);
        let vault = onecrawl_crypto::vault::Vault::open(&path, &p.password).mcp()?;
        let variables = vault.export_for_workflow(&p.service);

        json_ok(&serde_json::json!({
            "action": "vault_use",
            "service": p.service,
            "variables": variables,
            "count": variables.len(),
        }))
    }

    pub(crate) fn vault_change_password(
        &self,
        p: VaultChangePasswordParams,
    ) -> Result<CallToolResult, McpError> {
        let path = vault_path_or_default(&p.path);
        let mut vault = onecrawl_crypto::vault::Vault::open(&path, &p.password).mcp()?;
        vault.change_password(&p.new_password).mcp()?;

        json_ok(&serde_json::json!({
            "action": "vault_change_password",
            "path": path,
            "status": "password_changed",
        }))
    }

    pub(crate) fn vault_import_env(
        &self,
        p: VaultImportEnvParams,
    ) -> Result<CallToolResult, McpError> {
        let path = vault_path_or_default(&p.path);
        let prefix = p.prefix.as_deref().unwrap_or("ONECRAWL_VAULT_");
        let mut vault = open_or_create(&path, &p.password).mcp()?;
        let count = vault.import_env(prefix).mcp()?;

        json_ok(&serde_json::json!({
            "action": "vault_import_env",
            "prefix": prefix,
            "imported": count,
            "total_entries": vault.len(),
            "status": "imported",
        }))
    }
}

/// Open an existing vault or create a new one if it doesn't exist.
fn open_or_create(path: &str, password: &str) -> onecrawl_core::Result<onecrawl_crypto::vault::Vault> {
    if std::path::Path::new(path).exists() {
        onecrawl_crypto::vault::Vault::open(path, password)
            .map_err(|e| onecrawl_core::Error::Crypto(format!("Failed to open vault (wrong password?): {}", e)))
    } else {
        onecrawl_crypto::vault::Vault::create(path, password)
            .map_err(|e| onecrawl_core::Error::Crypto(format!("Failed to create vault: {}", e)))
    }
}
