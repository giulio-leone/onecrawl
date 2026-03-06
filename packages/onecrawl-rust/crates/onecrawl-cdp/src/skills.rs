//! Skill package format, discovery, and built-in skill definitions.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A skill package definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    /// MCP tools this skill provides.
    #[serde(default)]
    pub tools: Vec<SkillTool>,
    /// Required capabilities (e.g. "browser", "network", "stealth").
    #[serde(default)]
    pub requires: Vec<String>,
    /// Author info.
    pub author: Option<String>,
}

/// A single tool exposed by a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTool {
    pub name: String,
    pub description: String,
    /// JS code to execute for this tool.
    pub script: Option<String>,
    /// CLI command to run.
    pub command: Option<String>,
}

/// A skill loaded from the filesystem.
pub struct LoadedSkill {
    pub manifest: SkillManifest,
    pub path: PathBuf,
    pub readme: Option<String>,
}

/// Registry that discovers and holds skill packages.
pub struct SkillRegistry {
    skills: Vec<LoadedSkill>,
}

impl SkillRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { skills: Vec::new() }
    }

    /// Discover skills from a directory tree.
    ///
    /// Walks `dir` looking for `skill.json` files. Each parent directory is
    /// treated as a skill package and may optionally contain a `README.md`.
    pub fn discover(dir: &Path) -> Result<Self, String> {
        let mut skills = Vec::new();
        Self::walk_dir(dir, &mut skills)?;
        Ok(Self { skills })
    }

    /// List all discovered skills.
    pub fn list(&self) -> &[LoadedSkill] {
        &self.skills
    }

    /// Find a skill by name.
    pub fn find(&self, name: &str) -> Option<&LoadedSkill> {
        self.skills.iter().find(|s| s.manifest.name == name)
    }

    /// Load a single skill from a directory containing `skill.json`.
    pub fn load_skill(dir: &Path) -> Result<LoadedSkill, String> {
        let manifest_path = dir.join("skill.json");
        let content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("cannot read {}: {e}", manifest_path.display()))?;
        let manifest: SkillManifest = serde_json::from_str(&content)
            .map_err(|e| format!("invalid skill.json in {}: {e}", dir.display()))?;

        let readme_path = dir.join("README.md");
        let readme = std::fs::read_to_string(readme_path).ok();

        Ok(LoadedSkill {
            manifest,
            path: dir.to_path_buf(),
            readme,
        })
    }

    /// Return built-in skill manifests.
    pub fn builtins() -> Vec<SkillManifest> {
        vec![
            serde_json::from_str(BUILTIN_BROWSER_AUTOMATION).expect("valid built-in manifest"),
            serde_json::from_str(BUILTIN_WEB_SCRAPING).expect("valid built-in manifest"),
            serde_json::from_str(BUILTIN_QA_TESTING).expect("valid built-in manifest"),
        ]
    }

    // ── private ─────────────────────────────────────────────────

    fn walk_dir(dir: &Path, out: &mut Vec<LoadedSkill>) -> Result<(), String> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("cannot read directory {}: {e}", dir.display()))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("read_dir entry error: {e}"))?;
            let path = entry.path();
            if path.is_dir() {
                if path.join("skill.json").exists() {
                    match Self::load_skill(&path) {
                        Ok(skill) => out.push(skill),
                        Err(e) => eprintln!("warning: skipping {}: {e}", path.display()),
                    }
                }
                // recurse into subdirectories
                Self::walk_dir(&path, out)?;
            }
        }
        Ok(())
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Built-in skill manifests ───────────────────────────────────

const BUILTIN_BROWSER_AUTOMATION: &str = r#"{
    "name": "browser-automation",
    "version": "1.0.0",
    "description": "Generic web automation: navigate, click, fill forms, take screenshots",
    "tools": [
        { "name": "navigate", "description": "Navigate to a URL" },
        { "name": "click",    "description": "Click an element by selector" },
        { "name": "fill",     "description": "Fill a form field with text" },
        { "name": "screenshot","description": "Capture a screenshot of the page" },
        { "name": "evaluate", "description": "Execute JavaScript in the page context" }
    ],
    "requires": ["browser"],
    "author": "OneCrawl"
}"#;

const BUILTIN_WEB_SCRAPING: &str = r#"{
    "name": "web-scraping",
    "version": "1.0.0",
    "description": "Extraction-focused automation: CSS queries, text extraction, structured data",
    "tools": [
        { "name": "css_select",      "description": "Query elements with a CSS selector" },
        { "name": "extract_text",    "description": "Extract visible text from elements" },
        { "name": "extract_links",   "description": "Extract all links from the page" },
        { "name": "structured_data", "description": "Extract JSON-LD, OpenGraph, and Twitter Card metadata" },
        { "name": "extract_table",   "description": "Extract tabular data from HTML tables" }
    ],
    "requires": ["browser", "parser"],
    "author": "OneCrawl"
}"#;

const BUILTIN_QA_TESTING: &str = r#"{
    "name": "qa-testing",
    "version": "1.0.0",
    "description": "Visual regression, screenshot diff, and accessibility auditing",
    "tools": [
        { "name": "screenshot_diff", "description": "Compare screenshots for visual regression" },
        { "name": "a11y_audit",      "description": "Run an accessibility audit on the page" },
        { "name": "snapshot",        "description": "Take a DOM snapshot for comparison" },
        { "name": "compare",         "description": "Compare two DOM snapshots and report differences" }
    ],
    "requires": ["browser"],
    "author": "OneCrawl"
}"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_parse_correctly() {
        let builtins = SkillRegistry::builtins();
        assert_eq!(builtins.len(), 3);
        assert_eq!(builtins[0].name, "browser-automation");
        assert_eq!(builtins[1].name, "web-scraping");
        assert_eq!(builtins[2].name, "qa-testing");
        assert!(!builtins[0].tools.is_empty());
    }

    #[test]
    fn empty_registry() {
        let reg = SkillRegistry::new();
        assert!(reg.list().is_empty());
        assert!(reg.find("missing").is_none());
    }

    #[test]
    fn discover_from_temp_dir() {
        let tmp = std::env::temp_dir().join("onecrawl_skills_test");
        let skill_dir = tmp.join("my-skill");
        let _ = std::fs::create_dir_all(&skill_dir);
        std::fs::write(
            skill_dir.join("skill.json"),
            r#"{"name":"my-skill","version":"0.1.0","description":"test"}"#,
        )
        .unwrap();
        std::fs::write(skill_dir.join("README.md"), "# My Skill").unwrap();

        let reg = SkillRegistry::discover(&tmp).unwrap();
        assert_eq!(reg.list().len(), 1);
        let s = reg.find("my-skill").unwrap();
        assert_eq!(s.manifest.version, "0.1.0");
        assert_eq!(s.readme.as_deref(), Some("# My Skill"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn load_single_skill() {
        let tmp = std::env::temp_dir().join("onecrawl_skill_load_test");
        let _ = std::fs::create_dir_all(&tmp);
        std::fs::write(
            tmp.join("skill.json"),
            r#"{"name":"single","version":"2.0.0","description":"d","requires":["browser"]}"#,
        )
        .unwrap();

        let skill = SkillRegistry::load_skill(&tmp).unwrap();
        assert_eq!(skill.manifest.name, "single");
        assert_eq!(skill.manifest.requires, vec!["browser"]);
        assert!(skill.readme.is_none());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
