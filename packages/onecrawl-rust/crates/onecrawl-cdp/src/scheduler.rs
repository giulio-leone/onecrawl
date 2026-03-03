//! Cron-like task scheduler for browser automation tasks.

use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// A scheduled browser automation task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub name: String,
    pub task_type: String,
    pub config: String,
    pub schedule: TaskSchedule,
    pub status: String,
    pub last_run: Option<f64>,
    pub next_run: f64,
    pub run_count: usize,
    pub max_runs: Option<usize>,
    pub created_at: f64,
}

/// Schedule definition for a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSchedule {
    pub interval_ms: u64,
    pub delay_ms: u64,
    pub max_runs: Option<usize>,
}

/// Result of a single task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration_ms: f64,
    pub timestamp: f64,
}

/// Task scheduler managing a collection of scheduled tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scheduler {
    pub tasks: Vec<ScheduledTask>,
    pub results: Vec<TaskResult>,
    pub status: String,
}

fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1000.0
}

fn gen_id(prefix: &str) -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{prefix}-{ts:x}")
}

impl Scheduler {
    /// Create a new empty scheduler.
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            results: Vec::new(),
            status: "running".to_string(),
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Add a task to the scheduler. Returns the generated task ID.
pub fn add_task(
    scheduler: &mut Scheduler,
    name: &str,
    task_type: &str,
    config: &str,
    schedule: TaskSchedule,
) -> String {
    let now = now_ms();
    let id = gen_id("task");
    let max_runs = schedule.max_runs;
    let next_run = now + schedule.delay_ms as f64;
    let task = ScheduledTask {
        id: id.clone(),
        name: name.to_string(),
        task_type: task_type.to_string(),
        config: config.to_string(),
        schedule,
        status: "active".to_string(),
        last_run: None,
        next_run,
        run_count: 0,
        max_runs,
        created_at: now,
    };
    scheduler.tasks.push(task);
    id
}

/// Remove a task by ID. Returns `true` if found and removed.
pub fn remove_task(scheduler: &mut Scheduler, id: &str) -> bool {
    let before = scheduler.tasks.len();
    scheduler.tasks.retain(|t| t.id != id);
    scheduler.tasks.len() < before
}

/// Pause a task by ID. Returns `true` if found and paused.
pub fn pause_task(scheduler: &mut Scheduler, id: &str) -> bool {
    if let Some(task) = scheduler.tasks.iter_mut().find(|t| t.id == id) {
        task.status = "paused".to_string();
        return true;
    }
    false
}

/// Resume a paused task by ID. Returns `true` if found and resumed.
pub fn resume_task(scheduler: &mut Scheduler, id: &str) -> bool {
    if let Some(task) = scheduler.tasks.iter_mut().find(|t| t.id == id)
        && task.status == "paused"
    {
        task.status = "active".to_string();
        return true;
    }
    false
}
/// Get tasks that are due to execute (active and past their next_run time).
pub fn get_due_tasks(scheduler: &Scheduler) -> Vec<&ScheduledTask> {
    let now = now_ms();
    scheduler
        .tasks
        .iter()
        .filter(|t| t.status == "active" && t.next_run <= now)
        .collect()
}

/// Record the result of a task execution and update task state.
pub fn record_result(scheduler: &mut Scheduler, result: TaskResult) {
    let now = now_ms();
    if let Some(task) = scheduler.tasks.iter_mut().find(|t| t.id == result.task_id) {
        task.run_count += 1;
        task.last_run = Some(now);

        if let Some(max) = task.max_runs
            && task.run_count >= max
        {
            task.status = "completed".to_string();
        }

        if !result.success {
            task.status = "failed".to_string();
        }

        if task.status == "active" && task.schedule.interval_ms > 0 {
            task.next_run = now + task.schedule.interval_ms as f64;
        }
    }
    scheduler.results.push(result);
}

/// Get aggregate stats: counts of active, paused, completed, and failed tasks.
pub fn get_stats(scheduler: &Scheduler) -> HashMap<String, usize> {
    let mut map = HashMap::new();
    map.insert("total".to_string(), scheduler.tasks.len());
    map.insert("results".to_string(), scheduler.results.len());
    for status in &["active", "paused", "completed", "failed"] {
        let count = scheduler
            .tasks
            .iter()
            .filter(|t| t.status == *status)
            .count();
        map.insert((*status).to_string(), count);
    }
    map
}

/// Save the scheduler to a JSON file.
pub fn save_scheduler(scheduler: &Scheduler, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(scheduler)
        .map_err(|e| Error::Cdp(format!("serialize scheduler failed: {e}")))?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load a scheduler from a JSON file.
pub fn load_scheduler(path: &Path) -> Result<Scheduler> {
    let data = std::fs::read_to_string(path)?;
    let scheduler: Scheduler = serde_json::from_str(&data)
        .map_err(|e| Error::Cdp(format!("parse scheduler failed: {e}")))?;
    Ok(scheduler)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_scheduler() {
        let s = Scheduler::new();
        assert!(s.tasks.is_empty());
        assert!(s.results.is_empty());
        assert_eq!(s.status, "running");
    }

    #[test]
    fn test_add_task_returns_id() {
        let mut s = Scheduler::new();
        let id = add_task(
            &mut s,
            "test",
            "navigate",
            "{}",
            TaskSchedule {
                interval_ms: 1000,
                delay_ms: 0,
                max_runs: None,
            },
        );
        assert!(id.starts_with("task-"));
        assert_eq!(s.tasks.len(), 1);
        assert_eq!(s.tasks[0].status, "active");
    }

    #[test]
    fn test_remove_task() {
        let mut s = Scheduler::new();
        let id = add_task(
            &mut s,
            "x",
            "crawl",
            "{}",
            TaskSchedule {
                interval_ms: 0,
                delay_ms: 0,
                max_runs: Some(1),
            },
        );
        assert!(remove_task(&mut s, &id));
        assert!(s.tasks.is_empty());
        assert!(!remove_task(&mut s, "nonexistent"));
    }

    #[test]
    fn test_pause_and_resume() {
        let mut s = Scheduler::new();
        let id = add_task(
            &mut s,
            "t",
            "extract",
            "{}",
            TaskSchedule {
                interval_ms: 5000,
                delay_ms: 0,
                max_runs: None,
            },
        );
        assert!(pause_task(&mut s, &id));
        assert_eq!(s.tasks[0].status, "paused");
        assert!(resume_task(&mut s, &id));
        assert_eq!(s.tasks[0].status, "active");
        assert!(!resume_task(&mut s, "missing"));
    }

    #[test]
    fn test_get_due_tasks() {
        let mut s = Scheduler::new();
        add_task(
            &mut s,
            "due",
            "screenshot",
            "{}",
            TaskSchedule {
                interval_ms: 1000,
                delay_ms: 0,
                max_runs: None,
            },
        );
        // delay_ms = 0 → next_run = now, so it should be due immediately
        let due = get_due_tasks(&s);
        assert_eq!(due.len(), 1);
    }

    #[test]
    fn test_record_result_increments_run_count() {
        let mut s = Scheduler::new();
        let id = add_task(
            &mut s,
            "r",
            "custom",
            "{}",
            TaskSchedule {
                interval_ms: 500,
                delay_ms: 0,
                max_runs: None,
            },
        );
        record_result(
            &mut s,
            TaskResult {
                task_id: id.clone(),
                success: true,
                output: Some("ok".to_string()),
                error: None,
                duration_ms: 42.0,
                timestamp: now_ms(),
            },
        );
        assert_eq!(s.tasks[0].run_count, 1);
        assert!(s.tasks[0].last_run.is_some());
        assert_eq!(s.results.len(), 1);
    }

    #[test]
    fn test_record_result_completes_after_max_runs() {
        let mut s = Scheduler::new();
        let id = add_task(
            &mut s,
            "once",
            "navigate",
            "{}",
            TaskSchedule {
                interval_ms: 0,
                delay_ms: 0,
                max_runs: Some(1),
            },
        );
        record_result(
            &mut s,
            TaskResult {
                task_id: id,
                success: true,
                output: None,
                error: None,
                duration_ms: 10.0,
                timestamp: now_ms(),
            },
        );
        assert_eq!(s.tasks[0].status, "completed");
    }

    #[test]
    fn test_record_result_marks_failed() {
        let mut s = Scheduler::new();
        let id = add_task(
            &mut s,
            "fail",
            "crawl",
            "{}",
            TaskSchedule {
                interval_ms: 1000,
                delay_ms: 0,
                max_runs: None,
            },
        );
        record_result(
            &mut s,
            TaskResult {
                task_id: id,
                success: false,
                output: None,
                error: Some("timeout".to_string()),
                duration_ms: 5000.0,
                timestamp: now_ms(),
            },
        );
        assert_eq!(s.tasks[0].status, "failed");
    }

    #[test]
    fn test_get_stats() {
        let mut s = Scheduler::new();
        add_task(
            &mut s,
            "a",
            "navigate",
            "{}",
            TaskSchedule {
                interval_ms: 0,
                delay_ms: 0,
                max_runs: None,
            },
        );
        let id2 = add_task(
            &mut s,
            "b",
            "crawl",
            "{}",
            TaskSchedule {
                interval_ms: 0,
                delay_ms: 0,
                max_runs: None,
            },
        );
        pause_task(&mut s, &id2);
        let stats = get_stats(&s);
        assert_eq!(stats["total"], 2);
        assert_eq!(stats["active"], 1);
        assert_eq!(stats["paused"], 1);
    }

    #[test]
    fn test_save_and_load() {
        let mut s = Scheduler::new();
        add_task(
            &mut s,
            "persist",
            "extract",
            "{\"url\":\"https://example.com\"}",
            TaskSchedule {
                interval_ms: 60000,
                delay_ms: 1000,
                max_runs: Some(10),
            },
        );
        let dir = std::env::temp_dir();
        let path = dir.join("test_scheduler.json");
        save_scheduler(&s, &path).unwrap();
        let loaded = load_scheduler(&path).unwrap();
        assert_eq!(loaded.tasks.len(), 1);
        assert_eq!(loaded.tasks[0].name, "persist");
        let _ = std::fs::remove_file(&path);
    }
}
