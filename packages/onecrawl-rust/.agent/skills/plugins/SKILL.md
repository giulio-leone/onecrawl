# Plugin System Skill

## Overview

The Plugin System enables extending OneCrawl with custom commands, MCP actions, and event hooks via JSON manifest-based plugins. Plugins are installed from local directories, managed through a registry, and support scaffolding from built-in templates. Each plugin declares its commands, actions, and hooks in a `plugin.json` manifest.

## Key Files

- `crates/onecrawl-cdp/src/plugin.rs` — Core `PluginRegistry` with install, lifecycle, and execution
- `crates/onecrawl-mcp-rs/src/handlers/plugin.rs` — 9 MCP action handlers
- `crates/onecrawl-cli-rs/src/commands/plugin.rs` — CLI plugin commands

## API Reference

### MCP Actions

| Action | Description | Parameters |
|--------|-------------|------------|
| `plugin_install` | Install plugin from local directory | `path` |
| `plugin_uninstall` | Remove an installed plugin | `name` |
| `plugin_enable` | Activate a plugin | `name` |
| `plugin_disable` | Deactivate a plugin | `name` |
| `plugin_list` | List all plugins with status | _(none)_ |
| `plugin_info` | Get plugin details and manifest | `name` |
| `plugin_create` | Generate plugin scaffold from template | `name`, `path?` |
| `plugin_execute` | Execute a plugin action | `plugin`, `action`, `params` (JSON) |
| `plugin_configure` | Set plugin configuration | `name`, `config` (JSON) |

### CLI Commands

| Command | Description |
|---------|-------------|
| `onecrawl plugin install <path>` | Install plugin from local directory |
| `onecrawl plugin uninstall <name>` | Remove plugin |
| `onecrawl plugin enable <name>` | Activate plugin |
| `onecrawl plugin disable <name>` | Deactivate plugin |
| `onecrawl plugin list` | List all plugins with status |
| `onecrawl plugin info <name>` | Show plugin manifest as JSON |
| `onecrawl plugin create <name> [--path]` | Generate scaffold (plugin.json, handlers/, README) |
| `onecrawl plugin run <plugin> <action> [params]` | Execute plugin action with JSON params |
| `onecrawl plugin config <name> [--set json]` | Get or set plugin configuration |

### Plugin Manifest (`plugin.json`)

```json
{
  "name": "my-plugin",
  "version": "1.0.0",
  "description": "Custom automation plugin",
  "author": "Your Name",
  "license": "MIT",
  "onecrawl_version": ">=3.0.0",
  "commands": [
    {
      "name": "my-command",
      "description": "Run custom automation",
      "args": [
        { "name": "url", "arg_type": "string", "required": true },
        { "name": "headless", "arg_type": "string", "required": false, "default": "true" }
      ],
      "handler": "handlers/my-command.json"
    }
  ],
  "actions": [
    {
      "name": "my_action",
      "description": "Custom MCP action",
      "params": [
        { "name": "target", "param_type": "string", "required": true },
        { "name": "timeout", "param_type": "string", "required": false, "default": "5000" }
      ],
      "handler": "handlers/my-action.json"
    }
  ],
  "hooks": [
    {
      "event": "page:loaded",
      "handler": "handlers/on-page-load.json",
      "filter": "*.example.com"
    }
  ],
  "dependencies": [],
  "config_schema": {}
}
```

### Built-in Templates

| Template | Description |
|----------|-------------|
| `captcha-solver` | CAPTCHA detection and solving automation |
| `auth-flow` | Authentication flow automation |
| `data-extractor` | Structured data extraction pipeline |
| `notification` | Event notification and alerting |

### Core Rust API

```rust
use onecrawl_cdp::{PluginRegistry, PluginManifest, PluginStatus};

// Create or load registry
let mut registry = PluginRegistry::new("~/.onecrawl/plugins")?;

// Install from local path
let plugin = registry.install_local("/path/to/my-plugin")?;

// Lifecycle management
registry.enable("my-plugin")?;
registry.disable("my-plugin")?;
registry.uninstall("my-plugin")?;

// List and query
let all_plugins = registry.list();
let info = registry.get("my-plugin");

// Get active registrations
let commands = registry.active_commands();  // Vec<(plugin_name, PluginCommand)>
let actions = registry.active_actions();    // Vec<(plugin_name, PluginAction)>
let hooks = registry.active_hooks();        // Vec<(plugin_name, PluginHook)>

// Execute action
let result = registry.execute_action("my-plugin", "my_action", json!({"target": "#btn"})).await?;

// Configure
registry.configure("my-plugin", json!({"api_key": "..."}))?;

// Scaffold
registry.create_scaffold("new-plugin", "/output/path")?;

// Persist registry
registry.save()?;
```

## Architecture

### Plugin Lifecycle

```
Install → Installed → Enable → Active → Disable → Disabled → Uninstall → Removed
                                  ↓
                              Error(msg)
```

- **PluginStatus**: `Installed` | `Active` | `Disabled` | `Error(String)`
- Only `Active` plugins contribute commands, actions, and hooks

### File Structure

```
~/.onecrawl/plugins/
├── registry.json           # Plugin registry state
├── my-plugin/
│   ├── plugin.json         # Manifest
│   ├── handlers/
│   │   ├── my-command.json # Workflow handler
│   │   └── my-action.json  # Action handler
│   └── README.md
```

### Security

- **Name validation**: Plugin names must be alphanumeric + `-` + `_` only
- **Path traversal protection**: Handler paths checked for `..` sequences
- **Sandboxing**: Handlers are JSON workflow files (MVP); no arbitrary code execution
- **Registry isolation**: Each plugin installed in its own directory

### Handler Execution

Handlers are JSON workflow files containing step definitions. The `execute_action` method:
1. Validates plugin exists and is `Active`
2. Finds the action by name in the manifest
3. Resolves handler file path (relative to plugin directory)
4. Returns handler content + params with `status: "ready"`

## Best Practices

- Use one of the built-in templates as a starting point with `plugin_create`
- Follow the `service.action` naming convention for actions (e.g., `captcha.solve`)
- Always include a `description` for commands and actions — it appears in CLI help and MCP tool listings
- Set `required: true` only for parameters that have no sensible default
- Use event hooks for side-effect automation (screenshots on page load, logging, etc.)
- Test plugins with `plugin_execute` before enabling hooks
- Version your plugins and set `onecrawl_version` for compatibility tracking

## Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| Install fails: "invalid name" | Plugin name contains special characters | Use only alphanumeric, `-`, `_` in name |
| Plugin not found after install | Registry not saved | Call `registry.save()` or re-install |
| Action execution returns "not active" | Plugin is `Disabled` or `Installed` | Enable with `plugin_enable` |
| Handler file not found | Path mismatch in manifest | Verify `handler` path is relative to plugin directory |
| Hooks not firing | Plugin disabled or pattern mismatch | Check plugin status and hook `filter` pattern |
| Config not persisting | Registry not saved after configure | Ensure `save()` is called after `configure()` |
