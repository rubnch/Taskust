use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use crate::models::{Task, Template};

/// Returns the path to the tasks database file (`tasks.json`).
///
/// The path is determined in the following order:
/// 1. `TASKS_DB` environment variable.
/// 2. `~/.local/share/taskust/tasks.json` (on Linux).
/// 3. `./tasks.json` (fallback).
fn db_path() -> PathBuf {
    std::env::var("TASKS_DB").map(PathBuf::from).unwrap_or_else(|_| {
        let mut p = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        p.push("taskust");
        if !p.exists() {
            let _ = fs::create_dir_all(&p);
        }
        p.push("tasks.json");
        p
    })
}

/// Returns the path to the templates database file (`templates.json`).
///
/// Located in the same directory as the tasks database.
fn templates_path() -> PathBuf {
    let mut p = db_path();
    p.pop();
    p.push("templates.json");
    p
}

/// Returns the path to the archive database file (`archive.json`).
///
/// Located in the same directory as the tasks database.
fn archive_path() -> PathBuf {
    let mut p = db_path();
    p.pop();
    p.push("archive.json");
    p
}

/// Loads a single task by its ID.
/// 
/// Returns `None` if the task is not found.
pub fn load_task(id: u64) -> Option<Task> {
    let tasks = load_tasks();
    if let Some(t) = tasks.iter().find(|t| t.id == id) {
        return Some(t.clone());
    }
    None
}

/// Loads all tasks from the storage file.
///
/// Returns an empty vector if the file does not exist or cannot be read.
pub fn load_tasks() -> Vec<Task> {
    let path = db_path();
    if !path.exists() {
        return Vec::new();
    }
    let mut f = match OpenOptions::new().read(true).open(&path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let mut s = String::new();
    if f.read_to_string(&mut s).is_err() {
        return Vec::new();
    }
    serde_json::from_str(&s).unwrap_or_else(|_| Vec::new())
}

/// Saves or updates a single task in the storage file.
/// 
/// If the task with the same ID exists, it is updated; otherwise, it is added.
pub fn save_task(task: &Task) -> std::io::Result<()> {
    let mut tasks = load_tasks();
    if let Some(t) = tasks.iter_mut().find(|t| t.id == task.id) {
        *t = task.clone();
    }
    else {
        tasks.push(task.clone());
    }
    save_tasks(&tasks)
}

/// Saves the given list of tasks to the storage file.
///
/// Overwrites the existing file.
pub fn save_tasks(tasks: &Vec<Task>) -> std::io::Result<()> {
    let path = db_path();
    let s = serde_json::to_string_pretty(tasks).unwrap();
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)?;
    f.write_all(s.as_bytes())?;
    Ok(())
}

/// Loads all templates from the storage file.
pub fn load_templates() -> Vec<Template> {
    let path = templates_path();
    if !path.exists() {
        return Vec::new();
    }
    let mut f = match OpenOptions::new().read(true).open(&path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let mut s = String::new();
    if f.read_to_string(&mut s).is_err() {
        return Vec::new();
    }
    serde_json::from_str(&s).unwrap_or_else(|_| Vec::new())
}

/// Saves the given list of templates to the storage file.
pub fn save_templates(templates: &Vec<Template>) -> std::io::Result<()> {
    let path = templates_path();
    let s = serde_json::to_string_pretty(templates).unwrap();
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)?;
    f.write_all(s.as_bytes())?;
    Ok(())
}

/// Loads a single template by its name.
pub fn load_template(name: &str) -> Option<Template> {
    let templates = load_templates();
    if let Some(t) = templates.iter().find(|t| t.name == name) {
        return Some(t.clone());
    }
    None
}

/// Deletes the tasks and templates database files.
pub fn delete_database() -> std::io::Result<()> {
    let t_path = db_path();
    if t_path.exists() {
        fs::remove_file(t_path)?;
    }
    let tmpl_path = templates_path();
    if tmpl_path.exists() {
        fs::remove_file(tmpl_path)?;
    }
    Ok(())
}

/// Loads all archived tasks from the storage file.
pub fn load_archived_tasks() -> Vec<Task> {
    let path = archive_path();
    if !path.exists() {
        return Vec::new();
    }
    let mut f = match OpenOptions::new().read(true).open(&path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let mut s = String::new();
    if f.read_to_string(&mut s).is_err() {
        return Vec::new();
    }
    serde_json::from_str(&s).unwrap_or_else(|_| Vec::new())
}

/// Appends tasks to the archive file.
pub fn append_to_archive(new_tasks: Vec<Task>) -> std::io::Result<()> {
    let mut archive = load_archived_tasks();
    archive.extend(new_tasks);
    
    let path = archive_path();
    let s = serde_json::to_string_pretty(&archive).unwrap();
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)?;
    f.write_all(s.as_bytes())?;
    Ok(())
}
