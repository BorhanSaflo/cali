use std::collections::HashMap;
use std::time::{Instant, Duration};
use crossterm::event::{KeyEvent, KeyCode};
use crate::evaluator::{Value, evaluate_lines};

pub struct App {
    pub lines: Vec<String>,
    pub cursor_pos: (usize, usize), // (line, column)
    pub variables: HashMap<String, Value>,
    pub results: Vec<String>,          // Real-time results (without errors if within debounce period)
    pub debounced_results: Vec<String>, // Complete results (with errors) after debounce period
    pub last_keystroke: Instant,       // Time of last keystroke
    pub debounce_period: Duration,     // Debounce period for showing errors
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
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        // Update last keystroke time
        self.last_keystroke = Instant::now();
        
        match key.code {
            KeyCode::Enter => {
                self.insert_newline();
            }
            KeyCode::Backspace => {
                if self.cursor_at_start_of_line() && self.cursor_pos.0 > 0 {
                    // Join with previous line
                    self.join_with_previous_line();
                } else {
                    self.delete_char_before_cursor();
                }
            }
            KeyCode::Delete => {
                if self.cursor_at_end_of_line() && self.cursor_pos.0 < self.lines.len() - 1 {
                    // Join with next line
                    self.join_with_next_line();
                } else {
                    self.delete_char_at_cursor();
                }
            }
            KeyCode::Up => {
                self.move_cursor_up();
            }
            KeyCode::Down => {
                self.move_cursor_down();
            }
            KeyCode::Left => {
                self.move_cursor_left();
            }
            KeyCode::Right => {
                self.move_cursor_right();
            }
            KeyCode::Home => {
                self.move_cursor_to_start_of_line();
            }
            KeyCode::End => {
                self.move_cursor_to_end_of_line();
            }
            KeyCode::Char(c) => {
                self.insert_char(c);
            }
            _ => {}
        }

        // Evaluate the expressions after any change
        self.evaluate_expressions();
    }

    fn evaluate_expressions(&mut self) {
        // Always calculate the full results, but we'll filter errors for display
        let full_results = evaluate_lines(&self.lines, &mut self.variables);
        
        // Store the full results for later (for debouncing)
        self.debounced_results = full_results.clone();
        
        // For immediate display, filter out errors if we're within the debounce period
        if self.last_keystroke.elapsed() < self.debounce_period {
            // Only show errors for lines that haven't changed recently
            self.results = full_results
                .iter()
                .map(|result| {
                    if result.starts_with("Error:") {
                        // Temporarily hide errors
                        String::new()
                    } else {
                        result.clone()
                    }
                })
                .collect();
        } else {
            // Outside debounce period, show everything including errors
            self.results = full_results;
        }
    }

    // Check if it's time to show errors (called on tick)
    pub fn update_on_tick(&mut self) {
        // If the debounce period has passed since the last keystroke,
        // update results to show any pending errors
        if self.last_keystroke.elapsed() >= self.debounce_period {
            self.results = self.debounced_results.clone();
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
        }
    }

    fn move_cursor_down(&mut self) {
        if self.cursor_pos.0 < self.lines.len() - 1 {
            self.cursor_pos.0 += 1;
            let line_len = self.lines[self.cursor_pos.0].len();
            if self.cursor_pos.1 > line_len {
                self.cursor_pos.1 = line_len;
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
} 