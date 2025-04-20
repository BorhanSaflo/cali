mod app;
mod ui;
mod parser;
mod evaluator;
mod currency;
#[cfg(test)]
mod tests;

use std::io;
use std::env;
use std::fs;
use std::path::Path;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use app::App;

fn main() -> Result<(), io::Error> {
    // Parse command line args
    let args: Vec<String> = env::args().collect();
    
    // Check for version flags
    if args.len() > 1 && (args[1] == "-v" || args[1] == "--version") {
        println!("Cali version {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Check for help flag
    if args.len() > 1 && (args[1] == "-h" || args[1] == "--help") {
        print_help();
        return Ok(());
    }
    
    // Create app state
    let mut app = App::new();
    
    // Track the current file path
    let mut current_file_path: Option<String> = None;
    
    // If a file path is provided, load it
    if args.len() > 1 {
        let file_path = &args[1];
        if !file_path.starts_with("-") {  // Ensure it's not a flag
            current_file_path = Some(file_path.clone());
            if let Err(e) = load_file_into_app(file_path, &mut app) {
                eprintln!("Error loading file '{}': {}", file_path, e);
                return Ok(());
            }
        }
    }

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
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
                    match app.input_mode {
                        app::InputMode::Normal => {
                            // Handle keys in normal mode
                            match key.code {
                                KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    break;
                                }
                                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    // Check if we already have a file path
                                    if let Some(path) = &current_file_path {
                                        // Save to the existing path
                                        match save_file_from_app(path, &app) {
                                            Ok(_) => {
                                                // Show success message in status bar
                                                app.set_status_message(format!("File saved successfully to '{}'", path));
                                            }
                                            Err(e) => {
                                                // Show error message in status bar
                                                app.set_status_message(format!("Error saving file: {}", e));
                                            }
                                        }
                                    } else {
                                        // Need to get a file path from the user
                                        // Switch to file path input mode
                                        app.set_input_mode(app::InputMode::FilePath);
                                    }
                                }
                                _ => {
                                    app.handle_key(key);
                                }
                            }
                        },
                        app::InputMode::FilePath => {
                            // Handle file path input
                            if let Some(path) = app.handle_status_input(key) {
                                if !path.is_empty() {
                                    // Save file
                                    match save_file_from_app(&path, &app) {
                                        Ok(_) => {
                                            current_file_path = Some(path.clone());
                                            app.set_status_message(format!("File saved successfully to '{}'", path));
                                        }
                                        Err(e) => {
                                            app.set_status_message(format!("Error saving file: {}", e));
                                        }
                                    }
                                } else {
                                    app.set_status_message("Save cancelled - no file path provided.".to_string());
                                }
                            }
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

// Load calculations from a file into the app
fn load_file_into_app(file_path: &str, app: &mut App) -> io::Result<()> {
    // Check if file exists
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File not found: {}", file_path)
        ));
    }
    
    // Read file contents
    let content = fs::read_to_string(path)?;
    
    // Clear existing content
    app.lines.clear();
    app.results.clear();
    app.debounced_results.clear();
    app.variables.clear();
    app.cursor_pos = (0, 0);
    
    // Split content by lines and add each line to the app
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            app.add_line(trimmed.to_string());
        }
    }
    
    // If file was empty or only had empty lines, add at least one empty line
    if app.lines.is_empty() {
        app.add_line(String::new());
    }
    
    // Evaluate all lines
    app.evaluate_expressions();
    
    // Position cursor at the end of the loaded content
    let last_line_idx = app.lines.len() - 1;
    let last_line_len = app.lines[last_line_idx].len();
    app.cursor_pos = (last_line_idx, last_line_len);
    
    Ok(())
}

// Save calculations from the app to a file
fn save_file_from_app(file_path: &str, app: &App) -> io::Result<()> {
    use std::fs::File;
    use std::io::Write;
    
    let mut file = File::create(Path::new(file_path))?;
    
    // Write each line to the file
    for line in &app.lines {
        writeln!(file, "{}", line)?;
    }
    
    Ok(())
}

// Print help information
fn print_help() {
    println!("Cali v{} - A terminal calculator with unit conversions and natural language expressions", env!("CARGO_PKG_VERSION"));
    println!();
    println!("USAGE:");
    println!("  cali                    Start interactive calculator");
    println!("  cali [FILE]             Load and execute calculations from FILE");
    println!("  cali -v, --version      Display version information");
    println!("  cali -h, --help         Display this help message");
    println!();
    println!("KEYBOARD SHORTCUTS:");
    println!("  Ctrl+Q                  Quit the application");
    println!("  Ctrl+S                  Save the current work to a file");
    println!();
    println!("EXAMPLES:");
    println!("  cali                    Start interactive calculator");
    println!("  cali calculations.txt   Load calculations from file");
    println!();
}
