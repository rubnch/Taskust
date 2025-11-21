use std::io::{self, Write};
use chrono::{Local, NaiveDate, Duration};
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use crate::models::{Task, Template};
use crate::storage::{delete_database, load_task, load_tasks, load_template, load_templates, save_tasks, save_task, save_templates};
use crate::urgency::compute_urgency;

/// Adds a new task to the database.
///
/// If a `template_name` is provided, it attempts to use defaults from that template.
/// It also checks past completed tasks of that template to estimate duration intelligently.
pub fn cmd_add(name: String, project: Option<String>, hours: Option<f64>, due: String, template_name: Option<String>, recur: Option<String>, silent: bool) {
    let due_date = match parse_date(&due) {
        Ok(d) => d,
        Err(e) => {
            if !silent { eprintln!("{}", e); }
            return;
        }
    };

    let mut final_project = project;
    let mut final_hours = hours.unwrap_or(1.0);

    if let Some(t_name) = &template_name {
        if let Some(tmpl) = load_template(t_name) {
            if final_project.is_none() {
                final_project = tmpl.project.clone();
            }
            if hours.is_none() {
                final_hours = tmpl.default_hours;
            }
        } else {
            create_template_if_missing(t_name, &final_project, final_hours, silent);
        }
    }

    modify_tasks(silent, |tasks| {
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
        Some(format!("Task added (id = {})", next_id))
    });
}

/// Marks a task as complete by ID.
///
/// If the task is recurring, a new task is created with the next due date.
pub fn cmd_complete(id: u64, silent: bool) {
    let mut template_to_update: Option<String> = None;

    modify_tasks(silent, |tasks| {
        let mut new_task: Option<Task> = None;
        
        if let Some(t) = tasks.iter_mut().find(|t| t.id == id) {
            t.completed = true;
            if !silent { println!("Task {} marked as complete.", id); }

            if let Some(recur) = &t.recurrence {
                if let Some(due) = get_next_recurrence(recur, t.due_date) {
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
                } else if !silent {
                    eprintln!("Unknown recurrence pattern '{}'. Supported: daily, weekly, monthly.", recur);
                }
            }

            if let Some(template) = &t.template {
                template_to_update = Some(template.clone());
            }
        } else {
            if !silent { eprintln!("Task {} not found.", id); }
            return None;
        }

        if let Some(mut nt) = new_task {
            let next_id = tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
            nt.id = next_id;
            tasks.push(nt);
        }
        
        // Return empty string to signal save but no extra print (we printed inside)
        Some(String::new())
    });

    // Update template average duration
    if let Some(tmpl_name) = template_to_update {
        recalculate_template_average(&tmpl_name, silent);
    }
}

/// Removes a task from the database by ID.
pub fn cmd_remove(id: u64, silent: bool) {
    modify_tasks(silent, |tasks| {
        let len_before = tasks.len();
        tasks.retain(|t| t.id != id);
        if tasks.len() == len_before {
            if !silent { eprintln!("Task {} not found.", id); }
            None
        } else {
            Some(format!("Task {} removed.", id))
        }
    });
}

/// Edits an existing task's details.
pub fn cmd_edit(
    id: u64, 
    name: Option<String>, 
    project: Option<String>, 
    template_name: Option<String>,
    expected_hours: Option<f64>, 
    hours_worked: Option<f64>, 
    due: Option<String>, 
    recur: Option<String>, 
    silent: bool
) {
    modify_task(id, silent, |task| {
        if let Some(n) = name { task.name = n; }
        if let Some(p) = project { task.project = Some(p); }
        if let Some(tmpl) = template_name { 
            task.template = Some(tmpl.clone());
            create_template_if_missing(&tmpl, &task.project, task.expected_hours, silent);
        }
        if let Some(h) = expected_hours { task.expected_hours = h; }
        if let Some(h) = hours_worked { task.hours_worked = h; }
        if let Some(r) = recur { task.recurrence = Some(r); }
        if let Some(d) = due {
             match parse_date(&d) {
                Ok(date) => task.due_date = date,
                Err(e) => {
                    if !silent { eprintln!("{}", e); }
                    return None;
                }
            }
        }
        Some(format!("Task {} updated.", id))
    });
}

/// Logs hours worked on a specific task.
/// 
/// hours_worked += hours
pub fn cmd_log(id: u64, hours: f64, silent: bool) {
    modify_task(id, silent, |task| {
        task.hours_worked += hours;
        Some(format!("Logged {:.2} hours to task {}. Total worked: {:.2} hours.", hours, id, task.hours_worked))
    });
}

/// Updates the estimated remaining hours for a task.
///
/// expected_hours = hours_worked + remaining
pub fn cmd_estimate(id: u64, remaining: f64, silent: bool) {
    modify_task(id, silent, |task| {
        let new_total = task.hours_worked + remaining;
        let worked = task.hours_worked;
        task.expected_hours = new_total;
        Some(format!("Updated task {} estimate. Total expected: {:.2}h (Worked: {:.2}h + Remaining: {:.2}h)", 
                id, new_total, worked, remaining))
    });
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
        table.add_row(create_task_row(&t, today));
    }

    println!("{table}");
}

/// Adds a new task template.
pub fn cmd_template_add(name: String, project: Option<String>, hours: f64, silent: bool) {
    modify_templates(silent, |templates| {
        if templates.iter().any(|t| t.name == name) {
            if !silent { eprintln!("Template '{}' already exists.", name); }
            return None;
        }
        templates.push(Template { name: name.clone(), project, default_hours: hours });
        Some(format!("Template '{}' added.", name))
    });
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
    let mut removed = false;
    modify_templates(silent, |templates| {
        let len_before = templates.len();
        templates.retain(|t| t.name != name);
        
        if templates.len() == len_before {
            if !silent { eprintln!("Template '{}' not found.", name); }
            None
        } else {
            removed = true;
            Some(format!("Template '{}' removed.", name))
        }
    });

    if removed {
        // Update tasks that used this template
        modify_tasks(true, |tasks| {
            let mut changed = false;
            for t in tasks.iter_mut().filter(|t| t.template.as_ref() == Some(&name)) {
                t.template = None;
                changed = true;
            }
            if changed { Some(String::new()) } else { None }
        });
    }
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

/// Edits an existing template.
pub fn cmd_template_edit(name: String, project: Option<String>, hours: Option<f64>, silent: bool) {
    modify_template(&name, silent, |t| {
        if let Some(p) = project {
            t.project = Some(p);
        }
        if let Some(h) = hours {
            t.default_hours = h;
        }
        Some(format!("Template '{}' updated.", name))
    });
}

fn modify_task<F>(id: u64, silent: bool, f: F)
where
    F: FnOnce(&mut Task) -> Option<String>,
{
    let mut t = load_task(id);
    match t {
        Some(ref mut task) => {
            if let Some(msg) = f(task) {
                if let Err(e) = save_task(task) {
                    if !silent { eprintln!("Failed to save task: {}", e); }
                } else {
                    if !silent { println!("{}", msg); }
                }
            }
        },
        None => {
            if !silent { eprintln!("Task {} not found.", id); }
        }
    }
}

fn modify_template<F>(name: &str, silent: bool, f: F)
where
    F: FnOnce(&mut Template) -> Option<String>,
{
    let mut templates = load_templates();
    if let Some(t) = templates.iter_mut().find(|t| t.name == name) {
        if let Some(msg) = f(t) {
            if let Err(e) = save_templates(&templates) {
                if !silent { eprintln!("Failed to save templates: {}", e); }
            } else {
                if !silent { println!("{}", msg); }
            }
        }
    } else {
        if !silent { eprintln!("Template '{}' not found.", name); }
    }
}

fn modify_tasks<F>(silent: bool, f: F)
where
    F: FnOnce(&mut Vec<Task>) -> Option<String>,
{
    let mut tasks = load_tasks();
    if let Some(msg) = f(&mut tasks) {
        if let Err(e) = save_tasks(&tasks) {
            if !silent { eprintln!("Failed to save tasks: {}", e); }
        } else if !msg.is_empty() {
            if !silent { println!("{}", msg); }
        }
    }
}

fn modify_templates<F>(silent: bool, f: F)
where
    F: FnOnce(&mut Vec<Template>) -> Option<String>,
{
    let mut templates = load_templates();
    if let Some(msg) = f(&mut templates) {
        if let Err(e) = save_templates(&templates) {
            if !silent { eprintln!("Failed to save templates: {}", e); }
        } else if !msg.is_empty() {
            if !silent { println!("{}", msg); }
        }
    }
}

fn get_next_recurrence(recur: &str, current: NaiveDate) -> Option<NaiveDate> {
    match recur.to_lowercase().as_str() {
        "daily" => Some(current + Duration::days(1)),
        "weekly" => Some(current + Duration::weeks(1)),
        "monthly" => Some(current + Duration::days(30)),
        _ => None,
    }
}

fn recalculate_template_average(tmpl_name: &str, silent: bool) {
    let tasks = load_tasks();
    let completed_with_template: Vec<&Task> = tasks.iter()
        .filter(|t| t.completed && t.template.as_deref() == Some(tmpl_name))
        .collect();
    
    if !completed_with_template.is_empty() {
        let total_worked: f64 = completed_with_template.iter().map(|t| t.hours_worked).sum();
        let avg = total_worked / completed_with_template.len() as f64;
        
        modify_template(tmpl_name, silent, |tmpl| {
            if !silent { 
                println!("Updating template '{}' average duration to {:.2}h (based on {} tasks)", 
                    tmpl_name, avg, completed_with_template.len()); 
            }
            tmpl.default_hours = avg;
            Some(String::new())
        });
    }
}

fn parse_date(date_str: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|e| format!("Invalid due date '{}': {}. Use YYYY-MM-DD.", date_str, e))
}

fn create_task_row(t: &Task, today: NaiveDate) -> Vec<Cell> {
    let urgency = compute_urgency(t);
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

    vec![
        Cell::new(t.id),
        Cell::new(&t.name),
        Cell::new(t.project.as_deref().unwrap_or_default()),
        Cell::new(t.due_date),
        Cell::new(time_left_str).fg(if days_left < 0 && !t.completed { Color::Red } else { Color::Reset }),
        Cell::new(format!("{:.1}", t.hours_worked)),
        Cell::new(format!("{:.1}", t.expected_hours)),
        Cell::new(format!("{:.1}", urgency)).fg(urgency_color),
        Cell::new(status).fg(status_color),
    ]
}

/// Helper function to create a template if it doesn't exist.
///
/// This is used when adding or editing a task with a template name that is not yet in the database.
fn create_template_if_missing(name: &str, project: &Option<String>, hours: f64, silent: bool) {
    if load_template(name).is_none() {
        if !silent { println!("Template '{}' not found. Creating it.", name); }
        modify_templates(silent, |templates| {
            templates.push(Template {
                name: name.to_string(),
                project: project.clone(),
                default_hours: hours,
            });
            None
        });
    }
}
