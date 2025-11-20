use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
    Frame,
};
use chrono::Local;
use crate::urgency::compute_urgency;
use super::app::{App, InputMode, ViewMode, InputField};

pub fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Table
            Constraint::Length(3)  // Help
        ].as_ref())
        .split(f.area());

    match app.view_mode {
        ViewMode::Tasks => {
            let today = Local::now().date_naive();

            let rows: Vec<Row> = app
                .tasks
                .iter()
                .map(|t| {
                    let urgency = compute_urgency(t);
                    let days_left = (t.due_date - today).num_days();
                    let time_left_str = if days_left < 0 {
                        format!("{}d overdue", days_left.abs())
                    } else if days_left == 0 {
                        "Today".to_string()
                    } else {
                        format!("{}d", days_left)
                    };

                    let style = if urgency > 50.0 {
                        Style::default().fg(Color::Red)
                    } else if urgency > 20.0 {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Green)
                    };
                    
                    Row::new(vec![
                        Cell::from(t.id.to_string()),
                        Cell::from(t.name.clone()),
                        Cell::from(t.project.clone().unwrap_or_default()),
                        Cell::from(t.template.clone().unwrap_or_default()),
                        Cell::from(t.due_date.to_string()),
                        Cell::from(time_left_str),
                        Cell::from(format!("{:.1}", t.hours_worked)),
                        Cell::from(format!("{:.1}", t.expected_hours)),
                        Cell::from(format!("{:.1}", urgency)),
                        Cell::from(if t.completed { "Done" } else { "Pending" }),
                    ]).style(style)
                })
                .collect();

            let widths = [
                Constraint::Length(4),
                Constraint::Min(20),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(6),
                Constraint::Length(6),
                Constraint::Length(6),
                Constraint::Length(8),
            ];

            let table = Table::new(rows, widths)
                .header(Row::new(vec!["ID", "Name", "Project", "Template", "Due", "Time Left", "Worked", "Est", "Urg", "Status"])
                    .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                    .bottom_margin(1))
                .block(Block::default().borders(Borders::ALL).title("Taskust - Tasks"))
                .row_highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::DarkGray))
                .highlight_symbol(">> ");

            f.render_stateful_widget(table, chunks[0], &mut app.state);
        }
        ViewMode::Templates => {
            let rows: Vec<Row> = app
                .templates
                .iter()
                .map(|t| {
                    Row::new(vec![
                        Cell::from(t.name.clone()),
                        Cell::from(t.project.clone().unwrap_or_default()),
                        Cell::from(format!("{:.1}", t.default_hours)),
                    ])
                })
                .collect();

            let widths = [
                Constraint::Min(20),
                Constraint::Length(20),
                Constraint::Length(10),
            ];

            let table = Table::new(rows, widths)
                .header(Row::new(vec!["Name", "Project", "Est Hours"])
                    .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                    .bottom_margin(1))
                .block(Block::default().borders(Borders::ALL).title("Taskust - Templates"))
                .row_highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::DarkGray))
                .highlight_symbol(">> ");

            f.render_stateful_widget(table, chunks[0], &mut app.template_state);
        }
    }

    let help_text = match app.input_mode {
        InputMode::Normal => match app.view_mode {
            ViewMode::Tasks => "q: Quit | a: Add | n: Name | p: Proj | t: Due | h: Hrs | r: Recur | l: Log | u: Est | c: Toggle Done | Space: Done | d: Del | v: View Templates",
            ViewMode::Templates => "q: Quit | a: Add | v: View Tasks | Enter: Create Task from Template | d: Del",
        },
        InputMode::Editing => "Enter: Save | Esc: Cancel",
        InputMode::Adding => "Enter: Next Step | Esc: Cancel",
    };
    
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL));
    
    f.render_widget(help, chunks[1]);

    // Render Input Box if needed
    match app.input_mode {
        InputMode::Editing | InputMode::Adding => {
            let area = centered_rect(60, 3, f.area()); // Fixed height of 3 (border + 1 line)
            f.render_widget(Clear, area); // Clear the area first
            
            let title_string;
            let title = match app.input_mode {
                InputMode::Adding => {
                    if let Some(tmpl) = &app.add_state.template {
                        title_string = match app.add_state.step {
                            0 => format!("Add Task from '{}': Enter Name", tmpl),
                            1 => format!("Add Task from '{}': Enter Due Date (YYYY-MM-DD)", tmpl),
                            2 => format!("Add Task from '{}': Enter Recurrence (Optional)", tmpl),
                            _ => "Add Task".to_string(),
                        };
                        title_string.as_str()
                    } else {
                        match app.view_mode {
                            ViewMode::Tasks => {
                                match app.add_state.step {
                                    0 => "Add Task: Enter Name",
                                    1 => "Add Task: Enter Project (Optional)",
                                    2 => "Add Task: Enter Due Date (YYYY-MM-DD)",
                                    3 => "Add Task: Enter Expected Hours",
                                    4 => "Add Task: Enter Recurrence (Optional)",
                                    _ => "Add Task",
                                }
                            }
                            ViewMode::Templates => {
                                match app.add_state.step {
                                    0 => "Add Template: Enter Name",
                                    1 => "Add Template: Enter Project (Optional)",
                                    2 => "Add Template: Enter Expected Hours",
                                    _ => "Add Template",
                                }
                            }
                        }
                    }
                },
                InputMode::Editing => {
                    match app.input_field {
                        InputField::Name => "Edit Name",
                        InputField::Project => "Edit Project",
                        InputField::Due => "Edit Due Date (YYYY-MM-DD)",
                        InputField::Hours => "Edit Expected Hours",
                        InputField::Recur => "Edit Recurrence",
                        InputField::LogHours => "Log Hours Worked",
                        InputField::EstimateHours => "Update Estimate (Remaining)",
                        _ => "Edit",
                    }
                },
                _ => "",
            };

            let input = Paragraph::new(app.input_buffer.as_str())
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL).title(title));
            
            f.render_widget(input, area);
        }
        _ => {}
    }
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height - height) / 2),
            Constraint::Length(height),
            Constraint::Length((r.height - height) / 2),
        ].as_ref())
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ].as_ref())
        .split(popup_layout[1])[1]
}
