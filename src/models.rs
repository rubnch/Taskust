use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Represents a single task in the task manager.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Task {
    /// Unique identifier for the task.
    pub id: u64,
    /// The name or description of the task.
    pub name: String,
    /// Optional project or category the task belongs to.
    pub project: Option<String>,
    /// Estimated hours required to complete the task.
    pub expected_hours: f64,
    /// The due date of the task.
    pub due_date: NaiveDate,
    /// Timestamp when the task was created (ISO 8601).
    pub created_at: String,
    /// Whether the task has been completed.
    #[serde(default)]
    pub completed: bool,
    /// Total hours actually worked on the task.
    #[serde(default)]
    pub hours_worked: f64,
    /// Name of the template used to create this task, if any.
    #[serde(default)]
    pub template: Option<String>,
    /// Recurrence pattern (e.g., "daily", "weekly", "monthly").
    #[serde(default)]
    pub recurrence: Option<String>,
}

/// Represents a reusable task template.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Template {
    /// The unique name of the template.
    pub name: String,
    /// Default project for tasks created from this template.
    pub project: Option<String>,
    /// Default estimated duration for tasks created from this template.
    pub default_hours: f64,
}
