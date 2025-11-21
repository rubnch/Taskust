use std::io::{self, Write};
use chrono::{Local, NaiveDate, Duration};
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use crate::models::{Task, Template};
use crate::storage::{load_tasks, save_tasks, load_templates, save_templates, delete_database};
use crate::urgency::compute_urgency;

/// Adds a new task to the database.
///
/// If a `template_name` is provided, it attempts to use defaults from that template.
/// It also checks past completed tasks of that template to estimate duration intelligently.
pub fn cmd_add(name: String, project: Option<String>, hours: Option<f64>, due: String, template_name: Option<String>, recur: Option<String>, silent: bool) {
    let due_date = match NaiveDate::parse_from_str(&due, "%Y-%m-%d") {
        Ok(d) => d,
        Err(e) => {
            if !silent { eprintln!("Invalid due date '{}': {}. Use YYYY-MM-DD.", due, e); }
            return;
        }
    };

    let mut final_project = project;
    let mut final_hours = hours.unwrap_or(1.0);

    if let Some(t_name) = &template_name {
        let mut templates = load_templates();
        let template_idx = templates.iter().position(|t| t.name == *t_name);

        if let Some(idx) = template_idx {
            let tmpl = &templates[idx];
            if final_project.is_none() {
                final_project = tmpl.project.clone();
            }
            if hours.is_none() {
                final_hours = tmpl.default_hours;
            }
        } else {
            if !silent { println!("Template '{}' not found. Creating it.", t_name); }
            let new_tmpl = Template {
                name: t_name.clone(),
                project: final_project.clone(),
                default_hours: final_hours,
            };
            templates.push(new_tmpl);
            if let Err(e) = save_templates(&templates) {
                if !silent { eprintln!("Failed to save new template: {}", e); }
            }
        }
    }

    let mut tasks = load_tasks();
    let next_id = tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
    let t = Task {
        id: next_id,
        name,
        project: final_project,
        expected_hours: final_hours.max(0.0),
        due_date,
        created_at: Local::now().to_rfc3339(),
        completed: false,
        hours_worked: 0.0,
        template: template_name,
        recurrence: recur,
    };
    tasks.push(t);
    if let Err(e) = save_tasks(&tasks) {
        if !silent { eprintln!("Failed to save tasks: {}", e); }
    } else {
        if !silent { println!("Task added (id = {})", next_id); }
    }
}

/// Marks a task as complete by ID.
///
/// If the task is recurring, a new task is created with the next due date.
pub fn cmd_complete(id: u64, silent: bool) {
    let mut tasks = load_tasks();
    let mut new_task: Option<Task> = None;
    let mut template_update_info: Option<(String, f64)> = None;

    if let Some(t) = tasks.iter_mut().find(|t| t.id == id) {
        t.completed = true;
        if !silent { println!("Task {} marked as complete.", id); }

        if let Some(recur) = &t.recurrence {
            let next_due = match recur.to_lowercase().as_str() {
                "daily" => Some(t.due_date + Duration::days(1)),
                "weekly" => Some(t.due_date + Duration::weeks(1)),
                "monthly" => Some(t.due_date + Duration::days(30)), // Approximation
                _ => {
                    if !silent { eprintln!("Unknown recurrence pattern '{}'. Supported: daily, weekly, monthly.", recur); }
                    None
                }
            };

            if let Some(due) = next_due {
                new_task = Some(Task {
                    id: 0, // Placeholder
                    name: t.name.clone(),
                    project: t.project.clone(),
                    expected_hours: t.expected_hours,
                    due_date: due,
                    created_at: Local::now().to_rfc3339(),
                    completed: false,
                    hours_worked: 0.0,
                    template: t.template.clone(),
                    recurrence: t.recurrence.clone(),
                });
                if !silent { println!("Recurring task created due on {}", due); }
            }
        }

        if let Some(template) = &t.template {
            template_update_info = Some((template.clone(), t.hours_worked));
        }
    } else {
        if !silent { eprintln!("Task {} not found.", id); }
        return;
    }

    if let Some(mut nt) = new_task {
        let next_id = tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        nt.id = next_id;
        tasks.push(nt);
    }

    if let Err(e) = save_tasks(&tasks) {
        if !silent { eprintln!("Failed to save tasks: {}", e); }
        return;
    }

    // Update template average duration
    if let Some((tmpl_name, _)) = template_update_info {
        let completed_with_template: Vec<&Task> = tasks.iter()
            .filter(|t| t.completed && t.template.as_ref() == Some(&tmpl_name))
            .collect();
        
        if !completed_with_template.is_empty() {
            let total_worked: f64 = completed_with_template.iter().map(|t| t.hours_worked).sum();
            let avg = total_worked / completed_with_template.len() as f64;
            
            let mut templates = load_templates();
            if let Some(tmpl) = templates.iter_mut().find(|t| t.name == tmpl_name) {
                if !silent { 
                    println!("Updating template '{}' average duration to {:.2}h (based on {} tasks)", 
                        tmpl_name, avg, completed_with_template.len()); 
                }
                tmpl.default_hours = avg;
                if let Err(e) = save_templates(&templates) {
                    if !silent { eprintln!("Failed to save templates: {}", e); }
                }
            }
        }
    }
}

/// Removes a task from the database by ID.
pub fn cmd_remove(id: u64, silent: bool) {
    let mut tasks = load_tasks();
    let len_before = tasks.len();
    tasks.retain(|t| t.id != id);
    if tasks.len() == len_before {
        if !silent { eprintln!("Task {} not found.", id); }
    } else {
        if let Err(e) = save_tasks(&tasks) {
            if !silent { eprintln!("Failed to save tasks: {}", e); }
        } else {
            if !silent { println!("Task {} removed.", id); }
        }
    }
}

/// Edits an existing task's details.
pub fn cmd_edit(id: u64, name: Option<String>, project: Option<String>, expected_hours: Option<f64>, hours_worked: Option<f64>, due: Option<String>, recur: Option<String>, silent: bool) {
    let mut tasks = load_tasks();
    if let Some(t) = tasks.iter_mut().find(|t| t.id == id) {
        if let Some(n) = name { t.name = n; }
        if let Some(p) = project { t.project = Some(p); }
        if let Some(h) = expected_hours { t.expected_hours = h; }
        if let Some(h) = hours_worked { t.hours_worked = h; }
        if let Some(r) = recur { t.recurrence = Some(r); }
        if let Some(d) = due {
             match NaiveDate::parse_from_str(&d, "%Y-%m-%d") {
                Ok(date) => t.due_date = date,
                Err(e) => {
                    if !silent { eprintln!("Invalid due date '{}': {}. Use YYYY-MM-DD.", d, e); }
                    return;
                }
            }
        }
        if let Err(e) = save_tasks(&tasks) {
            if !silent { eprintln!("Failed to save tasks: {}", e); }
        } else {
            if !silent { println!("Task {} updated.", id); }
        }
    } else {
        if !silent { eprintln!("Task {} not found.", id); }
    }
}

/// Logs hours worked on a specific task.
/// 
/// hours_worked += hours
pub fn cmd_log(id: u64, hours: f64, silent: bool) {
    match load_tasks().iter().find(|t| t.id == id).map(|t| t.hours_worked) {
        Some(h) => {
            cmd_edit(id, None, None, None, Some(h + hours), None, None, silent);
        },
        None => { if !silent { eprintln!("Task {} not found.", id); } },
    }
}

/// Updates the estimated remaining hours for a task.
///
/// expected_hours = hours_worked + remaining
pub fn cmd_estimate(id: u64, remaining: f64, silent: bool) {
    match load_tasks().iter().find(|t| t.id == id).map(|t| t.hours_worked) {
        Some(hours_worked) => {
            cmd_edit(id, None, None, Some(hours_worked + remaining), None, None, None, silent);
        },
        None => { if !silent { eprintln!("Task {} not found.", id); } },
    }
}

/// Lists tasks in a formatted table, sorted by urgency.
///
/// By default, hides completed tasks unless `all` is true.
pub fn cmd_list(all: bool) {
    let mut tasks = load_tasks();
    if !all {
        tasks.retain(|t| !t.completed);
    }
    if tasks.is_empty() {
        println!("No tasks found.");
        return;
    }
    
    // Sort by urgency descending
    tasks.sort_by(|a, b| compute_urgency(b).partial_cmp(&compute_urgency(a)).unwrap());

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("ID").add_attribute(Attribute::Bold),
            Cell::new("Name").add_attribute(Attribute::Bold),
            Cell::new("Project").add_attribute(Attribute::Bold),
            Cell::new("Due").add_attribute(Attribute::Bold),
            Cell::new("Time Left").add_attribute(Attribute::Bold),
            Cell::new("Worked").add_attribute(Attribute::Bold),
            Cell::new("Est").add_attribute(Attribute::Bold),
            Cell::new("Urg").add_attribute(Attribute::Bold),
            Cell::new("Status").add_attribute(Attribute::Bold),
        ]);

    let today = Local::now().date_naive();

    for t in tasks {
        let urgency = compute_urgency(&t);
        let days_left = (t.due_date - today).num_days();
        let time_left_str = if days_left < 0 {
            format!("{}d overdue", days_left.abs())
        } else if days_left == 0 {
            "Today".to_string()
        } else {
            format!("{}d", days_left)
        };

        let urgency_color = if t.completed {
            Color::Grey
        } else if urgency > 50.0 {
            Color::Red
        } else if urgency > 20.0 {
            Color::Yellow
        } else {
            Color::Green
        };

        let status = if t.completed { "Done" } else { "Pending" };
        let status_color = if t.completed { Color::Green } else { Color::Yellow };

        table.add_row(vec![
            Cell::new(t.id),
            Cell::new(&t.name),
            Cell::new(t.project.unwrap_or_default()),
            Cell::new(t.due_date),
            Cell::new(time_left_str).fg(if days_left < 0 && !t.completed { Color::Red } else { Color::Reset }),
            Cell::new(format!("{:.1}", t.hours_worked)),
            Cell::new(format!("{:.1}", t.expected_hours)),
            Cell::new(format!("{:.1}", urgency)).fg(urgency_color),
            Cell::new(status).fg(status_color),
        ]);
    }

    println!("{table}");
}

/// Adds a new task template.
pub fn cmd_template_add(name: String, project: Option<String>, hours: f64, silent: bool) {
    let mut templates = load_templates();
    if templates.iter().any(|t| t.name == name) {
        if !silent { eprintln!("Template '{}' already exists.", name); }
        return;
    }
    templates.push(Template { name: name.clone(), project, default_hours: hours });
    if let Err(e) = save_templates(&templates) {
        if !silent { eprintln!("Failed to save templates: {}", e); }
    } else {
        if !silent { println!("Template '{}' added.", name); }
    }
}

/// Lists all available templates.
pub fn cmd_template_list() {
    let templates = load_templates();
    if templates.is_empty() {
        println!("No templates found.");
        return;
    }
    let mut table = Table::new();
    table.load_preset(UTF8_FULL)
        .set_header(vec!["Name", "Default Project", "Default Hours"]);
    for t in templates {
        table.add_row(vec![
            t.name,
            t.project.unwrap_or_else(|| "-".into()),
            format!("{:.2}", t.default_hours),
        ]);
    }
    println!("{table}");
}

/// Removes a template and updates associated tasks.
pub fn cmd_template_remove(name: String, silent: bool) {
    let mut templates = load_templates();
    let len_before = templates.len();
    templates.retain(|t| t.name != name);
    
    if templates.len() == len_before {
        if !silent { eprintln!("Template '{}' not found.", name); }
        return;
    }

    if let Err(e) = save_templates(&templates) {
        if !silent { eprintln!("Failed to save templates: {}", e); }
        return;
    }

    // Update tasks that used this template
    let mut tasks = load_tasks();
    let mut updated = false;
    for t in tasks.iter_mut() {
        if t.template.as_ref() == Some(&name) {
            t.template = None;
            updated = true;
        }
    }

    if updated {
        if let Err(e) = save_tasks(&tasks) {
            if !silent { eprintln!("Failed to update tasks: {}", e); }
        }
    }

    if !silent { println!("Template '{}' removed.", name); }
}

/// Resets the database by deleting all tasks and templates.
pub fn cmd_reset(force: bool) {
    if !force {
        print!("Are you sure you want to delete all tasks and templates? This cannot be undone. [y/N] ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if input.trim().to_lowercase() != "y" {
            println!("Aborted.");
            return;
        }
    }

    if let Err(e) = delete_database() {
        eprintln!("Failed to reset database: {}", e);
    } else {
        println!("Database reset successfully.");
    }
}

pub fn cmd_template_edit(name: String, project: Option<String>, hours: Option<f64>, silent: bool) {
    let mut templates = load_templates();
    if let Some(t) = templates.iter_mut().find(|t| t.name == name) {
        if let Some(p) = project {
            t.project = Some(p);
        }
        if let Some(h) = hours {
            t.default_hours = h;
        }
        if let Err(e) = save_templates(&templates) {
            if !silent { eprintln!("Failed to save templates: {}", e); }
        } else {
            if !silent { println!("Template '{}' updated.", name); }
        }
    } else {
        if !silent { eprintln!("Template '{}' not found.", name); }
    }
}
