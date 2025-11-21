use chrono::Local;
use crate::models::Task;

/// Calculates the urgency score for a given task.
///
/// The score is based on:
/// - **Due Date**: Closer deadlines yield higher scores. Overdue tasks get a significant boost.
/// - **Expected Duration**: Longer tasks slightly increase urgency.
///
/// # Returns
/// - `-1.0` if the task is completed.
/// - A positive float representing urgency (higher is more urgent).
pub fn compute_urgency(task: &Task) -> f64 {
    if task.completed {
        return -1.0;
    }
    let today = Local::now().date_naive();
    let days_left = (task.due_date - today).num_days();
    let base = if days_left <= 0 {
        // overdue or due today -> high urgency
        100.0 + (task.expected_hours) + (days_left.abs() as f64 * 2.0)
    } else {
        // closer due date -> higher urgency; longer tasks increase urgency
        (1.0 / (days_left as f64)) * 10.0 * (1.0 + task.expected_hours / 8.0)
    };
    // clamp to a reasonable range
    if base.is_finite() { base } else { 0.0 }
}

