mod app;
mod ui;
mod parser;
mod evaluator;
mod currency;
#[cfg(test)]
mod tests;

use std::io;
use std::env;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use app::App;

fn main() -> Result<(), io::Error> {
    // Check for version flags
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && (args[1] == "-v" || args[1] == "--version") {
        println!("Cali version {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();
    
    // Tick rate for UI updates (for debouncing errors)
    let tick_rate = std::time::Duration::from_millis(100);
    
    // Main loop
    loop {
        // Draw UI
        terminal.draw(|f| ui::draw(f, &mut app))?;

        // Handle input with timeout to allow periodic ticks
        if crossterm::event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                            break;
                        }
                        _ => {
                            app.handle_key(key);
                        }
                    }
                }
            }
        } else {
            // No input received, this is a tick event
            app.update_on_tick();
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
