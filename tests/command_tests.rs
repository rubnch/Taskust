use taskust::commands::*;
use taskust::storage::{load_tasks, load_templates};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

// Use a mutex to ensure tests run serially since they modify the environment variable
static TEST_MUTEX: Mutex<()> = Mutex::new(());

fn with_test_db<F>(test_name: &str, f: F)
where
    F: FnOnce(PathBuf),
{
    let _guard = TEST_MUTEX.lock().unwrap();
    
    let mut db_path = env::temp_dir();
    db_path.push(format!("taskust_test_{}.json", test_name));
    
    // Set env var
    env::set_var("TASKS_DB", db_path.to_str().unwrap());
    
    // Clean up before test
    if db_path.exists() {
        fs::remove_file(&db_path).unwrap();
    }
    let mut archive_path = db_path.clone();
    archive_path.pop();
    archive_path.push("archive.json");
    if archive_path.exists() {
        fs::remove_file(&archive_path).unwrap();
    }
    let mut templates_path = db_path.clone();
    templates_path.pop();
    templates_path.push("templates.json");
    if templates_path.exists() {
        fs::remove_file(&templates_path).unwrap();
    }

    // Run test
    f(db_path.clone());

    // Clean up after test
    if db_path.exists() {
        fs::remove_file(&db_path).unwrap();
    }
    if archive_path.exists() {
        fs::remove_file(&archive_path).unwrap();
    }
    if templates_path.exists() {
        fs::remove_file(&templates_path).unwrap();
    }
    env::remove_var("TASKS_DB");
}

#[test]
fn test_add_and_list() {
    with_test_db("add_list", |_path| {
        cmd_add("Test Task".into(), Some("Project".into()), Some(1.0), "2025-12-01".into(), None, None, true);
        
        let tasks = load_tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, "Test Task");
        assert_eq!(tasks[0].project, Some("Project".into()));
    });
}

#[test]
fn test_complete_task() {
    with_test_db("complete", |_path| {
        cmd_add("Task to complete".into(), None, None, "2025-12-01".into(), None, None, true);
        let tasks = load_tasks();
        let id = tasks[0].id;

        cmd_complete(id, true);
        
        let tasks = load_tasks();
        assert!(tasks[0].completed);
        assert!(tasks[0].completed_at.is_some());
    });
}

#[test]
fn test_archive_task() {
    with_test_db("archive", |_path| {
        cmd_add("Task to archive".into(), None, None, "2025-12-01".into(), None, None, true);
        let tasks = load_tasks();
        let id = tasks[0].id;

        cmd_complete(id, true);
        
        // Archive all completed tasks
        cmd_archive(None, true);

        let tasks = load_tasks();
        assert!(tasks.is_empty());

        let archived = taskust::storage::load_archived_tasks();
        assert_eq!(archived.len(), 1);
        assert_eq!(archived[0].id, id);
    });
}

#[test]
fn test_recurrence() {
    with_test_db("recurrence", |_path| {
        cmd_add("Recurring Task".into(), None, None, "2025-12-01".into(), None, Some("daily".into()), true);
        let tasks = load_tasks();
        let id = tasks[0].id;

        cmd_complete(id, true);

        let tasks = load_tasks();
        // Should have 2 tasks: one completed, one new
        assert_eq!(tasks.len(), 2);
        
        let completed = tasks.iter().find(|t| t.completed).unwrap();
        let new_task = tasks.iter().find(|t| !t.completed).unwrap();
        
        assert_eq!(completed.name, "Recurring Task");
        assert_eq!(new_task.name, "Recurring Task");
        assert_ne!(completed.id, new_task.id);
    });
}

#[test]
fn test_template_creation_and_usage() {
    with_test_db("template_usage", |_path| {
        // Create a template
        cmd_template_add("dev".into(), Some("Coding".into()), 2.0, true);
        
        let templates = load_templates();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "dev");
        assert_eq!(templates[0].default_hours, 2.0);

        // Create task using template
        cmd_add("Task 1".into(), None, None, "2025-12-01".into(), Some("dev".into()), None, true);
        
        let tasks = load_tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].project, Some("Coding".into()));
        assert_eq!(tasks[0].expected_hours, 2.0);
        assert_eq!(tasks[0].template, Some("dev".into()));
    });
}

#[test]
fn test_template_auto_update() {
    with_test_db("template_update", |_path| {
        cmd_template_add("writing".into(), Some("Docs".into()), 1.0, true);
        
        // Add task with template
        cmd_add("Doc 1".into(), None, None, "2025-12-01".into(), Some("writing".into()), None, true);
        let tasks = load_tasks();
        let id = tasks[0].id;

        // Log more hours than expected (3.0 total)
        cmd_log(id, 3.0, true);
        
        // Complete task
        cmd_complete(id, true);

        // Check template updated
        let templates = load_templates();
        assert_eq!(templates[0].name, "writing");
        assert_eq!(templates[0].default_hours, 3.0);
    });
}

#[test]
fn test_template_remove() {
    with_test_db("template_remove", |_path| {
        cmd_template_add("temp".into(), None, 1.0, true);
        cmd_add("Task".into(), None, None, "2025-12-01".into(), Some("temp".into()), None, true);
        
        cmd_template_remove("temp".into(), true);
        
        let templates = load_templates();
        assert!(templates.is_empty());

        let tasks = load_tasks();
        assert_eq!(tasks[0].template, None);
    });
}
