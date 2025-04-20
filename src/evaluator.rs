use std::collections::HashMap;
use chrono::{NaiveDate, Local, Datelike, Duration, Weekday};
use crate::parser::{Expr, Op};

// Value types that can be stored in variables
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Percentage(f64),
    Unit(f64, String),
    Date(NaiveDate),
    Error(String),
    Assignment(String, Box<Value>),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => {
                // Format integers without decimals, format decimals with up to 6 places
                if n.fract() == 0.0 {
                    write!(f, "{:.0}", n)
                } else {
                    // First try with 2 decimal places
                    let s = format!("{:.2}", n);
                    // If it rounds back to the original value, use that
                    if let Ok(parsed) = s.parse::<f64>() {
                        if (parsed - n).abs() < 1e-10 {
                            return write!(f, "{}", s);
                        }
                    }
                    // Otherwise use 6 decimal places
                    write!(f, "{:.6}", n)
                }
            },
            Value::Percentage(p) => write!(f, "{}%", p),
            Value::Unit(v, u) => {
                // Special handling for currencies (3-letter uppercase codes)
                let is_currency = is_currency_code(u);
                
                if is_currency {
                    match u.as_str() {
                        "USD" => {
                            if v.fract() == 0.0 {
                                write!(f, "${:.0}", v)
                            } else {
                                write!(f, "${:.2}", v)
                            }
                        },
                        "EUR" => {
                            if v.fract() == 0.0 {
                                write!(f, "€{:.0}", v)
                            } else {
                                write!(f, "€{:.2}", v)
                            }
                        },
                        "GBP" => {
                            if v.fract() == 0.0 {
                                write!(f, "£{:.0}", v)
                            } else {
                                write!(f, "£{:.2}", v)
                            }
                        },
                        // For other currencies, use the regular format
                        _ => {
                            if v.fract() == 0.0 {
                                write!(f, "{:.0} {}", v, u)
                            } else {
                                write!(f, "{:.2} {}", v, u)
                            }
                        }
                    }
                } else if v.fract() == 0.0 {
                    write!(f, "{:.0} {}", v, u)
                } else {
                    // First try with 2 decimal places
                    let s = format!("{:.2}", v);
                    // If it rounds back to the original value, use that
                    if let Ok(parsed) = s.parse::<f64>() {
                        if (parsed - v).abs() < 1e-10 {
                            return write!(f, "{} {}", s, u);
                        }
                    }
                    // Otherwise use 6 decimal places
                    write!(f, "{:.6} {}", v, u)
                }
            },
            Value::Date(d) => write!(f, "{}", d),
            Value::Error(e) => write!(f, "Error: {}", e),
            Value::Assignment(_, value) => write!(f, "{}", value),
        }
    }
}

// Evaluate an expression to a value
pub fn evaluate(expr: &Expr, variables: &mut HashMap<String, Value>) -> Value {
    match expr {
        Expr::Number(n) => Value::Number(*n),
        
        Expr::Percentage(p) => Value::Percentage(*p),
        
        Expr::Variable(name) => {
            if let Some(value) = variables.get(name) {
                value.clone()
            } else {
                Value::Error(format!("Unknown variable: {}", name))
            }
        },
        
        Expr::UnitValue(value, unit) => {
            Value::Unit(*value, unit.clone())
        },
        
        Expr::Assignment(name, expr) => {
            let value = evaluate(expr, variables);
            // Return a special value that indicates an assignment was made
            Value::Assignment(name.clone(), Box::new(value.clone()))
        },
        
        Expr::BinaryOp(left, op, right) => {
            evaluate_binary_op(left, op, right, variables)
        },
        
        Expr::PercentOf(percent, value) => {
            evaluate_percent_of(percent, value, variables)
        },
        
        Expr::Convert(value_expr, target_unit) => {
            convert_unit(value_expr, target_unit, variables)
        },
        
        Expr::DateOffset(day_name, amount, unit) => {
            calculate_date_offset(day_name, *amount, unit)
        },
        
        Expr::Error(msg) => Value::Error(msg.clone()),
    }
}

// Evaluate a binary operation (a + b, a * b, etc.)
fn evaluate_binary_op(left: &Expr, op: &Op, right: &Expr, variables: &mut HashMap<String, Value>) -> Value {
    let left_val = evaluate(left, variables);
    let right_val = evaluate(right, variables);
    
    match (left_val, op, right_val) {
        // Number operations
        (Value::Number(a), Op::Add, Value::Number(b)) => Value::Number(a + b),
        (Value::Number(a), Op::Subtract, Value::Number(b)) => Value::Number(a - b),
        (Value::Number(a), Op::Multiply, Value::Number(b)) => Value::Number(a * b),
        
        // Percentage operations
        (Value::Number(a), Op::Multiply, Value::Percentage(p)) => Value::Number(a * (p / 100.0)),
        (Value::Percentage(p), Op::Multiply, Value::Number(a)) => Value::Number((p / 100.0) * a),
        
        // Add support for addition and subtraction with percentages
        (Value::Number(a), Op::Add, Value::Percentage(p)) => Value::Number(a + (a * p / 100.0)),
        (Value::Unit(a, unit), Op::Add, Value::Percentage(p)) => Value::Unit(a + (a * p / 100.0), unit),
        (Value::Number(a), Op::Subtract, Value::Percentage(p)) => Value::Number(a - (a * p / 100.0)),
        (Value::Unit(a, unit), Op::Subtract, Value::Percentage(p)) => Value::Unit(a - (a * p / 100.0), unit),
        
        (Value::Number(a), Op::Divide, Value::Number(b)) => {
            if b == 0.0 {
                Value::Error("Division by zero".to_string())
            } else {
                Value::Number(a / b)
            }
        },
        (Value::Number(a), Op::Modulo, Value::Number(b)) => {
            if b == 0.0 {
                Value::Error("Modulo by zero".to_string())
            } else {
                Value::Number(a % b)
            }
        },
        (Value::Number(a), Op::Power, Value::Number(b)) => Value::Number(a.powf(b)),
        
        // Unit operations - same units
        (Value::Unit(a, unit_a), Op::Add, Value::Unit(b, unit_b)) if unit_a == unit_b => 
            Value::Unit(a + b, unit_a),
        (Value::Unit(a, unit_a), Op::Subtract, Value::Unit(b, unit_b)) if unit_a == unit_b => 
            Value::Unit(a - b, unit_a),
            
        // Unit with number operations
        (Value::Unit(a, unit), Op::Multiply, Value::Number(b)) => {
            // For unit values (like CAD, USD, etc.), always preserve the unit
            Value::Unit(a * b, unit)
        },
        (Value::Unit(a, unit), Op::Divide, Value::Number(b)) => {
            if b == 0.0 {
                Value::Error("Division by zero".to_string())
            } else {
                Value::Unit(a / b, unit)
            }
        },
        
        // Number with unit operations (new cases)
        (Value::Number(a), Op::Add, Value::Unit(b, unit)) => Value::Unit(a + b, unit),
        (Value::Number(a), Op::Subtract, Value::Unit(b, unit)) => Value::Unit(a - b, unit),
        (Value::Number(a), Op::Multiply, Value::Unit(b, unit)) => Value::Unit(a * b, unit),
        
        // Unit operations with different units - auto-convert for currencies
        (Value::Unit(a, unit_a), op @ (Op::Add | Op::Subtract), Value::Unit(b, unit_b)) => {
            // Normalize both units
            let normalized_unit_a = normalize_unit(&unit_a);
            let normalized_unit_b = normalize_unit(&unit_b);
            
            // Check if the normalized units are the same
            if normalized_unit_a == normalized_unit_b {
                // If they're the same after normalization, directly perform the operation
                match op {
                    Op::Add => Value::Unit(a + b, unit_a),
                    Op::Subtract => Value::Unit(a - b, unit_a),
                    _ => unreachable!(),
                }
            } else {
                // Check if both are currencies
                let is_unit_a_currency = is_currency_code(&normalized_unit_a);
                let is_unit_b_currency = is_currency_code(&normalized_unit_b);
                
                if is_unit_a_currency && is_unit_b_currency {
                    // For currencies, always convert to the first currency
                    if let Some(converted_b) = convert_units(b, &normalized_unit_b, &normalized_unit_a) {
                        match op {
                            Op::Add => Value::Unit(a + converted_b, unit_a),
                            Op::Subtract => Value::Unit(a - converted_b, unit_a),
                            _ => unreachable!(),
                        }
                    } else {
                        Value::Error(format!("Cannot convert from {} to {}", unit_b, unit_a))
                    }
                } else if let Some(converted_b) = convert_units(b, &normalized_unit_b, &normalized_unit_a) {
                    // For regular units, try to convert if possible
                    match op {
                        Op::Add => Value::Unit(a + converted_b, unit_a),
                        Op::Subtract => Value::Unit(a - converted_b, unit_a),
                        _ => unreachable!(),
                    }
                } else {
                    Value::Error(format!("Cannot perform {:?} on {} and {}", op, unit_a, unit_b))
                }
            }
        },
        
        // Handle date operations
        (Value::Date(date), Op::Add, Value::Number(days)) => 
            Value::Date(date + Duration::days(days as i64)),
        (Value::Date(date), Op::Subtract, Value::Number(days)) => 
            Value::Date(date - Duration::days(days as i64)),
            
        // Error for incompatible types
        (a, op, b) => Value::Error(format!("Cannot perform {:?} on {:?} and {:?}", op, a, b)),
    }
}

// Evaluate percentage expression (X% of Y)
fn evaluate_percent_of(percent_expr: &Expr, value_expr: &Expr, variables: &mut HashMap<String, Value>) -> Value {
    let percent_val = evaluate(percent_expr, variables);
    let value_val = evaluate(value_expr, variables);
    
    match (percent_val, value_val) {
        (Value::Number(p), Value::Number(v)) => {
            Value::Number((p / 100.0) * v)
        },
        (Value::Percentage(p), Value::Number(v)) => {
            Value::Number((p / 100.0) * v)
        },
        (Value::Number(p), Value::Unit(v, unit)) => {
            Value::Unit((p / 100.0) * v, unit)
        },
        (Value::Percentage(p), Value::Unit(v, unit)) => {
            Value::Unit((p / 100.0) * v, unit)
        },
        _ => Value::Error("Invalid percentage calculation".to_string()),
    }
}

// Convert a value from one unit to another
fn convert_unit(value_expr: &Expr, target_unit: &str, variables: &mut HashMap<String, Value>) -> Value {
    let value = evaluate(value_expr, variables);
    
    // Normalize the target unit
    let normalized_target_unit = normalize_unit(target_unit);
    
    // Prepare the display unit for output
    let display_unit = if ["KB", "MB", "GB", "TB", "PB", "B"].contains(&normalized_target_unit.as_str()) {
        normalized_target_unit.clone()
    } else if target_unit.chars().all(|c| c.is_uppercase()) {
        target_unit.to_string()
    } else {
        normalized_target_unit.clone()
    };
    
    match value {
        Value::Unit(v, source_unit) => {
            // Normalize the source unit
            let normalized_source_unit = normalize_unit(&source_unit);
            
            // If units are the same after normalization, no conversion needed
            if normalized_source_unit == normalized_target_unit {
                return Value::Unit(v, display_unit);
            }
            
            // Attempt conversion
            match convert_units(v, &normalized_source_unit, &normalized_target_unit) {
                Some(converted_value) => Value::Unit(converted_value, display_unit),
                None => Value::Error(format!("Cannot convert from {} to {}", source_unit, target_unit))
            }
        },
        Value::Number(v) => {
            // For unitless numbers, just apply the target unit
            Value::Unit(v, display_unit)
        },
        _ => Value::Error(format!("Cannot convert value to {}. Try assigning the unit first with 'variable * 1 {}'", target_unit, target_unit)),
    }
}

// Calculate date from expressions like "next friday + 2 weeks"
fn calculate_date_offset(day_name: &str, amount: i64, unit: &str) -> Value {
    // Start with today's date
    let today = Local::now().date_naive();
    
    // Find the next occurrence of the specified day
    let day_of_week = match day_name {
        "monday" => Weekday::Mon,
        "tuesday" => Weekday::Tue,
        "wednesday" => Weekday::Wed,
        "thursday" => Weekday::Thu,
        "friday" => Weekday::Fri,
        "saturday" => Weekday::Sat,
        "sunday" => Weekday::Sun,
        _ => return Value::Error(format!("Unknown day: {}", day_name)),
    };
    
    // Calculate days until next occurrence
    let today_weekday = today.weekday();
    let days_until = (day_of_week.num_days_from_monday() + 7 - today_weekday.num_days_from_monday()) % 7;
    
    // If it's the same day and days_until is 0, we want the next week
    let days_until = if days_until == 0 { 7 } else { days_until };
    
    // Calculate the next occurrence of the day
    let next_day = today + Duration::days(days_until as i64);
    
    // Add the specified offset
    let result_date = match unit {
        "days" | "day" => next_day + Duration::days(amount),
        "weeks" | "week" => next_day + Duration::days(amount * 7),
        "months" | "month" => {
            // Approximate month as 30 days
            next_day + Duration::days(amount * 30)
        },
        _ => return Value::Error(format!("Unknown time unit: {}", unit)),
    };
    
    Value::Date(result_date)
}

// Function to check if a string is a valid currency code
fn is_currency_code(unit: &str) -> bool {
    unit.len() == 3 && unit.chars().all(|c| c.is_ascii_uppercase())
}

// Convert between different units
fn convert_units(value: f64, from_unit: &str, to_unit: &str) -> Option<f64> {
    // Special case for unit identity (same unit)
    if from_unit == to_unit {
        return Some(value);
    }
    
    // Normalize units to handle aliases
    let from_unit = normalize_unit(from_unit);
    let to_unit = normalize_unit(to_unit);
    
    // Check again after normalization
    if from_unit == to_unit {
        return Some(value);
    }
    
    // Check if both units are currencies (uppercase 3-letter codes like USD, EUR, etc.)
    let is_from_currency = is_currency_code(&from_unit);
    let is_to_currency = is_currency_code(&to_unit);
    
    if is_from_currency && is_to_currency {
        // Use currency API for currency conversions
        if let Some(rate) = crate::currency::get_exchange_rate(&from_unit, &to_unit) {
            return Some(value * rate);
        }
        return None;
    }
    
    // For non-currency conversions, use the lookup table
    match (from_unit.as_str(), to_unit.as_str()) {
        // Data units conversions
        ("B", "bit") => Some(value * 8.0),
        ("bit", "B") => Some(value / 8.0),
        
        // Time conversions
        ("s", "min") => Some(value / 60.0),
        ("min", "s") => Some(value * 60.0),
        ("min", "h") => Some(value / 60.0),
        ("h", "min") => Some(value * 60.0),
        ("h", "s") => Some(value * 3600.0),
        ("s", "h") => Some(value / 3600.0),
        ("day", "h") => Some(value * 24.0),
        ("h", "day") => Some(value / 24.0),
        ("day", "s") => Some(value * 86400.0),
        ("s", "day") => Some(value / 86400.0),
        ("week", "day") => Some(value * 7.0),
        ("day", "week") => Some(value / 7.0),
        ("month", "day") => Some(value * 30.44), // average month length
        ("day", "month") => Some(value / 30.44),
        ("year", "day") => Some(value * 365.25), // average year length
        ("day", "year") => Some(value / 365.25),
        ("year", "month") => Some(value * 12.0),
        ("month", "year") => Some(value / 12.0),
        ("decade", "year") => Some(value * 10.0),
        ("year", "decade") => Some(value / 10.0),
        ("century", "year") => Some(value * 100.0),
        ("year", "century") => Some(value / 100.0),
        
        // Time conversions for milliseconds, microseconds, nanoseconds
        ("ms", "s") => Some(value / 1000.0),
        ("s", "ms") => Some(value * 1000.0),
        ("us", "ms") => Some(value / 1000.0),
        ("ms", "us") => Some(value * 1000.0),
        ("ns", "us") => Some(value / 1000.0),
        ("us", "ns") => Some(value * 1000.0),
        
        // Length conversions
        ("cm", "m") => Some(value / 100.0),
        ("m", "cm") => Some(value * 100.0),
        ("cm", "mm") => Some(value * 10.0),
        ("mm", "cm") => Some(value / 10.0),
        ("in", "cm") => Some(value * 2.54),
        ("cm", "in") => Some(value / 2.54),
        ("ft", "m") => Some(value * 0.3048),
        ("m", "ft") => Some(value / 0.3048),
        ("mm", "m") => Some(value / 1000.0),
        ("m", "mm") => Some(value * 1000.0),
        ("km", "m") => Some(value * 1000.0),
        ("m", "km") => Some(value / 1000.0),
        ("mi", "km") => Some(value * 1.60934),
        ("km", "mi") => Some(value / 1.60934),
        ("mi", "m") => Some(value * 1609.34),
        ("m", "mi") => Some(value / 1609.34),
        ("in", "mm") => Some(value * 25.4),
        ("mm", "in") => Some(value / 25.4),
        ("ft", "in") => Some(value * 12.0),
        ("in", "ft") => Some(value / 12.0),
        ("yd", "ft") => Some(value * 3.0),
        ("ft", "yd") => Some(value / 3.0),
        ("yd", "m") => Some(value * 0.9144),
        ("m", "yd") => Some(value / 0.9144),
        
        // Area conversions
        ("m2", "cm2") => Some(value * 10000.0),
        ("cm2", "m2") => Some(value / 10000.0),
        ("km2", "m2") => Some(value * 1000000.0),
        ("m2", "km2") => Some(value / 1000000.0),
        ("ha", "m2") => Some(value * 10000.0),
        ("m2", "ha") => Some(value / 10000.0),
        ("acre", "m2") => Some(value * 4046.86),
        ("m2", "acre") => Some(value / 4046.86),
        ("acre", "ha") => Some(value * 0.404686),
        ("ha", "acre") => Some(value / 0.404686),
        ("mi2", "km2") => Some(value * 2.58999),
        ("km2", "mi2") => Some(value / 2.58999),
        
        // Volume conversions
        ("ml", "l") => Some(value / 1000.0),
        ("l", "ml") => Some(value * 1000.0),
        ("ml", "tsp") => Some(value * 0.2),
        ("tsp", "ml") => Some(value / 0.2),
        ("ml", "tbsp") => Some(value / 15.0),
        ("tbsp", "ml") => Some(value * 15.0),
        ("ml", "teasp") => Some(value * 0.2),  // Alias for tea spoons
        ("teasp", "ml") => Some(value / 0.2),
        ("l", "gal") => Some(value * 0.264172),
        ("gal", "l") => Some(value / 0.264172),
        ("cup", "ml") => Some(value * 236.588),
        ("ml", "cup") => Some(value / 236.588),
        ("pt", "ml") => Some(value * 473.176),
        ("ml", "pt") => Some(value / 473.176),
        ("qt", "ml") => Some(value * 946.353),
        ("ml", "qt") => Some(value / 946.353),
        ("floz", "ml") => Some(value * 29.5735),
        ("ml", "floz") => Some(value / 29.5735),
        ("cup", "floz") => Some(value * 8.0),
        ("floz", "cup") => Some(value / 8.0),
        ("m3", "l") => Some(value * 1000.0),
        ("l", "m3") => Some(value / 1000.0),
        ("ft3", "m3") => Some(value * 0.0283168),
        ("m3", "ft3") => Some(value / 0.0283168),
        
        // Weight conversions
        ("g", "kg") => Some(value / 1000.0),
        ("kg", "g") => Some(value * 1000.0),
        ("lb", "kg") => Some(value * 0.453592),
        ("kg", "lb") => Some(value / 0.453592),
        ("oz", "g") => Some(value * 28.3495),
        ("g", "oz") => Some(value / 28.3495),
        ("mg", "g") => Some(value / 1000.0),
        ("g", "mg") => Some(value * 1000.0),
        ("kg", "ton") => Some(value / 1000.0),
        ("ton", "kg") => Some(value * 1000.0),
        ("lb", "oz") => Some(value * 16.0),
        ("oz", "lb") => Some(value / 16.0),
        ("st", "lb") => Some(value * 14.0),
        ("lb", "st") => Some(value / 14.0),
        ("st", "kg") => Some(value * 6.35029),
        ("kg", "st") => Some(value / 6.35029),
        
        // Temperature conversions
        ("C", "F") => Some(value * 9.0/5.0 + 32.0),
        ("F", "C") => Some((value - 32.0) * 5.0/9.0),
        ("K", "C") => Some(value - 273.15),
        ("C", "K") => Some(value + 273.15),
        ("F", "K") => Some((value + 459.67) * 5.0/9.0),
        ("K", "F") => Some(value * 9.0/5.0 - 459.67),
        
        // Data storage conversions
        ("B", "KB") => Some(value / 1024.0),
        ("KB", "B") => Some(value * 1024.0),
        ("KB", "MB") => Some(value / 1024.0),
        ("MB", "KB") => Some(value * 1024.0),
        ("MB", "GB") => Some(value / 1024.0),
        ("GB", "MB") => Some(value * 1024.0),
        ("GB", "TB") => Some(value / 1024.0),
        ("TB", "GB") => Some(value * 1024.0),
        ("TB", "PB") => Some(value / 1024.0),
        ("PB", "TB") => Some(value * 1024.0),
        
        // Energy conversions
        ("J", "kJ") => Some(value / 1000.0),
        ("kJ", "J") => Some(value * 1000.0),
        ("cal", "J") => Some(value * 4.184),
        ("J", "cal") => Some(value / 4.184),
        ("kcal", "cal") => Some(value * 1000.0),
        ("cal", "kcal") => Some(value / 1000.0),
        ("kWh", "J") => Some(value * 3600000.0),
        ("J", "kWh") => Some(value / 3600000.0),
        ("eV", "J") => Some(value * 1.602176634e-19),
        ("J", "eV") => Some(value / 1.602176634e-19),
        
        // Power conversions
        ("W", "kW") => Some(value / 1000.0),
        ("kW", "W") => Some(value * 1000.0),
        ("MW", "kW") => Some(value * 1000.0),
        ("kW", "MW") => Some(value / 1000.0),
        ("hp", "W") => Some(value * 745.7),
        ("W", "hp") => Some(value / 745.7),
        ("hp", "kW") => Some(value * 0.7457),
        ("kW", "hp") => Some(value / 0.7457),
        
        // Pressure conversions
        ("Pa", "kPa") => Some(value / 1000.0),
        ("kPa", "Pa") => Some(value * 1000.0),
        ("bar", "kPa") => Some(value * 100.0),
        ("kPa", "bar") => Some(value / 100.0),
        ("psi", "kPa") => Some(value * 6.895),
        ("kPa", "psi") => Some(value / 6.895),
        ("atm", "kPa") => Some(value * 101.325),
        ("kPa", "atm") => Some(value / 101.325),
        
        // Speed conversions
        ("mps", "kmph") => Some(value * 3.6),  // meters per second to km per hour
        ("kmph", "mps") => Some(value / 3.6),
        ("mph", "kmph") => Some(value * 1.60934),
        ("kmph", "mph") => Some(value / 1.60934),
        ("mph", "mps") => Some(value * 0.44704),
        ("mps", "mph") => Some(value / 0.44704),
        ("knot", "kmph") => Some(value * 1.852),
        ("kmph", "knot") => Some(value / 1.852),
        
        // Same unit, no conversion needed
        (a, b) if a == b => Some(value),
        
        // Unknown conversion
        _ => None,
    }
}

// Function to normalize unit strings - convert aliases to canonical forms
fn normalize_unit(unit: &str) -> String {
    use once_cell::sync::Lazy;
    use std::collections::HashMap;

    // Single, consolidated mapping of unit aliases to canonical forms
    static UNIT_MAP: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
        let mut map = HashMap::new();
        
        // Special cases that need exact case preservation
        map.insert("bit", "bit");
        map.insert("s", "s");
        map.insert("min", "min");
        map.insert("h", "h");
        map.insert("day", "day");
        map.insert("week", "week");
        map.insert("month", "month");
        map.insert("year", "year");
        map.insert("ms", "ms");
        map.insert("us", "us");
        map.insert("ns", "ns");
        map.insert("b", "B");

        // Data units that need uppercase
        map.insert("kb", "KB");
        map.insert("mb", "MB");
        map.insert("gb", "GB");
        map.insert("tb", "TB");
        map.insert("pb", "PB");
        
        // Temperature units are uppercase
        map.insert("c", "C");
        map.insert("f", "F");
        map.insert("k", "K");
        
        // Data units
        map.insert("bytes", "B");
        map.insert("kilobytes", "KB");
        map.insert("megabytes", "MB");
        map.insert("gigabytes", "GB");
        map.insert("terabytes", "TB");
        map.insert("petabytes", "PB");
        map.insert("bits", "bit");
        
        // Currencies
        map.insert("eur", "EUR");
        map.insert("usd", "USD");
        map.insert("gbp", "GBP");
        map.insert("cad", "CAD");
        map.insert("jpy", "JPY");
        map.insert("aud", "AUD");
        map.insert("cny", "CNY");
        map.insert("inr", "INR");
        
        // Time units
        map.insert("minute", "min");
        map.insert("minutes", "min");
        map.insert("mins", "min");
        map.insert("m", "min");
        map.insert("second", "s");
        map.insert("seconds", "s");
        map.insert("sec", "s");
        map.insert("secs", "s");
        map.insert("hour", "h");
        map.insert("hours", "h");
        map.insert("hr", "h");
        map.insert("hrs", "h");
        map.insert("millisecond", "ms");
        map.insert("milliseconds", "ms");
        map.insert("msec", "ms");
        map.insert("msecs", "ms");
        map.insert("microsecond", "us");
        map.insert("microseconds", "us");
        map.insert("usec", "us");
        map.insert("usecs", "us");
        map.insert("nanosecond", "ns");
        map.insert("nanoseconds", "ns");
        map.insert("nsec", "ns");
        map.insert("nsecs", "ns");
        map.insert("days", "day");
        map.insert("weeks", "week");
        map.insert("months", "month");
        map.insert("years", "year");
        
        // Length units
        map.insert("meters", "m");
        map.insert("metre", "m");
        map.insert("metres", "m");
        map.insert("centimeters", "cm");
        map.insert("centimetre", "cm");
        map.insert("centimetres", "cm");
        map.insert("millimeters", "mm");
        map.insert("millimetre", "mm");
        map.insert("millimetres", "mm");
        map.insert("kilometers", "km");
        map.insert("kilometre", "km");
        map.insert("kilometres", "km");
        map.insert("inches", "in");
        map.insert("feet", "ft");
        map.insert("foot", "ft");
        map.insert("yards", "yd");
        map.insert("miles", "mi");
        
        // Weight units
        map.insert("grams", "g");
        map.insert("kilograms", "kg");
        map.insert("kgs", "kg");
        map.insert("kilos", "kg");
        map.insert("milligrams", "mg");
        map.insert("pounds", "lb");
        map.insert("lbs", "lb");
        map.insert("ounces", "oz");
        map.insert("tons", "ton");
        map.insert("tonnes", "ton");
        map.insert("stones", "st");
        
        // Volume units
        map.insert("milliliters", "ml");
        map.insert("millilitres", "ml");
        map.insert("liters", "l");
        map.insert("litres", "l");
        map.insert("teaspoons", "tsp");
        map.insert("tablespoons", "tbsp");
        map.insert("cups", "cup");
        map.insert("pints", "pt");
        map.insert("quarts", "qt");
        map.insert("gallons", "gal");
        map.insert("fluid ounces", "floz");
        map.insert("fluidounces", "floz");
        
        // Temperature units
        map.insert("celsius", "C");
        map.insert("centigrade", "C");
        map.insert("fahrenheit", "F");
        map.insert("kelvin", "K");
        
        // Energy units
        map.insert("joules", "J");
        map.insert("kilojoules", "kJ");
        map.insert("calories", "cal");
        map.insert("kilocalories", "kcal");
        map.insert("kcals", "kcal");
        map.insert("kilowatt hours", "kWh");
        map.insert("kilowatt-hours", "kWh");
        map.insert("electron volts", "eV");
        
        // Power units
        map.insert("watts", "W");
        map.insert("kilowatts", "kW");
        map.insert("megawatts", "MW");
        map.insert("horsepower", "hp");
        
        // Pressure units
        map.insert("pascals", "Pa");
        map.insert("kilopascals", "kPa");
        map.insert("bars", "bar");
        map.insert("pounds per square inch", "psi");
        map.insert("atmospheres", "atm");
        
        // Speed units
        map.insert("meters per second", "mps");
        map.insert("metres per second", "mps");
        map.insert("kilometers per hour", "kmph");
        map.insert("kilometres per hour", "kmph");
        map.insert("kph", "kmph");
        map.insert("miles per hour", "mph");
        map.insert("knots", "knot");
        
        map
    });

    let original = unit.trim();
    let lowercase = original.to_lowercase();
    
    // First try the map lookup which includes all special cases
    if let Some(canonical) = UNIT_MAP.get(lowercase.as_str()) {
        return (*canonical).to_string();
    }
    
    // Special case for currency detection (3-letter uppercase codes)
    if lowercase.len() == 3 && lowercase.chars().all(|c| c.is_ascii_alphabetic()) {
        return lowercase.to_uppercase();
    }
    
    // If no match, return the original lowercase
    lowercase
}

// Evaluate a list of expressions and return formatted results
#[allow(dead_code)]
pub fn evaluate_lines(lines: &[String], variables: &mut HashMap<String, Value>) -> Vec<String> {
    lines.iter()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                String::new()
            } else if trimmed.starts_with('#') {
                // Return an empty string for comment lines
                String::new()
            } else {
                let expr = crate::parser::parse_line(line, variables);
                let result = evaluate(&expr, variables);
                if let Value::Assignment(name, value) = &result {
                    // Store the variable for future use
                    variables.insert(name.clone(), (**value).clone());
                }
                format!("{}", result)
            }
        })
        .collect()
}