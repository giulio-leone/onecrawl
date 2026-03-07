//! Visual Workflow Builder — template library and project management.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Validate that an ID contains only safe characters: `[a-zA-Z0-9_-]`.
/// Prevents path traversal and XSS when IDs are embedded in HTML/JS contexts.
fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Workflow template for the studio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Category: "login", "scraping", "forms", "monitoring".
    pub category: String,
    pub tags: Vec<String>,
    /// The actual workflow JSON.
    pub workflow: serde_json::Value,
    pub variables: Vec<TemplateVariable>,
    pub preview_image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub description: String,
    /// "string", "url", "selector", "number", "boolean"
    pub var_type: String,
    pub default: Option<String>,
    pub required: bool,
}

/// Studio project (saved workflow + metadata).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioProject {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub workflow: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
    pub last_run: Option<String>,
    pub run_count: u64,
}

/// Studio workspace manager.
pub struct StudioWorkspace {
    workspace_dir: PathBuf,
}

fn iso_now() -> String {
    crate::util::iso_now()
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "HOME environment variable not set".to_string())
}

impl StudioWorkspace {
    pub fn new(workspace_dir: &str) -> Result<Self, String> {
        let path = if workspace_dir.starts_with('~') {
            let home = home_dir()?;
            home.join(workspace_dir.trim_start_matches("~/"))
        } else {
            PathBuf::from(workspace_dir)
        };
        std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
        Ok(Self { workspace_dir: path })
    }

    /// Get built-in workflow templates.
    pub fn templates() -> Vec<WorkflowTemplate> {
        vec![
            WorkflowTemplate {
                id: "login-basic".into(),
                name: "Basic Login Flow".into(),
                description: "Navigate to a page and fill in login credentials".into(),
                category: "login".into(),
                tags: vec!["auth".into(), "login".into(), "form".into()],
                workflow: serde_json::json!({
                    "name": "Basic Login",
                    "steps": [
                        { "name": "Navigate to login", "action": { "type": "navigate", "url": "${login_url}" } },
                        { "name": "Enter email", "action": { "type": "type", "selector": "${email_selector}", "text": "${email}" } },
                        { "name": "Enter password", "action": { "type": "type", "selector": "${password_selector}", "text": "${password}" } },
                        { "name": "Click submit", "action": { "type": "click", "selector": "${submit_selector}" } },
                        { "name": "Wait for dashboard", "action": { "type": "wait_for_selector", "selector": "${success_selector}", "timeout": 10000 } }
                    ]
                }),
                variables: vec![
                    TemplateVariable { name: "login_url".into(), description: "Login page URL".into(), var_type: "url".into(), default: None, required: true },
                    TemplateVariable { name: "email_selector".into(), description: "Email input selector".into(), var_type: "selector".into(), default: Some("input[type=email]".into()), required: false },
                    TemplateVariable { name: "email".into(), description: "Email address".into(), var_type: "string".into(), default: None, required: true },
                    TemplateVariable { name: "password_selector".into(), description: "Password input selector".into(), var_type: "selector".into(), default: Some("input[type=password]".into()), required: false },
                    TemplateVariable { name: "password".into(), description: "Password".into(), var_type: "string".into(), default: None, required: true },
                    TemplateVariable { name: "submit_selector".into(), description: "Submit button selector".into(), var_type: "selector".into(), default: Some("button[type=submit]".into()), required: false },
                    TemplateVariable { name: "success_selector".into(), description: "Element visible after success".into(), var_type: "selector".into(), default: None, required: true },
                ],
                preview_image: None,
            },
            WorkflowTemplate {
                id: "scrape-list".into(),
                name: "List Page Scraper".into(),
                description: "Extract items from a paginated list page".into(),
                category: "scraping".into(),
                tags: vec!["scrape".into(), "extract".into(), "pagination".into()],
                workflow: serde_json::json!({
                    "name": "List Scraper",
                    "steps": [
                        { "name": "Navigate", "action": { "type": "navigate", "url": "${target_url}" } },
                        { "name": "Wait for content", "action": { "type": "wait_for_selector", "selector": "${item_selector}", "timeout": 10000 } },
                        { "name": "Extract items", "action": { "type": "extract", "selector": "${item_selector}", "attributes": ["textContent", "href"] }, "save_as": "items" },
                        { "name": "Screenshot results", "action": { "type": "screenshot", "path": "scrape-result.png" } }
                    ]
                }),
                variables: vec![
                    TemplateVariable { name: "target_url".into(), description: "Page to scrape".into(), var_type: "url".into(), default: None, required: true },
                    TemplateVariable { name: "item_selector".into(), description: "CSS selector for items".into(), var_type: "selector".into(), default: None, required: true },
                ],
                preview_image: None,
            },
            WorkflowTemplate {
                id: "form-fill".into(),
                name: "Form Filler".into(),
                description: "Fill and submit a web form".into(),
                category: "forms".into(),
                tags: vec!["form".into(), "fill".into(), "submit".into()],
                workflow: serde_json::json!({
                    "name": "Form Fill",
                    "steps": [
                        { "name": "Navigate to form", "action": { "type": "navigate", "url": "${form_url}" } },
                        { "name": "Fill form fields", "action": { "type": "smart_fill", "query": "${field_query}", "value": "${field_value}" } },
                        { "name": "Submit", "action": { "type": "smart_click", "query": "submit" } },
                        { "name": "Wait for confirmation", "action": { "type": "wait_for_selector", "selector": "${confirmation_selector}", "timeout": 10000 } }
                    ]
                }),
                variables: vec![
                    TemplateVariable { name: "form_url".into(), description: "Form page URL".into(), var_type: "url".into(), default: None, required: true },
                    TemplateVariable { name: "field_query".into(), description: "Field to fill".into(), var_type: "string".into(), default: None, required: true },
                    TemplateVariable { name: "field_value".into(), description: "Value to enter".into(), var_type: "string".into(), default: None, required: true },
                    TemplateVariable { name: "confirmation_selector".into(), description: "Success indicator".into(), var_type: "selector".into(), default: None, required: false },
                ],
                preview_image: None,
            },
            WorkflowTemplate {
                id: "monitor-page".into(),
                name: "Page Monitor".into(),
                description: "Monitor a page for changes and take screenshots".into(),
                category: "monitoring".into(),
                tags: vec!["monitor".into(), "watch".into(), "screenshot".into()],
                workflow: serde_json::json!({
                    "name": "Page Monitor",
                    "steps": [
                        { "name": "Navigate", "action": { "type": "navigate", "url": "${monitor_url}" } },
                        { "name": "Wait for load", "action": { "type": "wait_for_selector", "selector": "body", "timeout": 10000 } },
                        { "name": "Screenshot", "action": { "type": "screenshot", "path": "${screenshot_path}" } },
                        { "name": "Extract text", "action": { "type": "extract", "selector": "${watch_selector}", "attributes": ["textContent"] }, "save_as": "content" }
                    ]
                }),
                variables: vec![
                    TemplateVariable { name: "monitor_url".into(), description: "URL to monitor".into(), var_type: "url".into(), default: None, required: true },
                    TemplateVariable { name: "watch_selector".into(), description: "Element to watch".into(), var_type: "selector".into(), default: Some("body".into()), required: false },
                    TemplateVariable { name: "screenshot_path".into(), description: "Screenshot save path".into(), var_type: "string".into(), default: Some("monitor.png".into()), required: false },
                ],
                preview_image: None,
            },
        ]
    }

    /// Save a project.
    pub fn save_project(&self, project: &StudioProject) -> Result<(), String> {
        if !is_safe_id(&project.id) {
            return Err("Invalid project ID: only alphanumeric, dash, and underscore allowed".into());
        }
        let path = self.workspace_dir.join(format!("{}.json", project.id));
        let content = serde_json::to_string_pretty(project).map_err(|e| e.to_string())?;
        std::fs::write(path, content).map_err(|e| e.to_string())
    }

    /// Load a project.
    pub fn load_project(&self, id: &str) -> Result<StudioProject, String> {
        if !is_safe_id(id) {
            return Err("Invalid project ID: only alphanumeric, dash, and underscore allowed".into());
        }
        let path = self.workspace_dir.join(format!("{}.json", id));
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    /// List all projects.
    pub fn list_projects(&self) -> Result<Vec<StudioProject>, String> {
        let mut projects = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.workspace_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().map(|e| e == "json").unwrap_or(false) {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        if let Ok(project) = serde_json::from_str::<StudioProject>(&content) {
                            projects.push(project);
                        }
                    }
                }
            }
        }
        projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(projects)
    }

    /// Delete a project.
    pub fn delete_project(&self, id: &str) -> Result<(), String> {
        if !is_safe_id(id) {
            return Err("Invalid project ID: only alphanumeric, dash, and underscore allowed".into());
        }
        let path = self.workspace_dir.join(format!("{}.json", id));
        std::fs::remove_file(path).map_err(|e| e.to_string())
    }

    /// Export a project as workflow JSON.
    pub fn export_workflow(&self, id: &str) -> Result<String, String> {
        let project = self.load_project(id)?;
        serde_json::to_string_pretty(&project.workflow).map_err(|e| e.to_string())
    }

    /// Import a workflow JSON as a new project.
    pub fn import_workflow(&self, name: &str, workflow_json: &str) -> Result<StudioProject, String> {
        let workflow: serde_json::Value =
            serde_json::from_str(workflow_json).map_err(|e| e.to_string())?;
        let now = iso_now();
        let id: String = name
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect();
        let project = StudioProject {
            id,
            name: name.to_string(),
            description: None,
            workflow,
            created_at: now.clone(),
            updated_at: now,
            last_run: None,
            run_count: 0,
        };
        self.save_project(&project)?;
        Ok(project)
    }

    /// Validate a workflow, returning warnings/suggestions.
    pub fn validate_workflow(workflow: &serde_json::Value) -> Result<Vec<String>, String> {
        let mut warnings = Vec::new();
        if let Some(steps) = workflow.get("steps").and_then(|s| s.as_array()) {
            if steps.is_empty() {
                warnings.push("Workflow has no steps".to_string());
            }
            for (i, step) in steps.iter().enumerate() {
                if step.get("action").is_none() {
                    warnings.push(format!("Step {} has no action", i + 1));
                }
            }
        } else {
            return Err("Workflow must have a 'steps' array".into());
        }
        Ok(warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_not_empty() {
        let templates = StudioWorkspace::templates();
        assert!(!templates.is_empty());
        assert_eq!(templates[0].id, "login-basic");
    }

    #[test]
    fn validate_workflow_ok() {
        let wf = serde_json::json!({
            "name": "test",
            "steps": [{ "name": "s1", "action": { "type": "navigate", "url": "http://x" } }]
        });
        let warnings = StudioWorkspace::validate_workflow(&wf).expect("should be valid");
        assert!(warnings.is_empty());
    }

    #[test]
    fn validate_workflow_no_steps() {
        let wf = serde_json::json!({ "name": "test" });
        let result = StudioWorkspace::validate_workflow(&wf);
        assert!(result.is_err());
    }

    #[test]
    fn validate_workflow_empty_steps() {
        let wf = serde_json::json!({ "name": "test", "steps": [] });
        let warnings = StudioWorkspace::validate_workflow(&wf).expect("should be valid");
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn iso_now_format() {
        let ts = iso_now();
        assert!(ts.contains('T'));
        assert!(ts.ends_with('Z'));
    }
}
