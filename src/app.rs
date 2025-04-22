use std::collections::{HashMap, HashSet};
use std::time::{Instant, Duration};
use crossterm::event::{KeyEvent, KeyCode};
use crate::evaluator::Value;

pub struct App {
    pub lines: Vec<String>,
    pub cursor_pos: (usize, usize), // (line, column)
    pub variables: HashMap<String, Value>,
    pub results: Vec<String>,          // Real-time results (without errors if within debounce period)
    pub debounced_results: Vec<String>, // Complete results (with errors) after debounce period
    pub last_keystroke: Instant,       // Time of last keystroke
    pub debounce_period: Duration,     // Debounce period for showing errors
    pub status_message: Option<String>, // Status message to display in the status bar
    pub input_mode: InputMode,         // Current input mode
    pub status_input: String,          // Input text for status bar when in input mode
    pub panel_focus: PanelFocus,       // Which panel is currently focused
    pub output_selected_idx: usize,    // Selected index in output panel when output is focused
    status_time: Option<Instant>,      // When the status message was set
    modified_lines: HashSet<usize>,    // Track which lines were modified since last evaluation
    cached_variables: HashMap<String, Value>, // Cache variables from previous evaluations
    pub input_panel_area: Option<(u16, u16, u16, u16)>,  // (x, y, width, height) of input panel
    pub output_panel_area: Option<(u16, u16, u16, u16)>, // (x, y, width, height) of output panel
    pub input_scroll: usize,           // Scroll position for input panel
    pub output_scroll: usize,          // Scroll position for output panel
}

// Input mode for the application
#[derive(PartialEq, Clone, Copy)]
pub enum InputMode {
    Normal,    // Regular calculator mode
    FilePath,  // Entering a file path in the status bar
}

// Track which panel has focus
#[derive(PartialEq, Clone, Copy)]
pub enum PanelFocus {
    Input,
    Output,
}

impl App {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_pos: (0, 0),
            variables: HashMap::new(),
            results: vec![String::new()],
            debounced_results: vec![String::new()],
            last_keystroke: Instant::now(),
            debounce_period: Duration::from_millis(500),
            status_message: None,
            input_mode: InputMode::Normal,
            status_input: String::new(),
            panel_focus: PanelFocus::Input,
            output_selected_idx: 0,
            status_time: None,
            modified_lines: HashSet::new(),
            cached_variables: HashMap::new(),
            input_panel_area: None,
            output_panel_area: None,
            input_scroll: 0,
            output_scroll: 0,
        }
    }

    // Set the input mode
    pub fn set_input_mode(&mut self, mode: InputMode) {
        self.input_mode = mode;
        if mode == InputMode::FilePath {
            self.status_input = String::new();
        }
    }
    
    // Process key input for status bar when in input mode
    pub fn handle_status_input(&mut self, key: KeyEvent) -> Option<String> {
        match key.code {
            KeyCode::Enter => {
                // User has confirmed the input
                let result = self.status_input.clone();
                self.status_input.clear();
                self.input_mode = InputMode::Normal;
                Some(result)
            }
            KeyCode::Esc => {
                // User has cancelled the input
                self.status_input.clear();
                self.input_mode = InputMode::Normal;
                None
            }
            KeyCode::Backspace => {
                // Delete the character before the cursor
                self.status_input.pop();
                None
            }
            KeyCode::Char(c) => {
                // Add the character to the input
                self.status_input.push(c);
                None
            }
            _ => None,
        }
    }
    
    // Set a status message that will be displayed in the status bar
    pub fn set_status_message(&mut self, message: String) {
        self.status_message = Some(message);
        self.status_time = Some(Instant::now());
    }
    
    // Clear the status message
    pub fn clear_status_message(&mut self) {
        self.status_message = None;
        self.status_time = None;
    }

    // Add a new line of text to the app
    pub fn add_line(&mut self, line: String) {
        let line_index = self.lines.len();
        self.lines.push(line);
        self.results.push(String::new());
        self.debounced_results.push(String::new());
        self.modified_lines.insert(line_index);
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        // Update last keystroke time
        self.last_keystroke = Instant::now();
        
        // Track which line is being modified
        let current_line = self.cursor_pos.0;
        self.modified_lines.insert(current_line);
        
        match key.code {
            KeyCode::Enter => {
                self.insert_newline();
                // New line affects both the current and next line
                self.modified_lines.insert(self.cursor_pos.0);
            }
            KeyCode::Backspace => {
                if self.cursor_at_start_of_line() && self.cursor_pos.0 > 0 {
                    // Join with previous line
                    let prev_line = self.cursor_pos.0 - 1;
                    self.join_with_previous_line();
                    // This affects the previous line
                    self.modified_lines.insert(prev_line);
                } else {
                    self.delete_char_before_cursor();
                }
                self.ensure_cursor_visible();
            }
            KeyCode::Delete => {
                if self.cursor_at_end_of_line() && self.cursor_pos.0 < self.lines.len() - 1 {
                    // Join with next line
                    self.join_with_next_line();
                    // This affects the current line
                    self.modified_lines.insert(self.cursor_pos.0);
                } else {
                    self.delete_char_at_cursor();
                }
                self.ensure_cursor_visible();
            }
            KeyCode::Up => {
                self.move_cursor_up();
                self.ensure_cursor_visible();
            }
            KeyCode::Down => {
                self.move_cursor_down();
                self.ensure_cursor_visible();
            }
            KeyCode::Left => {
                self.move_cursor_left();
                self.ensure_cursor_visible();
            }
            KeyCode::Right => {
                self.move_cursor_right();
                self.ensure_cursor_visible();
            }
            KeyCode::Home => {
                self.move_cursor_to_start_of_line();
            }
            KeyCode::End => {
                self.move_cursor_to_end_of_line();
            }
            KeyCode::PageUp => {
                // Move cursor up by the number of visible lines
                if let Some((_, _, _, h)) = self.input_panel_area {
                    let visible_lines = h.saturating_sub(2) as usize;
                    for _ in 0..visible_lines {
                        if self.cursor_pos.0 > 0 {
                            self.move_cursor_up();
                        }
                    }
                }
                self.ensure_cursor_visible();
            }
            KeyCode::PageDown => {
                // Move cursor down by the number of visible lines
                if let Some((_, _, _, h)) = self.input_panel_area {
                    let visible_lines = h.saturating_sub(2) as usize;
                    for _ in 0..visible_lines {
                        if self.cursor_pos.0 < self.lines.len() - 1 {
                            self.move_cursor_down();
                        }
                    }
                }
                self.ensure_cursor_visible();
            }
            KeyCode::Char(c) => {
                self.insert_char(c);
            }
            _ => {}
        }

        // Evaluate the expressions after any change
        self.evaluate_expressions();
    }

    // Make the evaluate_expressions method public so it can be called from outside
    pub fn evaluate_expressions(&mut self) {
        // Clone the current variables state for comparing after evaluation
        let prev_variables = self.variables.clone();
        
        // If there are no modified lines, nothing to do
        if self.modified_lines.is_empty() {
            return;
        }
        
        // Get a sorted list of modified lines
        let mut modified: Vec<usize> = self.modified_lines.iter().cloned().collect();
        modified.sort();
        
        // First pass: evaluate just the modified lines to update variables
        self.evaluate_modified_lines(&modified);
        
        // Second pass: find variables that changed and evaluate dependent lines
        self.evaluate_dependent_lines(&prev_variables);
        
        // Clear the modified lines set
        self.modified_lines.clear();
        
        // Store the current variables state for the next comparison
        self.cached_variables = self.variables.clone();
    }

    // Evaluate the modified lines to update variables
    fn evaluate_modified_lines(&mut self, modified_lines: &[usize]) {
        for &line_idx in modified_lines {
            if line_idx < self.lines.len() {
                let line = &self.lines[line_idx];
                // Skip empty lines and comments
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                
                // Parse and evaluate this line
                let expr = crate::parser::parse_line(line, &self.variables);
                let result = crate::evaluator::evaluate(&expr, &mut self.variables);
                
                // Update the result for this line
                self.update_result_for_line(line_idx, &result);
            }
        }
    }

    // Update the result for a specific line
    fn update_result_for_line(&mut self, line_idx: usize, result: &crate::evaluator::Value) {
        if line_idx < self.results.len() {
            // If it's an assignment, store the variable
            if let crate::evaluator::Value::Assignment(name, value) = result {
                self.variables.insert(name.clone(), (**value).clone());
            }

            // Format the result
            let result_str = if self.last_keystroke.elapsed() < self.debounce_period && matches!(result, crate::evaluator::Value::Error(_)) {
                String::new() // Hide errors during debounce period
            } else {
                match result {
                    crate::evaluator::Value::Error(msg) => format!("Error: {}", msg),
                    _ => format!("{}", result)
                }
            };
            
            // Update the results
            self.results[line_idx] = result_str;
            self.debounced_results[line_idx] = match result {
                crate::evaluator::Value::Error(msg) => format!("Error: {}", msg),
                _ => format!("{}", result)
            };
        }
    }

    // Find variables that changed and evaluate dependent lines
    fn evaluate_dependent_lines(&mut self, prev_variables: &HashMap<String, crate::evaluator::Value>) {
        // Check which variables changed
        let changed_vars = self.find_changed_variables(prev_variables);
        
        // If any variables changed, re-evaluate all lines that use those variables
        if !changed_vars.is_empty() {
            self.reevaluate_dependent_lines(&changed_vars);
        }
    }

    // Find which variables changed compared to previous state
    fn find_changed_variables(&self, prev_variables: &HashMap<String, crate::evaluator::Value>) -> HashSet<String> {
        let mut changed_vars = HashSet::new();
        
        for (var, val) in &self.variables {
            if !prev_variables.contains_key(var) || prev_variables.get(var) != Some(val) {
                changed_vars.insert(var.clone());
            }
        }
        
        changed_vars
    }

    // Re-evaluate lines that depend on changed variables
    fn reevaluate_dependent_lines(&mut self, changed_vars: &HashSet<String>) {
        // Simple approach: re-evaluate all lines that contain any of the changed variables
        for i in 0..self.lines.len() {
            let line = &self.lines[i];
            
            // Check if this line contains any of the changed variables
            // This is a simple string-based check, might have false positives
            let needs_eval = changed_vars.iter().any(|var| line.contains(var));
            
            if needs_eval {
                // Parse and evaluate this line
                let expr = crate::parser::parse_line(line, &self.variables);
                let result = crate::evaluator::evaluate(&expr, &mut self.variables);
                
                // Update the result for this line
                self.update_result_for_line(i, &result);
            }
        }
    }

    // Check if it's time to show errors (called on tick)
    pub fn update_on_tick(&mut self) {
        // If the debounce period has passed since the last keystroke,
        // update results to show any pending errors
        if self.last_keystroke.elapsed() >= self.debounce_period {
            self.results = self.debounced_results.clone();
        }
        
        // Clear status message after 3 seconds
        if let Some(time) = self.status_time {
            if time.elapsed() >= Duration::from_secs(3) {
                self.clear_status_message();
            }
        }
    }

    // Cursor movement and text manipulation methods
    fn insert_char(&mut self, c: char) {
        let line = &mut self.lines[self.cursor_pos.0];
        if self.cursor_pos.1 >= line.len() {
            line.push(c);
        } else {
            line.insert(self.cursor_pos.1, c);
        }
        self.cursor_pos.1 += 1;
    }

    fn delete_char_before_cursor(&mut self) {
        if self.cursor_pos.1 > 0 {
            let line = &mut self.lines[self.cursor_pos.0];
            line.remove(self.cursor_pos.1 - 1);
            self.cursor_pos.1 -= 1;
        }
    }

    fn delete_char_at_cursor(&mut self) {
        let line = &mut self.lines[self.cursor_pos.0];
        if self.cursor_pos.1 < line.len() {
            line.remove(self.cursor_pos.1);
        }
    }

    fn insert_newline(&mut self) {
        let current_line = &self.lines[self.cursor_pos.0];
        let new_line = if self.cursor_pos.1 >= current_line.len() {
            String::new()
        } else {
            current_line[self.cursor_pos.1..].to_string()
        };
        
        self.lines[self.cursor_pos.0] = current_line[..self.cursor_pos.1].to_string();
        self.lines.insert(self.cursor_pos.0 + 1, new_line);
        self.results.insert(self.cursor_pos.0 + 1, String::new());
        self.debounced_results.insert(self.cursor_pos.0 + 1, String::new());
        self.cursor_pos.0 += 1;
        self.cursor_pos.1 = 0;
        
        // Ensure the cursor remains visible after inserting a new line
        self.ensure_cursor_visible();
    }

    fn join_with_previous_line(&mut self) {
        if self.cursor_pos.0 > 0 {
            let current_line = self.lines.remove(self.cursor_pos.0);
            self.results.remove(self.cursor_pos.0);
            self.debounced_results.remove(self.cursor_pos.0);
            let prev_line_idx = self.cursor_pos.0 - 1;
            let prev_line_len = self.lines[prev_line_idx].len();
            self.lines[prev_line_idx].push_str(&current_line);
            self.cursor_pos.0 = prev_line_idx;
            self.cursor_pos.1 = prev_line_len;
        }
    }

    fn join_with_next_line(&mut self) {
        if self.cursor_pos.0 < self.lines.len() - 1 {
            let next_line = self.lines.remove(self.cursor_pos.0 + 1);
            self.results.remove(self.cursor_pos.0 + 1);
            self.debounced_results.remove(self.cursor_pos.0 + 1);
            self.lines[self.cursor_pos.0].push_str(&next_line);
        }
    }

    fn move_cursor_up(&mut self) {
        if self.cursor_pos.0 > 0 {
            self.cursor_pos.0 -= 1;
            let line_len = self.lines[self.cursor_pos.0].len();
            if self.cursor_pos.1 > line_len {
                self.cursor_pos.1 = line_len;
            }
            // Adjust scroll position if cursor moves above visible area
            if let Some((_, _y, _, h)) = self.input_panel_area {
                let _visible_lines = h.saturating_sub(2) as usize; // Subtract 2 for borders
                if self.cursor_pos.0 < self.input_scroll {
                    self.input_scroll = self.cursor_pos.0;
                }
            }
        }
    }

    fn move_cursor_down(&mut self) {
        if self.cursor_pos.0 < self.lines.len() - 1 {
            self.cursor_pos.0 += 1;
            let line_len = self.lines[self.cursor_pos.0].len();
            if self.cursor_pos.1 > line_len {
                self.cursor_pos.1 = line_len;
            }
            // Adjust scroll position if cursor moves below visible area
            if let Some((_, _y, _, h)) = self.input_panel_area {
                let visible_lines = h.saturating_sub(2) as usize; // Subtract 2 for borders
                if self.cursor_pos.0 >= self.input_scroll + visible_lines {
                    self.input_scroll = self.cursor_pos.0.saturating_sub(visible_lines) + 1;
                }
            }
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_pos.1 > 0 {
            self.cursor_pos.1 -= 1;
        } else if self.cursor_pos.0 > 0 {
            self.cursor_pos.0 -= 1;
            self.cursor_pos.1 = self.lines[self.cursor_pos.0].len();
        }
    }

    fn move_cursor_right(&mut self) {
        let line_len = self.lines[self.cursor_pos.0].len();
        if self.cursor_pos.1 < line_len {
            self.cursor_pos.1 += 1;
        } else if self.cursor_pos.0 < self.lines.len() - 1 {
            self.cursor_pos.0 += 1;
            self.cursor_pos.1 = 0;
        }
    }

    fn move_cursor_to_start_of_line(&mut self) {
        self.cursor_pos.1 = 0;
    }

    fn move_cursor_to_end_of_line(&mut self) {
        self.cursor_pos.1 = self.lines[self.cursor_pos.0].len();
    }

    fn cursor_at_start_of_line(&self) -> bool {
        self.cursor_pos.1 == 0
    }

    fn cursor_at_end_of_line(&self) -> bool {
        self.cursor_pos.1 == self.lines[self.cursor_pos.0].len()
    }

    // Toggle panel focus between input and output
    pub fn toggle_panel_focus(&mut self, forward: bool) {
        self.panel_focus = match (self.panel_focus, forward) {
            (PanelFocus::Input, true) | (PanelFocus::Input, false) => {
                if !self.results.is_empty() {
                    self.output_selected_idx = self.output_selected_idx.min(self.results.len() - 1);
                } else {
                    self.output_selected_idx = 0;
                }
                PanelFocus::Output
            },
            (PanelFocus::Output, true) | (PanelFocus::Output, false) => {
                PanelFocus::Input
            },
        };
    }
    
    // Handle navigation in the output panel
    pub fn navigate_output_panel(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.output_selected_idx > 0 {
                    self.output_selected_idx -= 1;
                    // Adjust scroll position if selection moves above visible area
                    if let Some((_, _, _, h)) = self.output_panel_area {
                        let _visible_lines = h.saturating_sub(2) as usize; // Subtract 2 for borders
                        if self.output_selected_idx < self.output_scroll {
                            self.output_scroll = self.output_selected_idx;
                        }
                    }
                }
            },
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.results.is_empty() && self.output_selected_idx < self.results.len() - 1 {
                    self.output_selected_idx += 1;
                    // Adjust scroll position if selection moves below visible area
                    if let Some((_, _, _, h)) = self.output_panel_area {
                        let visible_lines = h.saturating_sub(2) as usize; // Subtract 2 for borders
                        if self.output_selected_idx >= self.output_scroll + visible_lines {
                            self.output_scroll = self.output_selected_idx.saturating_sub(visible_lines) + 1;
                        }
                    }
                }
            },
            KeyCode::Home | KeyCode::Char('g') => {
                // Go to top (vim gg)
                self.output_selected_idx = 0;
                self.output_scroll = 0;
            },
            KeyCode::End | KeyCode::Char('G') => {
                // Go to bottom (vim G)
                if !self.results.is_empty() {
                    self.output_selected_idx = self.results.len() - 1;
                    // Adjust scroll position
                    if let Some((_, _, _, h)) = self.output_panel_area {
                        let visible_lines = h.saturating_sub(2) as usize;
                        self.output_scroll = self.output_selected_idx.saturating_sub(visible_lines.saturating_sub(1));
                    }
                }
            },
            _ => {}
        }
    }
    
    // Copy selected output to clipboard
    pub fn copy_selected_output_to_clipboard(&self) -> Result<(), String> {
        if self.results.is_empty() || self.output_selected_idx >= self.results.len() {
            return Err("No output selected to copy".to_string());
        }
        
        let output = &self.results[self.output_selected_idx];
        if output.is_empty() {
            return Err("Selected output is empty".to_string());
        }

        // Don't copy error messages
        if output.starts_with("Error:") {
            return Err("Cannot copy error messages".to_string());
        }
        
        // In WSL, simply use clip.exe which is the most reliable method
        if let Ok(_) = std::env::var("WSL_DISTRO_NAME") {
            match std::process::Command::new("clip.exe")
                .stdin(std::process::Stdio::piped())
                .spawn() 
            {
                Ok(mut child) => {
                    if let Some(stdin) = child.stdin.as_mut() {
                        match std::io::Write::write_all(stdin, output.as_bytes()) {
                            Ok(_) => {
                                // Wait for the process to complete to ensure the text is copied
                                if let Ok(_) = child.wait() {
                                    return Ok(());
                                }
                            },
                            Err(e) => return Err(format!("Failed to write to clip.exe: {}", e)),
                        }
                    }
                    return Err("Failed to access clip.exe stdin".to_string());
                },
                Err(e) => return Err(format!("Failed to launch clip.exe: {}", e)),
            }
        }
        
        // For non-WSL environments, try arboard
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => {
                match clipboard.set_text(output.clone()) {
                    Ok(_) => return Ok(()),
                    Err(e) => return Err(format!("Clipboard error: {}", e)),
                }
            },
            Err(e) => return Err(format!("Failed to access clipboard: {}", e)),
        }
    }

    // Handle mouse click events
    pub fn handle_mouse_click(&mut self, x: u16, y: u16, area: (u16, u16, u16, u16)) -> bool {
        let (input_x, input_y, input_width, input_height) = area;
        
        // Check if click is within input panel bounds (including borders)
        if x >= input_x && x < input_x + input_width && 
           y >= input_y && y < input_y + input_height {
            // Switch focus to input panel
            self.panel_focus = PanelFocus::Input;
            
            // If click is within the content area (excluding borders)
            if x > input_x && x < input_x + input_width - 1 && 
               y > input_y && y < input_y + input_height - 1 {
                // Convert screen coordinates to text coordinates (accounting for borders)
                let text_x = (x - input_x - 1) as usize;
                let text_y = (y - input_y - 1) as usize + self.input_scroll;
                
                // Check if we have a line at this y position
                if text_y < self.lines.len() {
                    // Set cursor position
                    self.cursor_pos.0 = text_y;
                    // Set x position, clamped to line length
                    self.cursor_pos.1 = text_x.min(self.lines[text_y].len());
                }
            }
            return true;
        }
        
        false
    }

    // Handle mouse click in output panel
    pub fn handle_output_mouse_click(&mut self, x: u16, y: u16, area: (u16, u16, u16, u16)) -> bool {
        let (output_x, output_y, output_width, output_height) = area;
        
        // Check if click is within output panel bounds
        if x >= output_x && x < output_x + output_width && 
           y >= output_y && y < output_y + output_height {
            // Switch focus to output panel
            self.panel_focus = PanelFocus::Output;
            
            // If click is within the content area (excluding borders)
            if x > output_x && x < output_x + output_width - 1 && 
               y > output_y && y < output_y + output_height - 1 {
                let text_y = (y - output_y - 1) as usize + self.output_scroll;
                
                // Check if we have a result at this y position
                if text_y < self.results.len() {
                    self.output_selected_idx = text_y;
                }
            }
            return true;
        }
        
        false
    }

    pub fn ensure_cursor_visible(&mut self) {
        if let Some((_, _, _, h)) = self.input_panel_area {
            let visible_lines = h.saturating_sub(2) as usize; // Subtract 2 for borders
            
            // If cursor is above visible area, scroll up
            if self.cursor_pos.0 < self.input_scroll {
                self.input_scroll = self.cursor_pos.0;
            }
            // If cursor is below visible area, scroll down
            else if self.cursor_pos.0 >= self.input_scroll + visible_lines {
                self.input_scroll = self.cursor_pos.0.saturating_sub(visible_lines) + 1;
            }
        }
    }
} 