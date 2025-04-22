use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    prelude::Alignment,
    Frame,
};
use crate::app::App;
use regex::Regex;
use once_cell::sync::Lazy;

// Define regex patterns for syntax highlighting
static NUMBER_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d+(?:\.\d+)?)").unwrap());
static PERCENTAGE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d+(?:\.\d+)?%)").unwrap());
static UNIT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b([A-Za-z][A-Za-z0-9_]*)\b").unwrap());
static OPERATOR_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"([\+\-\*/\^=])").unwrap());
static BRACKET_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"([\(\)\[\]\{\}])").unwrap());
static KEYWORD_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b(to|in|of|what|is|next)\b").unwrap());
static SPECIAL_WORD_REGEX: Lazy<Regex> = Lazy::new(|| 
    Regex::new(r"\b(monday|tuesday|wednesday|thursday|friday|saturday|sunday|week|month|day|weeks|months|days)\b").unwrap()
);
static COMMENT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(#.*)").unwrap());

pub fn draw(f: &mut Frame, app: &mut App) {
    // Create main layout with header, content, and status areas
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),      // Header
            Constraint::Min(1),         // Content area
            Constraint::Length(1)       // Status bar
        ].as_ref())
        .split(f.size());
    
    // Draw the branding in the header
    draw_header(f, main_chunks[0]);
    
    // Split the content area into two horizontal panels
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(main_chunks[1]);

    // Store panel areas for mouse handling
    app.input_panel_area = Some((
        content_chunks[0].x,
        content_chunks[0].y,
        content_chunks[0].width,
        content_chunks[0].height
    ));
    app.output_panel_area = Some((
        content_chunks[1].x,
        content_chunks[1].y,
        content_chunks[1].width,
        content_chunks[1].height
    ));

    draw_input_panel(f, app, content_chunks[0]);
    draw_output_panel(f, app, content_chunks[1]);
    
    // Draw the status bar
    draw_status_bar(f, app, main_chunks[2]);
}

// Function to draw the header with Cali branding
fn draw_header(f: &mut Frame, area: Rect) {
    // Create a block for the header with no borders
    let header_block = Block::default()
        .style(Style::default());
    
    // Create a paragraph with the Cali text and version
    let header = Paragraph::new(Line::from(vec![
        Span::styled("Cali", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" v{}", env!("CARGO_PKG_VERSION")), Style::default().fg(Color::DarkGray)),
    ]))
    .block(header_block)
    .alignment(Alignment::Left);

    f.render_widget(header, area);
}

fn draw_input_panel(f: &mut Frame, app: &App, area: Rect) {
    // Create a block for the input area with a style based on focus
    let input_block = Block::default()
        .title("Input")
        .borders(Borders::ALL)
        .style(Style::default().fg(if app.panel_focus == crate::app::PanelFocus::Input {
            Color::Cyan
        } else {
            Color::White
        }));

    // Convert lines to styled list items with syntax highlighting
    let items: Vec<ListItem> = app.lines
        .iter()
        .enumerate()
        .map(|(_, line)| {
            // Apply syntax highlighting to this line
            let highlighted_line = highlight_syntax(line);
            
            ListItem::new(highlighted_line)
        })
        .collect();

    // Create the list widget
    let input_list = List::new(items)
        .block(input_block)
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    f.render_widget(input_list, area);

    // Only show cursor in the input panel if it has focus
    if app.panel_focus == crate::app::PanelFocus::Input && app.lines.len() > app.cursor_pos.0 {
        let line = &app.lines[app.cursor_pos.0];
        let cursor_x = if app.cursor_pos.1 <= line.len() { 
            app.cursor_pos.1 as u16 
        } else { 
            line.len() as u16 
        };

        // Cursor is in input area, offset by border and line number
        f.set_cursor(
            area.x + cursor_x + 1, // +1 for border
            area.y + app.cursor_pos.0 as u16 + 1, // +1 for border
        );
    }
}

// Function to apply syntax highlighting to a line of text
fn highlight_syntax(text: &str) -> Line {
    // Start with an empty list of spans
    let mut spans = Vec::new();
    
    // Keep track of which parts of the text have been processed
    let mut processed_indices = vec![false; text.len()];
    
    // Find and highlight comments (both full line and inline)
    for captures in COMMENT_REGEX.captures_iter(text) {
        if let Some(m) = captures.get(1) {
            mark_as_processed(&mut processed_indices, m.start(), m.end());
            spans.push((m.start(), m.end(), Span::styled(
                m.as_str().to_string(),
                Style::default().fg(Color::DarkGray)
            )));
            
            // If it starts at the beginning of the line, it's a full comment line
            if m.start() == 0 {
                return Line::from(spans.into_iter().map(|(_, _, span)| span).collect::<Vec<_>>());
            }
        }
    }
    
    // Find and highlight percentages (must come before numbers)
    for captures in PERCENTAGE_REGEX.captures_iter(text) {
        if let Some(m) = captures.get(1) {
            mark_as_processed(&mut processed_indices, m.start(), m.end());
            spans.push((m.start(), m.end(), Span::styled(
                m.as_str().to_string(),
                Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)
            )));
        }
    }
    
    // Find and highlight numbers, but only if they're not already marked as processed
    for captures in NUMBER_REGEX.captures_iter(text) {
        if let Some(m) = captures.get(1) {
            // Skip if already processed (e.g., part of a percentage)
            if is_already_processed(&processed_indices, m.start(), m.end()) {
                continue;
            }
            
            mark_as_processed(&mut processed_indices, m.start(), m.end());
            spans.push((m.start(), m.end(), Span::styled(
                m.as_str().to_string(),
                Style::default().fg(Color::LightYellow)
            )));
        }
    }
    
    // Find and highlight operators
    for captures in OPERATOR_REGEX.captures_iter(text) {
        if let Some(m) = captures.get(1) {
            // Skip if already processed
            if is_already_processed(&processed_indices, m.start(), m.end()) {
                continue;
            }
            
            mark_as_processed(&mut processed_indices, m.start(), m.end());
            spans.push((m.start(), m.end(), Span::styled(
                m.as_str().to_string(),
                Style::default().fg(Color::LightRed)
            )));
        }
    }
    
    // Find and highlight brackets
    for captures in BRACKET_REGEX.captures_iter(text) {
        if let Some(m) = captures.get(1) {
            // Skip if already processed
            if is_already_processed(&processed_indices, m.start(), m.end()) {
                continue;
            }
            
            mark_as_processed(&mut processed_indices, m.start(), m.end());
            spans.push((m.start(), m.end(), Span::styled(
                m.as_str().to_string(),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            )));
        }
    }
    
    // Find and highlight keywords
    for captures in KEYWORD_REGEX.captures_iter(text) {
        if let Some(m) = captures.get(1) {
            // Skip if already processed
            if is_already_processed(&processed_indices, m.start(), m.end()) {
                continue;
            }
            
            mark_as_processed(&mut processed_indices, m.start(), m.end());
            spans.push((m.start(), m.end(), Span::styled(
                m.as_str().to_string(),
                Style::default().fg(Color::LightBlue)
            )));
        }
    }
    
    // Find and highlight special words (days, units)
    for captures in SPECIAL_WORD_REGEX.captures_iter(text) {
        if let Some(m) = captures.get(1) {
            // Skip if already processed
            if is_already_processed(&processed_indices, m.start(), m.end()) {
                continue;
            }
            
            mark_as_processed(&mut processed_indices, m.start(), m.end());
            spans.push((m.start(), m.end(), Span::styled(
                m.as_str().to_string(),
                Style::default().fg(Color::LightMagenta)
            )));
        }
    }
    
    // Find and highlight units
    for captures in UNIT_REGEX.captures_iter(text) {
        if let Some(m) = captures.get(1) {
            // Skip if already processed
            if is_already_processed(&processed_indices, m.start(), m.end()) {
                continue;
            }
            
            // Check if this is a currency unit (3 letters, all uppercase)
            let is_currency = m.as_str().len() == 3 && m.as_str().chars().all(|c| c.is_ascii_uppercase());
            
            mark_as_processed(&mut processed_indices, m.start(), m.end());
            spans.push((m.start(), m.end(), Span::styled(
                m.as_str().to_string(),
                Style::default().fg(if is_currency { Color::LightGreen } else { Color::LightCyan })
            )));
        }
    }
    
    // Add any remaining unprocessed text as plain spans
    let mut start = 0;
    for i in 0..text.len() {
        if !processed_indices[i] && (i == 0 || processed_indices[i-1]) {
            start = i;
        }
        
        if !processed_indices[i] && (i == text.len() - 1 || processed_indices[i+1]) {
            spans.push((start, i+1, Span::styled(
                text[start..=i].to_string(),
                Style::default().fg(Color::White)
            )));
        }
    }
    
    // Sort spans by start position
    spans.sort_by_key(|(start, _, _)| *start);
    
    // Extract just the spans for the Line
    Line::from(spans.into_iter().map(|(_, _, span)| span).collect::<Vec<_>>())
}

// Helper function to mark indices as processed
fn mark_as_processed(processed: &mut Vec<bool>, start: usize, end: usize) {
    for i in start..end {
        processed[i] = true;
    }
}

// Helper function to check if a range is already processed
fn is_already_processed(processed: &Vec<bool>, start: usize, end: usize) -> bool {
    for i in start..end {
        if processed[i] {
            return true;
        }
    }
    false
}

fn draw_output_panel(f: &mut Frame, app: &App, area: Rect) {
    // Create a block for the output area with a style based on focus
    let output_block = Block::default()
        .title("Output")
        .borders(Borders::ALL)
        .style(Style::default().fg(if app.panel_focus == crate::app::PanelFocus::Output {
            Color::Cyan
        } else {
            Color::White
        }));

    // Define the inner area (inside the borders)
    let inner_area = output_block.inner(area);
    
    // Render the block
    f.render_widget(output_block, area);

    // Convert result lines to styled list items
    let items: Vec<ListItem> = app.results
        .iter()
        .enumerate()
        .map(|(idx, result)| {
            // Check if this is the selected line
            let is_selected = app.panel_focus == crate::app::PanelFocus::Output && idx == app.output_selected_idx;
            
            // Style based on content and selection
            let line_style = if is_selected {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else if result.starts_with("Error:") {
                Style::default().fg(Color::Red)
            } else {
                Style::default()
            };
            
            // Apply styling to the line
            if result.starts_with("Error:") {
                // For error messages, style with red background and white text
                ListItem::new(Line::from(Span::styled(result.clone(), 
                    if is_selected {
                        Style::default()
                            .fg(Color::White)
                            .bg(Color::Red)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::White)
                            .bg(Color::Red)
                    }
                )))
            } else if result.is_empty() {
                // Empty result, just create an empty line with the appropriate style
                ListItem::new(Line::from(Span::styled("", line_style)))
            } else {
                // Apply syntax highlighting for normal results
                let highlighted = highlight_syntax(result);
                
                // If this is the selected line in output focus mode, apply background highlight to all spans
                if is_selected {
                    let styled_spans = highlighted.spans.iter().map(|span| {
                        let mut style = span.style;
                        style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
                        Span::styled(span.content.clone(), style)
                    }).collect::<Vec<_>>();
                    
                    ListItem::new(Line::from(styled_spans))
                } else {
                    ListItem::new(highlighted)
                }
            }
        })
        .collect();

    // Create the list widget
    let output_list = List::new(items);
    
    // Render the list inside the inner area
    f.render_widget(output_list, inner_area);
    
    // Draw a fill rectangle behind the currently selected line for vim-like highlighting
    if app.panel_focus == crate::app::PanelFocus::Output && !app.results.is_empty() {
        let selected_idx = app.output_selected_idx;
        if selected_idx < app.results.len() {
            // Calculate the y-position of the selected line
            let y_position = inner_area.y + selected_idx as u16;
            
            // Only highlight if the line is within the visible area
            if y_position >= inner_area.y && y_position < inner_area.y + inner_area.height {
                // Create a rectangle that spans the entire width of the inner area
                let highlight_area = Rect {
                    x: inner_area.x,
                    y: y_position,
                    width: inner_area.width,
                    height: 1,
                };
                
                // Create a blank paragraph with the highlight style
                let highlight = Paragraph::new("")
                    .style(Style::default().bg(Color::DarkGray));
                
                // Render the highlight underneath the text
                f.render_widget(highlight, highlight_area);
            }
        }
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    match app.input_mode {
        crate::app::InputMode::Normal => {
            // Normal mode: display status message
            let status_text = match &app.status_message {
                Some(message) => message.as_str(),
                None => ""
            };
            
            let status_bar = Paragraph::new(status_text)
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .block(Block::default());
            
            f.render_widget(status_bar, area);
        },
        crate::app::InputMode::FilePath => {
            // Input mode: show input field for file path
            let prompt = "Enter file path to save to: ";
            let input_text = format!("{}{}", prompt, app.status_input);
            
            let status_bar = Paragraph::new(input_text)
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default());
            
            f.render_widget(status_bar, area);
            
            // Set cursor position at the end of input
            f.set_cursor(
                area.x + (prompt.len() + app.status_input.len()) as u16,
                area.y,
            );
        }
    }
} 