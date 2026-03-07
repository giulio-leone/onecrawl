# Visual Workflow Builder (Studio) Skill

## Overview

Studio is a visual workflow builder for creating, managing, and executing browser automation workflows. It provides built-in templates for common tasks (login, scraping, forms, monitoring), project management with JSON export/import, workflow validation, and a web-based drag-and-drop editor served on localhost.

## Key Files

- `crates/onecrawl-cdp/src/studio.rs` — Core `StudioWorkspace` with templates and project CRUD
- `crates/onecrawl-mcp-rs/src/handlers/studio.rs` — 8 MCP action handlers
- `crates/onecrawl-cli-rs/src/commands/studio.rs` — CLI studio commands

## API Reference

### MCP Actions

| Action | Description | Parameters |
|--------|-------------|------------|
| `studio_templates` | List all built-in workflow templates | _(none)_ |
| `studio_projects` | List all saved projects | _(none)_ |
| `studio_save` | Save or update a project | `id`, `name`, `description?`, `workflow` (JSON) |
| `studio_load` | Load a project by ID | `id` |
| `studio_delete` | Delete a project | `id` |
| `studio_validate` | Validate workflow structure | `workflow` (JSON) |
| `studio_export` | Export project workflow as JSON | `id` |
| `studio_import` | Create project from workflow JSON | `name`, `workflow` (JSON) |

### CLI Commands

| Command | Description |
|---------|-------------|
| `onecrawl studio open [--port]` | Start web-based studio editor on port (default: 3333) |
| `onecrawl studio templates` | List built-in templates with details |
| `onecrawl studio projects` | List saved projects |
| `onecrawl studio export <id> [--output]` | Export project workflow to file |
| `onecrawl studio import <file> [--name]` | Import workflow from JSON file |
| `onecrawl studio validate <file>` | Validate workflow JSON file |

### Built-in Templates

| ID | Name | Category | Variables |
|----|------|----------|-----------|
| `login-basic` | Basic Login Flow | login | `login_url`, `email_selector`, `email`, `password_selector`, `password`, `submit_selector`, `success_selector` |
| `scrape-list` | List Scraper | scraping | `target_url`, `item_selector` |
| `form-fill` | Smart Form Fill | forms | `form_url`, `field_query`, `field_value`, `confirmation_selector` |
| `monitor-page` | Page Monitor | monitoring | `monitor_url`, `watch_selector`, `screenshot_path` |

### Workflow JSON Format

```json
{
  "name": "My Workflow",
  "steps": [
    {
      "action": {
        "type": "navigate",
        "url": "${target_url}"
      }
    },
    {
      "action": {
        "type": "wait",
        "selector": "${item_selector}",
        "timeout_ms": 5000
      }
    },
    {
      "action": {
        "type": "smart_fill",
        "query": "email field",
        "value": "${email}"
      }
    },
    {
      "action": {
        "type": "screenshot",
        "path": "result.png"
      }
    }
  ]
}
```

### Template Variables

Variables use `${variable_name}` substitution. Each template variable has:

```json
{
  "name": "login_url",
  "description": "URL of the login page",
  "var_type": "url",
  "required": true,
  "default": null
}
```

Variable types: `string`, `url`, `selector`, `number`, `boolean`

### Core Rust API

```rust
use onecrawl_cdp::{StudioWorkspace, StudioProject, WorkflowTemplate};

// Create workspace
let workspace = StudioWorkspace::new("~/.onecrawl/studio")?;

// Get templates
let templates: Vec<WorkflowTemplate> = StudioWorkspace::templates();

// Save project
workspace.save_project(&StudioProject {
    id: "my-scraper".into(),
    name: "My Scraper".into(),
    description: Some("Scrapes product data".into()),
    workflow: serde_json::json!({"name": "...", "steps": [...]}),
    created_at: "2024-01-01T00:00:00Z".into(),
    updated_at: "2024-01-01T00:00:00Z".into(),
    last_run: None,
    run_count: 0,
})?;

// Load, list, delete
let project = workspace.load_project("my-scraper")?;
let all = workspace.list_projects()?;  // Sorted by updated_at DESC
workspace.delete_project("my-scraper")?;

// Export/Import
let json = workspace.export_workflow("my-scraper")?;
let imported = workspace.import_workflow("Imported Flow", &json_string)?;

// Validate
let warnings = StudioWorkspace::validate_workflow(&workflow_json)?;
```

## Architecture

### Workspace Layout

```
~/.onecrawl/studio/
├── my-scraper.json
├── login-flow.json
└── monitor-prices.json
```

Each project is a standalone JSON file named `{id}.json`.

### Project Structure

```rust
StudioProject {
    id: String,                // Alphanumeric + `-` + `_` (becomes filename)
    name: String,              // Display name
    description: Option<String>,
    workflow: serde_json::Value, // The workflow definition
    created_at: String,        // ISO 8601
    updated_at: String,        // ISO 8601
    last_run: Option<String>,  // Last execution timestamp
    run_count: u64,            // Total executions
}
```

### Validation Rules

`validate_workflow()` checks:
1. Workflow is a JSON object
2. Has `name` field (string)
3. Has `steps` field (array)
4. Each step has an `action` object
5. Returns warnings for non-critical issues (e.g., empty step names)

### Web Editor

`onecrawl studio open` starts a local HTTP server serving the visual editor at `http://localhost:{port}/studio`. The editor provides:
- Drag-and-drop step builder
- Template instantiation
- Variable binding UI
- Live preview and validation
- Export to JSON

## Best Practices

- Use templates as starting points — they encode proven patterns for common tasks
- Keep project IDs short and descriptive (they become filenames)
- Use template variables for configurable values instead of hardcoding
- Validate workflows before saving to catch structural issues early
- Export workflows for version control or sharing across environments
- Use `run_count` and `last_run` to track workflow usage
- Organize workflows with descriptive `description` fields

## Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| Project not found | ID mismatch or file deleted | Check `studio_projects` list; verify `~/.onecrawl/studio/` |
| Validation fails | Missing `name` or `steps` in workflow | Ensure top-level `name` (string) and `steps` (array) fields exist |
| Import error | Invalid JSON | Validate JSON syntax before importing; use `studio_validate` |
| Variable not substituted | Template variable syntax error | Use `${variable_name}` format; check variable exists in template |
| Studio server won't start | Port in use | Use `--port` flag with a different port |
| Projects not sorted | Expected order issue | Projects are sorted by `updated_at` descending (most recent first) |
