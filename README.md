# Taskust

A powerful, terminal-based task manager written in Rust. Taskust combines a fast CLI for quick entry with a rich TUI (Terminal User Interface) for interactive management.

## Features

*   **Urgency-based Sorting**: Tasks are automatically prioritized based on due date and estimated effort.
*   **Dual Interface**:
    *   **CLI**: Scriptable and quick for single commands.
    *   **TUI**: Interactive dashboard to manage tasks visually.
*   **Templates**: Create reusable task templates for common workflows.
*   **Recurrence**: Support for daily, weekly, and monthly recurring tasks.
*   **Data Persistence**: Tasks are stored in standard XDG data directories (JSON format).

## Installation

```bash
cargo install --path .
```

## Usage

### Interactive Mode (TUI)

Simply run the command without arguments to launch the interactive UI:

```bash
taskust
# or explicitly
taskust ui
```

#### TUI Key Bindings

**Global**
*   `q`: Quit

**Task View**
*   `a`: Add new task
*   `Space`: Mark selected task as Done
*   `c`: Toggle Show/Hide Completed Tasks
*   `d`: Delete selected task
*   `l`: Log hours worked
*   `u`: Update remaining estimate
*   `n`: Edit name
*   `p`: Edit project
*   `t`: Edit due date
*   `h`: Edit expected hours
*   `r`: Edit recurrence
*   `v`: Switch to Templates view

**Template View**
*   `a`: Add new template
*   `v`: Switch to Tasks view
*   `Enter`: Create a new task from the selected template

### Command Line Interface (CLI)

You can also use `taskust` purely from the command line.

**Adding Tasks**
```bash
# Basic task
taskust add "Write report" --project Work --hours 2.5 --due 2025-12-01

# Recurring task
taskust add "Team Standup" --recur daily --hours 0.5

# From a template
taskust add --template "Bug Report"
```

**Managing Tasks**
```bash
# List tasks (sorted by urgency)
taskust list

# List all (including completed)
taskust list --all

# Complete a task
taskust complete <ID>

# Log hours
taskust log <ID> --hours 1.5
```

**Templates**
```bash
# Add a template
taskust template add "Bug Report" --project Dev --hours 1.0

# List templates
taskust template list
```

## Data Storage

Tasks are saved in your local data directory:
*   Linux: `~/.local/share/taskust/tasks.json`
*   macOS: `~/Library/Application Support/taskust/tasks.json`
*   Windows: `%APPDATA%\taskust\tasks.json`

You can override this by setting the `TASKS_DB` environment variable.

## Urgency Calculation

Tasks are scored based on:
1.  **Due Date**: Closer deadlines = higher urgency. Overdue tasks are critical.
2.  **Estimated Effort**: Larger tasks due soon are prioritized over smaller ones.

