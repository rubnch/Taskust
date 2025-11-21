pub mod app;
pub mod ui;

use std::{error::Error, io};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use app::{App, InputMode, InputField};
use ui::ui;

pub fn run_tui() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();

    // Run loop
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            match app.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down | KeyCode::Char('j') => app.next(),
                    KeyCode::Up | KeyCode::Char('k') => app.previous(),
                    KeyCode::Char(' ') => app.complete_selected(),
                    KeyCode::Char('d') | KeyCode::Delete => app.delete_selected(),
                    KeyCode::Char('a') => app.start_add(),
                    KeyCode::Char('n') => app.start_edit(InputField::Name),
                    KeyCode::Char('p') => app.start_edit(InputField::Project),
                    KeyCode::Char('t') => app.start_edit(InputField::Due), // 't' for Time/Date
                    KeyCode::Char('h') => app.start_edit(InputField::Hours),
                    KeyCode::Char('r') => app.start_edit(InputField::Recur),
                    KeyCode::Char('m') => app.start_edit(InputField::Template),
                    KeyCode::Char('l') => app.start_edit(InputField::LogHours),
                    KeyCode::Char('u') => app.start_edit(InputField::EstimateHours), // 'u' for Update
                    KeyCode::Char('c') => app.toggle_completed(),
                    KeyCode::Char('v') => app.toggle_view(),
                    KeyCode::Enter => app.start_add_from_template(),
                    _ => {}
                },
                InputMode::Editing | InputMode::Adding => match key.code {
                    KeyCode::Enter => app.handle_input(),
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                        app.input_buffer.clear();
                    }
                    KeyCode::Char(c) => {
                        app.input_buffer.push(c);
                    }
                    KeyCode::Backspace => {
                        app.input_buffer.pop();
                    }
                    _ => {}
                }
            }
        }
    }
}
