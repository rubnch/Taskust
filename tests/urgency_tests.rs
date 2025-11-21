use taskust::urgency::compute_urgency;
use taskust::models::Task;
use chrono::{Local, Duration};

#[test]
fn test_urgency_calculation() {
    let now = Local::now();
    let today = now.date_naive();
    let due_tomorrow = today + Duration::days(1);
    
    let task = Task {
        id: 1,
        name: "Test".into(),
        project: None,
        expected_hours: 1.0,
        due_date: due_tomorrow,
        created_at: now.to_rfc3339(),
        completed: false,
        hours_worked: 0.0,
        template: None,
        recurrence: None,
        completed_at: None,
    };

    let urgency = compute_urgency(&task);
    // Urgency should be positive
    assert!(urgency > 0.0);
}

#[test]
fn test_urgency_overdue() {
    let now = Local::now();
    let today = now.date_naive();
    let due_yesterday = today - Duration::days(1);
    
    let task = Task {
        id: 1,
        name: "Test".into(),
        project: None,
        expected_hours: 1.0,
        due_date: due_yesterday,
        created_at: now.to_rfc3339(),
        completed: false,
        hours_worked: 0.0,
        template: None,
        recurrence: None,
        completed_at: None,
    };

    let urgency = compute_urgency(&task);
    // Should be very high because it's overdue (base 100 + ...)
    assert!(urgency > 100.0);
}
