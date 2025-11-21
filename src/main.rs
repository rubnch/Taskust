//! # Taskust
//! 
//! A powerful, terminal-based task manager written in Rust. Taskust combines a fast CLI for quick entry with a rich TUI (Terminal User Interface) for interactive management.
//! 
//! ## Features
//! 
//! *   **Urgency-based Sorting**: Tasks are automatically prioritized based on due date and estimated effort.
//! *   **Dual Interface**:
//!     *   **CLI**: Scriptable and quick for single commands.
//!     *   **TUI**: Interactive dashboard to manage tasks visually.
//! *   **Templates**: Create reusable task templates for common workflows.
//! *   **Recurrence**: Support for daily, weekly, and monthly recurring tasks.
//! *   **Data Persistence**: Tasks are stored in standard XDG data directories (JSON format).
//! 
//! ## Installation
//! 
//! ```bash
//! cargo install --path .
//! ```
//! 
//! ## Usage
//! 
//! ### Interactive Mode (TUI)
//! 
//! Simply run the command without arguments to launch the interactive UI:
//! 
//! ```bash
//! taskust
//! # or explicitly
//! taskust ui
//! ```
//! 
//! #### TUI Key Bindings
//! 
//! **Global**
//! *   `q`: Quit
//! 
//! **Task View**
//! *   `a`: Add new task
//! *   `Space`: Mark selected task as Done
//! *   `c`: Toggle Show/Hide Completed Tasks
//! *   `d`: Delete selected task
//! *   `l`: Log hours worked
//! *   `u`: Update remaining estimate
//! *   `n`: Edit name
//! *   `p`: Edit project
//! *   `t`: Edit due date
//! *   `h`: Edit expected hours
//! *   `r`: Edit recurrence
//! *   `v`: Switch to Templates view
//! 
//! **Template View**
//! *   `a`: Add new template
//! *   `v`: Switch to Tasks view
//! *   `Enter`: Create a new task from the selected template
//! 
//! ### Command Line Interface (CLI)
//! 
//! You can also use `taskust` purely from the command line.
//! 
//! **Adding Tasks**
//! ```bash
//! # Basic task
//! taskust add "Write report" --project Work --hours 2.5 --due 2025-12-01
//! 
//! # Recurring task
//! taskust add "Team Standup" --recur daily --hours 0.5
//! 
//! # From a template
//! taskust add --template "Bug Report"
//! ```
//! 
//! **Managing Tasks**
//! ```bash
//! # List tasks (sorted by urgency)
//! taskust list
//! 
//! # List all (including completed)
//! taskust list --all
//! 
//! # Complete a task
//! taskust complete <ID>
//! 
//! # Log hours
//! taskust log <ID> --hours 1.5
//! ```
//! 
//! **Templates**
//! ```bash
//! # Add a template
//! taskust template add "Bug Report" --project Dev --hours 1.0
//! 
//! # List templates
//! taskust template list
//! ```
//! 
//! ## Data Storage
//! 
//! Tasks are saved in your local data directory:
//! *   Linux: `~/.local/share/taskust/tasks.json`
//! *   macOS: `~/Library/Application Support/taskust/tasks.json`
//! *   Windows: `%APPDATA%\taskust\tasks.json`
//! 
//! You can override this by setting the `TASKS_DB` environment variable.
//! 
//! ## Urgency Calculation
//! 
//! Tasks are scored based on:
//! 1.  **Due Date**: Closer deadlines = higher urgency. Overdue tasks are critical.
//! 2.  **Estimated Effort**: Larger tasks due soon are prioritized over smaller ones.


mod models;
mod storage;
mod urgency;
mod commands;
mod tui;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use std::io;
use commands::*;
use tui::run_tui;

#[derive(Parser)]
#[command(name = "taskust")]
#[command(about = "Simple terminal task manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new task
    Add {
        /// Task name (quoted if it has spaces)
        name: String,
        /// Project or category
        #[arg(short, long)]
        project: Option<String>,
        /// Expected duration in hours (float), e.g. 1.5
        #[arg(short = 'H', long)]
        hours: Option<f64>,
        /// Due date in YYYY-MM-DD
        #[arg(short, long)]
        due: String,
        /// Use a template
        #[arg(short, long)]
        template: Option<String>,
        /// Recurrence (daily, weekly, monthly)
        #[arg(short, long)]
        recur: Option<String>,
    },
    /// List tasks sorted by urgency
    List {
        /// Show completed tasks
        #[arg(short, long)]
        all: bool,
    },
    /// Mark a task as complete
    Complete {
        id: u64,
    },
    /// Remove a task
    Remove {
        id: u64,
    },
    /// Edit a task
    Edit {
        id: u64,
        /// New task name
        #[arg(short, long)]
        name: Option<String>,
        /// New project
        #[arg(short, long)]
        project: Option<String>,
        /// New expected duration
        #[arg(short = 'H', long)]
        hours: Option<f64>,
        /// New due date
        #[arg(short, long)]
        due: Option<String>,
        /// New recurrence
        #[arg(short, long)]
        recur: Option<String>,
        /// New template
        #[arg(short, long)]
        template: Option<String>,
    },
    /// Log hours worked on a task
    Log {
        id: u64,
        /// Hours to add
        hours: f64,
    },
    /// Re-estimate remaining hours for a task
    Estimate {
        id: u64,
        /// Remaining hours needed
        remaining: f64,
    },
    /// Manage templates
    Template {
        #[command(subcommand)]
        command: TemplateCommands,
    },
    /// Reset the database (delete all tasks and templates)
    Reset {
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for (bash, zsh, fish, powershell, elvish)
        shell: String,
    },
    /// Open interactive TUI
    Ui,
}

#[derive(Subcommand)]
enum TemplateCommands {
    /// Add a new template
    Add {
        /// Template name
        name: String,
        /// Default project
        #[arg(short, long)]
        project: Option<String>,
        /// Default duration
        #[arg(short = 'H', long, default_value_t = 1.0)]
        hours: f64,
    },
    /// List templates
    List,
    /// Remove a template
    Remove {
        /// Template name
        name: String,
    },
    /// Edit a template
    Edit {
        /// Template name
        name: String,
        /// New project
        #[arg(short, long)]
        project: Option<String>,
        /// New default duration
        #[arg(short = 'H', long)]
        hours: Option<f64>,
    }
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Add { name, project, hours, due, template, recur }) => cmd_add(name, project, hours, due, template, recur, false),
        Some(Commands::List { all }) => cmd_list(all),
        Some(Commands::Complete { id }) => cmd_complete(id, false),
        Some(Commands::Remove { id }) => cmd_remove(id, false),
        Some(Commands::Edit { id, name, project, hours, due, recur, template }) => cmd_edit(id, name, project, template, hours, None, due, recur, false),
        Some(Commands::Log { id, hours }) => cmd_log(id, hours, false),
        Some(Commands::Estimate { id, remaining }) => cmd_estimate(id, remaining, false),
        Some(Commands::Template { command }) => match command {
            TemplateCommands::Add { name, project, hours } => cmd_template_add(name, project, hours, false),
            TemplateCommands::List => cmd_template_list(),
            TemplateCommands::Remove { name } => cmd_template_remove(name, false),
            TemplateCommands::Edit { name, project, hours } => cmd_template_edit(name, project, hours, false),
        },
        Some(Commands::Reset { force }) => cmd_reset(force),
        Some(Commands::Completions { shell }) => {
            let shell_enum = match shell.as_str() {
                "bash" => Shell::Bash,
                "zsh" => Shell::Zsh,
                "fish" => Shell::Fish,
                "powershell" => Shell::PowerShell,
                "elvish" => Shell::Elvish,
                _ => {
                    eprintln!("Unsupported shell: {}", shell);
                    return;
                }
            };
            let mut cmd = Cli::command();
            generate(shell_enum, &mut cmd, "taskust", &mut io::stdout());
        }
        Some(Commands::Ui) | None => {
            if let Err(e) = run_tui() {
                eprintln!("Error running TUI: {}", e);
            }
        }
    }
}
