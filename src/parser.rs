use std::collections::HashMap;
use regex::Regex;
use crate::evaluator::Value;

// Expression type enum
#[derive(Debug, Clone)]
pub enum Expr {
    Assignment(String, Box<Expr>),
    BinaryOp(Box<Expr>, Op, Box<Expr>),
    Number(f64),
    Variable(String),
    UnitValue(f64, String),
    PercentOf(Box<Expr>, Box<Expr>),
    Convert(Box<Expr>, String),
    DateOffset(String, i64, String),
    Error(String),
    Percentage(f64),
}

// Operation enum
#[derive(Debug, Clone)]
pub enum Op {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
}

// Parse a line of input into an expression
pub fn parse_line(line: &str, variables: &HashMap<String, Value>) -> Expr {
    // Remove any inline comments (anything after #)
    let line = if let Some(pos) = line.find('#') {
        line[..pos].trim()
    } else {
        line.trim()
    };
    
    // Handle empty lines
    if line.is_empty() {
        return Expr::Error("Empty expression".to_string());
    }
    
    // Try to parse as a setrate command
    if let Some(rate_expr) = parse_set_rate(line) {
        return rate_expr;
    }
    
    // Try to parse as an assignment
    if let Some(assignment) = parse_assignment(line, variables) {
        return assignment;
    }
    
    // Try to parse as a unit conversion
    if let Some(conversion) = parse_conversion(line, variables) {
        return conversion;
    }
    
    // Try to parse as a percentage calculation
    if let Some(percentage) = parse_percentage(line, variables) {
        return percentage;
    }
    
    // Try to parse as a date expression
    if let Some(date_expr) = parse_date_expression(line) {
        return date_expr;
    }
    
    // Try to parse as a binary operation
    if let Some(binary_op) = parse_binary_op(line, variables) {
        return binary_op;
    }
    
    // Try to parse as a simple value (number, variable, or unit value)
    parse_simple_value(line, variables)
}

// Parse a setrate command (setrate USD to EUR = 0.92)
fn parse_set_rate(line: &str) -> Option<Expr> {
    let re = Regex::new(r"(?i)setrate\s+([A-Z]{3})\s+(?:to|in)\s+([A-Z]{3})\s*=\s*(\d+(?:\.\d+)?)").ok()?;
    if let Some(caps) = re.captures(line) {
        let from_currency = caps[1].to_uppercase();
        let to_currency = caps[2].to_uppercase();
        if let Ok(rate) = caps[3].parse::<f64>() {
            // Call the currency module to set the rate
            if crate::currency::set_exchange_rate(&from_currency, &to_currency, rate) {
                return Some(Expr::UnitValue(rate, to_currency));
            }
        }
    }
    None
}

// Parse an assignment expression (var = expr)
fn parse_assignment(line: &str, variables: &HashMap<String, Value>) -> Option<Expr> {
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() == 2 {
        let var_name = parts[0].trim().to_string();
        let expr = parse_line(parts[1], variables);
        Some(Expr::Assignment(var_name, Box::new(expr)))
    } else {
        None
    }
}

// Parse a unit conversion expression (expr in unit)
fn parse_conversion(line: &str, variables: &HashMap<String, Value>) -> Option<Expr> {
    // Match pattern like "X in Y" or "X to Y"
    let re = Regex::new(r"(.+)\s+(?:in|to)\s+(.+)").ok()?;
    if let Some(caps) = re.captures(line) {
        let value_expr = parse_line(&caps[1], variables);
        let target_unit = caps[2].trim().to_string();
        Some(Expr::Convert(Box::new(value_expr), target_unit))
    } else {
        None
    }
}

// Parse a percentage expression (X% of Y)
fn parse_percentage(line: &str, variables: &HashMap<String, Value>) -> Option<Expr> {
    // Handle X% of Y
    let re = Regex::new(r"(.+)%\s+of\s+(.+)").ok()?;
    if let Some(caps) = re.captures(line) {
        let percent_expr = parse_simple_value(&caps[1], variables);
        let value_expr = parse_line(&caps[2], variables);
        Some(Expr::PercentOf(Box::new(percent_expr), Box::new(value_expr)))
    } else {
        // Handle "X of Y" where X is a variable that might be a percentage
        let re = Regex::new(r"(\w+)\s+of\s+(.+)").ok()?;
        if let Some(caps) = re.captures(line) {
            let var_name = caps[1].trim();
            if variables.contains_key(var_name) {
                let percent_expr = Expr::Variable(var_name.to_string());
                let value_expr = parse_line(&caps[2], variables);
                return Some(Expr::PercentOf(Box::new(percent_expr), Box::new(value_expr)));
            }
        }
        
        // Alternative pattern: "X of what is Y"
        let re = Regex::new(r"(.+)\s+of\s+what\s+is\s+(.+)").ok()?;
        if let Some(caps) = re.captures(line) {
            let percent_expr = parse_simple_value(&caps[1], variables);
            let result_expr = parse_line(&caps[2], variables);
            // If X% of Y = Z, then Y = Z / (X/100)
            Some(Expr::PercentOf(Box::new(percent_expr), Box::new(result_expr)))
        } else {
            None
        }
    }
}

// Parse a date expression (next friday + 2 weeks)
fn parse_date_expression(line: &str) -> Option<Expr> {
    // Simple pattern for "next X + Y Z" where X is a day, Y is a number, Z is a unit
    let re = Regex::new(r"(?i)next\s+(\w+)(?:\s*\+\s*(\d+)\s+(\w+))?").ok()?;
    if let Some(caps) = re.captures(line) {
        let day = caps[1].to_lowercase();
        let amount = caps.get(2).map_or(0, |m| m.as_str().parse::<i64>().unwrap_or(0));
        // Store the lowercase unit in a new variable to avoid the temporary value issue
        let unit = if let Some(m) = caps.get(3) {
            m.as_str().to_lowercase()
        } else {
            "days".to_string()
        };
        
        Some(Expr::DateOffset(day, amount, unit))
    } else {
        None
    }
}

// Parse a binary operation (expr op expr)
fn parse_binary_op(line: &str, variables: &HashMap<String, Value>) -> Option<Expr> {
    // First, check for addition or subtraction
    let re = Regex::new(r"(.+?)([+\-])(.+)").ok()?;
    if let Some(caps) = re.captures(line) {
        let left = parse_line(&caps[1], variables);
        let right = parse_line(&caps[3], variables);
        
        let op = match &caps[2] {
            "+" => Op::Add,
            "-" => Op::Subtract,
            _ => return None,
        };
        
        return Some(Expr::BinaryOp(Box::new(left), op, Box::new(right)));
    }
    
    // If no addition/subtraction, check for multiplication, division, etc.
    let re = Regex::new(r"(.+?)([*/^%])(.+)").ok()?;
    if let Some(caps) = re.captures(line) {
        let left = parse_line(&caps[1], variables);
        let right = parse_line(&caps[3], variables);
        
        let op = match &caps[2] {
            "*" => Op::Multiply,
            "/" => Op::Divide,
            "^" => Op::Power,
            "%" => Op::Modulo,
            _ => return None,
        };
        
        return Some(Expr::BinaryOp(Box::new(left), op, Box::new(right)));
    }
    
    None
}

// Parse a value with a unit (10 USD, 5 kg, etc.)
fn parse_unit_value(text: &str) -> Option<(f64, String)> {
    // Pattern for numbers with units: "10 USD", "5.2 kg", "3 m2", etc.
    // This handles both pure alphabetic units (USD, kg) and units with numbers (m2, km2)
    let number_unit_re = Regex::new(r"(-?\d+(?:\.\d+)?)\s*([a-zA-Z][a-zA-Z0-9]*)").ok()?;
    if let Some(caps) = number_unit_re.captures(text) {
        let value = caps[1].parse::<f64>().ok()?;
        let unit = caps[2].trim().to_string();
        return Some((value, unit));
    }
    
    // We didn't find a number with a unit directly, let's return None
    None
}

// Parse a simple value (number, variable, or unit value)
fn parse_simple_value(line: &str, variables: &HashMap<String, Value>) -> Expr {
    let line = line.trim();
    
    // Try to parse as a percentage (e.g., "8%")
    if line.ends_with("%") {
        if let Ok(num) = line[..line.len()-1].trim().parse::<f64>() {
            return Expr::Percentage(num);
        }
    }
    
    // Try to parse as a number with a unit
    if let Some((value, unit)) = parse_unit_value(line) {
        return Expr::UnitValue(value, unit);
    }
    
    // Check for the pattern "variable unit" (e.g., "z USD")
    let var_unit_re = Regex::new(r"([a-zA-Z][a-zA-Z0-9]*)\s+([A-Z]{3})").ok();
    if let Some(re) = var_unit_re {
        if let Some(caps) = re.captures(line) {
            let var_name = caps[1].trim();
            let unit = caps[2].trim();
            
            if variables.contains_key(var_name) {
                return Expr::BinaryOp(
                    Box::new(Expr::Variable(var_name.to_string())),
                    Op::Multiply,
                    Box::new(Expr::UnitValue(1.0, unit.to_string()))
                );
            }
        }
    }
    
    // Try to parse as a simple number
    if let Ok(num) = line.parse::<f64>() {
        return Expr::Number(num);
    }
    
    // Check if it's a variable
    if variables.contains_key(line) {
        return Expr::Variable(line.to_string());
    }
    
    // If all else fails, return an error expression
    Expr::Error(format!("Cannot parse expression: {}", line))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_number() {
        let variables = HashMap::new();
        match parse_line("42", &variables) {
            Expr::Number(n) => assert_eq!(n, 42.0),
            _ => panic!("Expected Number expression"),
        }
    }
    
    #[test]
    fn test_parse_unit_value() {
        let variables = HashMap::new();
        match parse_line("10 USD", &variables) {
            Expr::UnitValue(v, u) => {
                assert_eq!(v, 10.0);
                assert_eq!(u, "USD");
            },
            _ => panic!("Expected UnitValue expression"),
        }
    }
    
    #[test]
    fn test_parse_assignment() {
        let variables = HashMap::new();
        match parse_line("x = 42", &variables) {
            Expr::Assignment(name, expr) => {
                assert_eq!(name, "x");
                match *expr {
                    Expr::Number(n) => assert_eq!(n, 42.0),
                    _ => panic!("Expected Number expression in assignment"),
                }
            },
            _ => panic!("Expected Assignment expression"),
        }
    }
    
    #[test]
    fn test_parse_binary_op() {
        let variables = HashMap::new();
        match parse_line("5 + 3", &variables) {
            Expr::BinaryOp(left, Op::Add, right) => {
                match *left {
                    Expr::Number(n) => assert_eq!(n, 5.0),
                    _ => panic!("Expected Number expression on left side"),
                }
                match *right {
                    Expr::Number(n) => assert_eq!(n, 3.0),
                    _ => panic!("Expected Number expression on right side"),
                }
            },
            _ => panic!("Expected BinaryOp expression"),
        }
    }
    
    #[test]
    fn test_parse_conversion() {
        let variables = HashMap::new();
        match parse_line("10 ml in l", &variables) {
            Expr::Convert(expr, unit) => {
                assert_eq!(unit, "l");
                match *expr {
                    Expr::UnitValue(v, u) => {
                        assert_eq!(v, 10.0);
                        assert_eq!(u, "ml");
                    },
                    _ => panic!("Expected UnitValue expression in conversion"),
                }
            },
            _ => panic!("Expected Convert expression"),
        }
    }
    
    #[test]
    fn test_parse_percentage() {
        let variables = HashMap::new();
        match parse_line("20% of 50", &variables) {
            Expr::PercentOf(percent, value) => {
                match *percent {
                    Expr::Number(n) => assert_eq!(n, 20.0),
                    _ => panic!("Expected Number expression for percentage"),
                }
                match *value {
                    Expr::Number(n) => assert_eq!(n, 50.0),
                    _ => panic!("Expected Number expression for value"),
                }
            },
            _ => panic!("Expected PercentOf expression"),
        }
    }
    
    #[test]
    fn test_parse_date_expression() {
        match parse_line("next friday", &HashMap::new()) {
            Expr::DateOffset(day, amount, unit) => {
                assert_eq!(day, "friday");
                assert_eq!(amount, 0);
                assert_eq!(unit, "days");
            },
            _ => panic!("Expected DateOffset expression"),
        }
        
        match parse_line("next monday + 2 weeks", &HashMap::new()) {
            Expr::DateOffset(day, amount, unit) => {
                assert_eq!(day, "monday");
                assert_eq!(amount, 2);
                assert_eq!(unit, "weeks");
            },
            _ => panic!("Expected DateOffset expression"),
        }
    }
} 