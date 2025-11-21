use ratatui::widgets::TableState;
use crate::models::{Task, Template};
use crate::storage::{load_tasks, save_tasks, load_templates};
use crate::urgency::compute_urgency;
use crate::commands::{cmd_complete, cmd_add, cmd_edit, cmd_log, cmd_estimate, cmd_template_add, cmd_template_remove};

#[derive(PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
    Adding,
}

pub enum ViewMode {
    Tasks,
    Templates,
}

pub enum InputField {
    None,
    Name,
    Project,
    Due,
    Hours,
    Recur,
    LogHours,
    EstimateHours,
}

pub struct App {
    pub tasks: Vec<Task>,
    pub templates: Vec<Template>,
    pub state: TableState,
    pub template_state: TableState,
    pub view_mode: ViewMode,
    pub input_mode: InputMode,
    pub input_field: InputField,
    pub input_buffer: String,
    pub target_id: Option<u64>,
    // For adding tasks, we need to store partial data
    pub add_state: AddState,
    pub show_completed: bool,
}

#[derive(Default)]
pub struct AddState {
    pub name: String,
    pub project: Option<String>,
    pub due: String,
    pub hours: Option<f64>,
    pub recur: Option<String>,
    pub step: usize, // 0: Name, 1: Project, 2: Due, 3: Hours, 4: Recur
    pub template: Option<String>,
}

impl App {
    pub fn new() -> App {
        let mut tasks = load_tasks();
        // Filter out completed tasks for the main view
        tasks.retain(|t| !t.completed);
        // Sort by urgency
        tasks.sort_by(|a, b| compute_urgency(b).partial_cmp(&compute_urgency(a)).unwrap());
        
        let mut state = TableState::default();
        if !tasks.is_empty() {
            state.select(Some(0));
        }

        let templates = load_templates();
        let mut template_state = TableState::default();
        if !templates.is_empty() {
            template_state.select(Some(0));
        }

        App { 
            tasks, 
            templates,
            state,
            template_state,
            view_mode: ViewMode::Tasks,
            input_mode: InputMode::Normal,
            input_field: InputField::None,
            input_buffer: String::new(),
            target_id: None,
            add_state: AddState::default(),
            show_completed: false,
        }
    }

    pub fn next(&mut self) {
        match self.view_mode {
            ViewMode::Tasks => {
                if self.tasks.is_empty() { return; }
                let i = match self.state.selected() {
                    Some(i) => {
                        if i >= self.tasks.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.state.select(Some(i));
            }
            ViewMode::Templates => {
                if self.templates.is_empty() { return; }
                let i = match self.template_state.selected() {
                    Some(i) => {
                        if i >= self.templates.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.template_state.select(Some(i));
            }
        }
    }

    pub fn previous(&mut self) {
        match self.view_mode {
            ViewMode::Tasks => {
                if self.tasks.is_empty() { return; }
                let i = match self.state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.tasks.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.state.select(Some(i));
            }
            ViewMode::Templates => {
                if self.templates.is_empty() { return; }
                let i = match self.template_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.templates.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.template_state.select(Some(i));
            }
        }
    }

    pub fn complete_selected(&mut self) {
        if let ViewMode::Templates = self.view_mode { return; }
        if let Some(i) = self.state.selected() {
            if i < self.tasks.len() {
                let id = self.tasks[i].id;
                // Use the command logic to handle recurrence
                cmd_complete(id, true);
                // Reload tasks
                self.reload();
            }
        }
    }

    pub fn delete_selected(&mut self) {
        match self.view_mode {
            ViewMode::Tasks => {
                if let Some(i) = self.state.selected() {
                    if i < self.tasks.len() {
                        let id = self.tasks[i].id;
                        // Direct deletion logic since cmd_remove prints
                        let mut all_tasks = load_tasks();
                        all_tasks.retain(|t| t.id != id);
                        let _ = save_tasks(&all_tasks);
                        self.reload();
                    }
                }
            }
            ViewMode::Templates => {
                if let Some(i) = self.template_state.selected() {
                    if i < self.templates.len() {
                        let name = self.templates[i].name.clone();
                        cmd_template_remove(name, true);
                        self.reload();
                    }
                }
            }
        }
    }

    pub fn reload(&mut self) {
        let mut tasks = load_tasks();
        if !self.show_completed {
            tasks.retain(|t| !t.completed);
        }
        tasks.sort_by(|a, b| compute_urgency(b).partial_cmp(&compute_urgency(a)).unwrap());
        self.tasks = tasks;
        if self.tasks.is_empty() {
            self.state.select(None);
        } else if let Some(i) = self.state.selected() {
            if i >= self.tasks.len() {
                self.state.select(Some(self.tasks.len() - 1));
            }
        } else {
            self.state.select(Some(0));
        }

        self.templates = load_templates();
        if self.templates.is_empty() {
            self.template_state.select(None);
        } else if let Some(i) = self.template_state.selected() {
            if i >= self.templates.len() {
                self.template_state.select(Some(self.templates.len() - 1));
            }
        } else {
            self.template_state.select(Some(0));
        }
    }

    pub fn toggle_completed(&mut self) {
        self.show_completed = !self.show_completed;
        self.reload();
    }

    pub fn toggle_view(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Tasks => ViewMode::Templates,
            ViewMode::Templates => ViewMode::Tasks,
        };
    }

    pub fn start_add(&mut self) {
        self.input_mode = InputMode::Adding;
        self.add_state = AddState::default();
        self.add_state.step = 0;
        self.input_buffer.clear();
    }

    pub fn start_add_from_template(&mut self) {
        if let ViewMode::Templates = self.view_mode {
            if let Some(i) = self.template_state.selected() {
                if i < self.templates.len() {
                    let tmpl_name = self.templates[i].name.clone();
                    self.input_mode = InputMode::Adding;
                    self.add_state = AddState::default();
                    self.add_state.template = Some(tmpl_name);
                    self.add_state.step = 0;
                    self.input_buffer.clear();
                }
            }
        }
    }

    pub fn start_edit(&mut self, field: InputField) {
        if let ViewMode::Templates = self.view_mode { return; }
        if let Some(i) = self.state.selected() {
            if i < self.tasks.len() {
                self.target_id = Some(self.tasks[i].id);
                self.input_mode = InputMode::Editing;
                self.input_field = field;
                self.input_buffer.clear();
                
                // Pre-fill buffer for editing
                let t = &self.tasks[i];
                match self.input_field {
                    InputField::Name => self.input_buffer = t.name.clone(),
                    InputField::Project => self.input_buffer = t.project.clone().unwrap_or_default(),
                    InputField::Due => self.input_buffer = t.due_date.to_string(),
                    InputField::Hours => self.input_buffer = t.expected_hours.to_string(),
                    InputField::Recur => self.input_buffer = t.recurrence.clone().unwrap_or_default(),
                    InputField::LogHours => self.input_buffer = String::new(),
                    InputField::EstimateHours => self.input_buffer = String::new(),
                    _ => {}
                }
            }
        }
    }

    pub fn handle_input(&mut self) {
        match self.input_mode {
            InputMode::Adding => {
                if let Some(tmpl_name) = &self.add_state.template {
                    // Adding task from template
                    match self.add_state.step {
                        0 => { // Name
                            if !self.input_buffer.is_empty() {
                                self.add_state.name = self.input_buffer.clone();
                                self.add_state.step += 1;
                                self.input_buffer.clear();
                            }
                        }
                        1 => { // Due
                            if !self.input_buffer.is_empty() {
                                self.add_state.due = self.input_buffer.clone();
                                self.add_state.step += 1;
                                self.input_buffer.clear();
                            }
                        }
                        2 => { // Recur
                            if !self.input_buffer.is_empty() {
                                self.add_state.recur = Some(self.input_buffer.clone());
                            }
                            // Finish Add
                            cmd_add(
                                self.add_state.name.clone(),
                                None, // Project from template
                                None, // Hours from template
                                self.add_state.due.clone(),
                                Some(tmpl_name.clone()),
                                self.add_state.recur.clone(),
                                true
                            );
                            self.input_mode = InputMode::Normal;
                            self.view_mode = ViewMode::Tasks; // Switch back to tasks view
                            self.reload();
                        }
                        _ => {}
                    }
                } else {
                    match self.view_mode {
                        ViewMode::Tasks => {
                            match self.add_state.step {
                                0 => { // Name
                                    if !self.input_buffer.is_empty() {
                                        self.add_state.name = self.input_buffer.clone();
                                        self.add_state.step += 1;
                                        self.input_buffer.clear();
                                    }
                                }
                                1 => { // Project
                                    if !self.input_buffer.is_empty() {
                                        self.add_state.project = Some(self.input_buffer.clone());
                                    }
                                    self.add_state.step += 1;
                                    self.input_buffer.clear();
                                }
                                2 => { // Due
                                    if !self.input_buffer.is_empty() {
                                        self.add_state.due = self.input_buffer.clone();
                                        self.add_state.step += 1;
                                        self.input_buffer.clear();
                                    }
                                }
                                3 => { // Hours
                                    if let Ok(h) = self.input_buffer.parse::<f64>() {
                                        self.add_state.hours = Some(h);
                                        self.add_state.step += 1;
                                        self.input_buffer.clear();
                                    } else if self.input_buffer.is_empty() {
                                         self.add_state.hours = Some(1.0); // Default
                                         self.add_state.step += 1;
                                         self.input_buffer.clear();
                                    }
                                }
                                4 => { // Recur
                                    if !self.input_buffer.is_empty() {
                                        self.add_state.recur = Some(self.input_buffer.clone());
                                    }
                                    // Finish Add
                                    cmd_add(
                                        self.add_state.name.clone(),
                                        self.add_state.project.clone(),
                                        self.add_state.hours,
                                        self.add_state.due.clone(),
                                        None,
                                        self.add_state.recur.clone(),
                                        true
                                    );
                                    self.input_mode = InputMode::Normal;
                                    self.reload();
                                }
                                _ => {}
                            }
                        }
                        ViewMode::Templates => {
                            match self.add_state.step {
                                0 => { // Name
                                    if !self.input_buffer.is_empty() {
                                        self.add_state.name = self.input_buffer.clone();
                                        self.add_state.step += 1;
                                        self.input_buffer.clear();
                                    }
                                }
                                1 => { // Project
                                    if !self.input_buffer.is_empty() {
                                        self.add_state.project = Some(self.input_buffer.clone());
                                    }
                                    self.add_state.step += 1;
                                    self.input_buffer.clear();
                                }
                                2 => { // Hours
                                    let hours = if let Ok(h) = self.input_buffer.parse::<f64>() {
                                        h
                                    } else {
                                        1.0
                                    };
                                    
                                    cmd_template_add(
                                        self.add_state.name.clone(),
                                        self.add_state.project.clone(),
                                        hours,
                                        true
                                    );
                                    self.input_mode = InputMode::Normal;
                                    self.reload();
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            InputMode::Editing => {
                if let Some(id) = self.target_id {
                    match self.input_field {
                        InputField::Name => cmd_edit(id, Some(self.input_buffer.clone()), None, None, None, None, None, true),
                        InputField::Project => cmd_edit(id, None, Some(self.input_buffer.clone()), None, None, None, None, true),
                        InputField::Due => cmd_edit(id, None, None, None, None, Some(self.input_buffer.clone()), None, true),
                        InputField::Hours => {
                            if let Ok(h) = self.input_buffer.parse::<f64>() {
                                cmd_edit(id, None, None, Some(h), None, None, None, true);
                            }
                        },
                        InputField::Recur => cmd_edit(id, None, None, None, None, None, Some(self.input_buffer.clone()), true),
                        InputField::LogHours => {
                            if let Ok(h) = self.input_buffer.parse::<f64>() {
                                cmd_log(id, h, true);
                            }
                        },
                        InputField::EstimateHours => {
                            if let Ok(h) = self.input_buffer.parse::<f64>() {
                                cmd_estimate(id, h, true);
                            }
                        },
                        _ => {}
                    }
                    self.input_mode = InputMode::Normal;
                    self.reload();
                }
            }
            _ => {}
        }
    }
}
