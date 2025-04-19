use std::collections::HashMap;
use crate::evaluator::{evaluate, Value};
use crate::parser::{parse_line, Expr, Op};

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_unit_preservation() {
        let mut variables = HashMap::new();
        
        // Store x = 10 CAD
        variables.insert("x".to_string(), Value::Unit(10.0, "CAD".to_string()));
        
        // Now test x * 1.13 directly
        let expr = Expr::BinaryOp(
            Box::new(Expr::Variable("x".to_string())),
            Op::Multiply,
            Box::new(Expr::Number(1.13))
        );
        
        let result = evaluate(&expr, &mut variables);
        println!("x * 1.13 = {:?}", result);
        
        // Make sure it's Value::Unit(11.3, "CAD")
        match result {
            Value::Unit(value, unit) => {
                assert_eq!(unit, "CAD");
                assert!((value - 11.3).abs() < 0.001);
            },
            _ => panic!("Expected Unit value, got {:?}", result),
        }
    }
    
    // Evaluator tests
    #[test]
    fn test_evaluate_number() {
        let mut variables = HashMap::new();
        let expr = parse_line("42", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Number(n) => assert_eq!(n, 42.0),
            _ => panic!("Expected Number value"),
        }
    }
    
    #[test]
    fn test_evaluate_unit_value() {
        let mut variables = HashMap::new();
        let expr = parse_line("10 USD", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert_eq!(v, 10.0);
                assert_eq!(u, "USD");
            },
            _ => panic!("Expected Unit value"),
        }
    }
    
    #[test]
    fn test_evaluate_binary_op() {
        let mut variables = HashMap::new();
        
        // Addition
        let expr = parse_line("5 + 3", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Number(n) => assert_eq!(n, 8.0),
            _ => panic!("Expected Number value for addition"),
        }
        
        // Multiplication
        let expr = parse_line("4 * 3", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Number(n) => assert_eq!(n, 12.0),
            _ => panic!("Expected Number value for multiplication"),
        }
        
        // Division
        let expr = parse_line("10 / 2", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Number(n) => assert_eq!(n, 5.0),
            _ => panic!("Expected Number value for division"),
        }
    }
    
    #[test]
    fn test_evaluate_assignment() {
        let mut variables = HashMap::new();
        
        // Assign a value
        let expr = parse_line("x = 42", &variables);
        let result = evaluate(&expr, &mut variables);
        
        // Manual storage for the test
        if let Value::Assignment(name, value) = result {
            variables.insert(name, (*value).clone());
        }
        
        // Check if the variable was stored
        assert!(variables.contains_key("x"));
        match variables.get("x") {
            Some(Value::Number(n)) => assert_eq!(*n, 42.0),
            _ => panic!("Expected Number value for variable"),
        }
        
        // Use the variable in an expression
        let expr = parse_line("x + 8", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Number(n) => assert_eq!(n, 50.0),
            _ => panic!("Expected Number value for expression with variable"),
        }
    }
    
    #[test]
    fn test_evaluate_unit_conversion() {
        let mut variables = HashMap::new();
        
        // Convert ml to l
        let expr = parse_line("10 ml in l", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert_eq!(v, 0.01); // 10 ml = 0.01 l
                assert_eq!(u, "l");
            },
            _ => panic!("Expected Unit value for conversion"),
        }
        
        // Convert cm to in
        let expr = parse_line("10 cm in in", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert!((v - 3.937).abs() < 0.001); // 10 cm ≈ 3.937 in
                assert_eq!(u, "in");
            },
            _ => panic!("Expected Unit value for conversion"),
        }
    }
    
    #[test]
    fn test_evaluate_percentage() {
        let mut variables = HashMap::new();
        
        // Simple percentage
        let expr = parse_line("20% of 50", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Number(n) => assert_eq!(n, 10.0), // 20% of 50 = 10
            _ => panic!("Expected Number value for percentage"),
        }
        
        // Percentage of a unit value
        let expr = parse_line("20% of 50 USD", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert_eq!(v, 10.0); // 20% of 50 USD = 10 USD
                assert_eq!(u, "USD");
            },
            _ => panic!("Expected Unit value for percentage of unit"),
        }
    }
    
    #[test]
    fn test_evaluate_lines() {
        let mut variables = HashMap::new();
        let lines = vec![
            "price = 10 USD".to_string(),
            "discount = 2 USD".to_string(),
            "total = price + discount".to_string(),
        ];
        
        let results = crate::evaluator::evaluate_lines(&lines, &mut variables);
        
        // Check that the variables were stored
        assert!(variables.contains_key("price"));
        assert!(variables.contains_key("discount"));
        assert!(variables.contains_key("total"));
        
        // Check the results formatting
        assert_eq!(results[0], "$10");
        assert_eq!(results[1], "$2");
        
        // The total should be price + discount = 10 + 2 = 12 USD
        match variables.get("total") {
            Some(Value::Unit(v, u)) => {
                assert_eq!(*v, 12.0);
                assert_eq!(*u, "USD");
            },
            _ => panic!("Expected Unit value for total"),
        }
    }
    
    #[test]
    fn test_currency_conversion() {
        let mut variables = HashMap::new();
        
        // Test USD to CAD conversion
        let expr = parse_line("10 USD in CAD", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                // We can't check the exact value since it depends on the API response
                // Just make sure it's positive and the unit is correct
                assert!(v > 0.0);
                assert_eq!(u, "CAD");
            },
            _ => panic!("Expected Unit value for currency conversion"),
        }
        
        // Test CAD to EUR conversion
        let expr = parse_line("20 CAD in EUR", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                // We can't check the exact value since it depends on the API response
                // Just make sure it's positive and the unit is correct
                assert!(v > 0.0);
                assert_eq!(u, "EUR");
            },
            _ => panic!("Expected Unit value for currency conversion"),
        }
    }
    
    #[test]
    fn test_set_exchange_rate() {
        let mut variables = HashMap::new();
        
        // First check the current rate from USD to GBP
        let expr = parse_line("10 USD in GBP", &variables);
        let _original_rate = match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert_eq!(u, "GBP");
                v / 10.0 // Extract the actual rate
            },
            _ => panic!("Expected Unit value for currency conversion"),
        };
        
        // Set a new custom rate
        let expr = parse_line("setrate USD to GBP = 0.65", &variables);
        evaluate(&expr, &mut variables);
        
        // Verify the new rate is used
        let expr = parse_line("10 USD in GBP", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert_eq!(u, "GBP");
                assert!((v / 10.0 - 0.65).abs() < 0.001);
            },
            _ => panic!("Expected Unit value for currency conversion"),
        }
        
        // Check the reverse direction works too
        let expr = parse_line("20 GBP in USD", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert_eq!(u, "USD");
                // Should be approximately 20 / 0.65 = 30.77
                assert!((v - 30.77).abs() < 0.1);
            },
            _ => panic!("Expected Unit value for currency conversion"),
        }
    }
    
    #[test]
    fn test_evaluate_percentage_variable() {
        let mut variables = HashMap::new();
        
        // First assign x = 10
        let expr = parse_line("x = 10", &variables);
        let result = evaluate(&expr, &mut variables);
        if let Value::Assignment(name, value) = result {
            variables.insert(name, (*value).clone());
        }
        
        // Then assign tax = 13%
        let expr = parse_line("tax = 13%", &variables);
        let result = evaluate(&expr, &mut variables);
        if let Value::Assignment(name, value) = result {
            variables.insert(name, (*value).clone());
        }
        
        // Now evaluate x * tax
        let expr = parse_line("x * tax", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Number(n) => {
                assert_eq!(n, 1.3); // 13% of 10 = 1.3
            },
            _ => panic!("Expected Number value for x * tax"),
        }
    }
    
    #[test]
    fn test_currency_unit_multiplication() {
        let mut variables = HashMap::new();
        
        // First convert currency
        let expr = parse_line("10 USD in CAD", &variables);
        let result = evaluate(&expr, &mut variables);
        
        match result {
            Value::Unit(value, unit) => {
                assert_eq!(unit, "CAD");
                assert!(value > 0.0);
                
                // Now try with explicit unit value
                let expr = parse_line(format!("{} CAD * 1.13", value).as_str(), &variables);
                match evaluate(&expr, &mut variables) {
                    Value::Unit(n, unit) => {
                        assert_eq!(unit, "CAD");
                        assert!(n > value); // Should be larger
                    },
                    other => panic!("Expected Unit result, got {:?}", other),
                }
            },
            other => panic!("Expected Unit result for conversion, got {:?}", other),
        }
    }
    
    #[test]
    fn test_variable_unit_preservation() {
        let mut variables = HashMap::new();
        
        // Assign x = 10 USD
        let expr = parse_line("x = 10 USD", &variables);
        let result = evaluate(&expr, &mut variables);
        if let Value::Assignment(name, value) = result {
            variables.insert(name, (*value).clone());
        }
        
        // Verify x contains the unit value
        match variables.get("x").cloned() {
            Some(Value::Unit(value, unit)) => {
                assert_eq!(value, 10.0);
                assert_eq!(unit, "USD");
            },
            other => panic!("Expected Unit value in variable x, got {:?}", other),
        }
        
        // Convert x to CAD
        let expr = parse_line("y = x to CAD", &variables);
        let result = evaluate(&expr, &mut variables);
        if let Value::Assignment(name, value) = result {
            variables.insert(name, (*value).clone());
        }
        
        // Get the CAD value before proceeding
        let y_value: f64;
        let _y_unit: String;
        
        match variables.get("y").cloned() {
            Some(Value::Unit(value, unit)) => {
                assert_eq!(unit, "CAD");
                assert!(value > 10.0); // Should be more CAD than USD
                
                y_value = value;
                _y_unit = unit;
                
                // Now calculate y * 1.13
                let expr = parse_line("total = y * 1.13", &variables);
                let result = evaluate(&expr, &mut variables);
                if let Value::Assignment(name, value) = result {
                    variables.insert(name, (*value).clone());
                }
            },
            other => panic!("Expected Unit value in variable y, got {:?}", other),
        }
        
        // Verify total has the CAD unit and correct value
        match variables.get("total").cloned() {
            Some(Value::Unit(total_value, total_unit)) => {
                assert_eq!(total_unit, "CAD");
                assert!(total_value > y_value);
            },
            other => panic!("Expected Unit value in variable total, got {:?}", other),
        }
    }

    #[test]
    fn test_evaluate_with_comments() {
        let mut variables = HashMap::new();
        let lines = vec![
            "# This is a comment".to_string(),
            "price = 10 USD # Setting the price".to_string(),
            "# Another comment line".to_string(),
            "tax = 5%".to_string(),
            "# Calculate total".to_string(),
            "total = price * 1.05".to_string(),  // Simplified expression instead of price * (1 + tax)
        ];
        
        let results = crate::evaluator::evaluate_lines(&lines, &mut variables);
        
        // Check the results - comments should have empty results
        assert_eq!(results[0], "");  // Comment line
        assert_eq!(results[1], "$10");  // Price assignment (comment at end is part of the line)
        assert_eq!(results[2], "");  // Comment line
        assert!(results[3].contains("5%")); // Tax assignment
        assert_eq!(results[4], "");  // Comment line
        
        // Verify total value is calculated correctly (price * 1.05 = 10 * 1.05 = 10.5 USD)
        match variables.get("total") {
            Some(val) => {
                match val {
                    Value::Unit(v, u) => {
                        assert_eq!(*u, "USD");
                        assert!((v - 10.5).abs() < 0.01, "Expected 10.5 USD, got {} USD", v);
                    },
                    _ => panic!("Expected Unit value for total, got {:?}", val),
                }
            },
            None => panic!("Variable 'total' not found in variables"),
        }
    }
    
    // Time unit conversions
    #[test]
    fn test_time_unit_conversions() {
        let mut variables = HashMap::new();
        
        // Test seconds to minutes
        let expr = parse_line("120 s in min", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert_eq!(v, 2.0); // 120 seconds = 2 minutes
                assert_eq!(u, "min");
            },
            other => panic!("Expected Unit value for s to min conversion, got {:?}", other),
        }
        
        // Test minutes to hours
        let expr = parse_line("90 min in h", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert_eq!(v, 1.5); // 90 minutes = 1.5 hours
                assert_eq!(u, "h");
            },
            other => panic!("Expected Unit value for min to h conversion, got {:?}", other),
        }
        
        // Test days to hours
        let expr = parse_line("2 day in h", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert_eq!(v, 48.0); // 2 days = 48 hours
                assert_eq!(u, "h");
            },
            other => panic!("Expected Unit value for day to h conversion, got {:?}", other),
        }
        
        // Test milliseconds to seconds
        let expr = parse_line("5000 ms in s", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert_eq!(v, 5.0); // 5000 ms = 5 seconds
                assert_eq!(u, "s");
            },
            other => panic!("Expected Unit value for ms to s conversion, got {:?}", other),
        }
    }
    
    #[test]
    fn test_data_unit_conversions() {
        let mut variables = HashMap::new();
        
        // Test KB to MB conversion
        let expr = parse_line("2048 KB in MB", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert_eq!(v, 2.0); // 2048 KB = 2 MB
                assert_eq!(u, "MB");
            },
            other => panic!("Expected Unit value for KB to MB conversion, got {:?}", other),
        }
        
        // Test bytes to bits
        let expr = parse_line("16 B in bit", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert_eq!(v, 128.0); // 16 bytes = 128 bits
                assert_eq!(u, "bit");
            },
            other => panic!("Expected Unit value for B to bit conversion, got {:?}", other),
        }
        
        // Test GB to TB
        let expr = parse_line("2048 GB in TB", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert_eq!(v, 2.0); // 2048 GB = 2 TB
                assert_eq!(u, "TB");
            },
            other => panic!("Expected Unit value for GB to TB conversion, got {:?}", other),
        }
    }
    
    #[test]
    fn test_area_and_volume_conversions() {
        let mut variables = HashMap::new();
        
        // Test square meters to square centimeters using m2/cm2 notation
        let expr = parse_line("2 m2 in cm2", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert_eq!(v, 20000.0); // 2 m² = 20,000 cm²
                assert_eq!(u, "cm2");
            },
            other => panic!("Expected Unit value for m2 to cm2 conversion, got {:?}", other),
        }
        
        // Test hectares to square meters
        let expr = parse_line("0.5 ha in m2", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert_eq!(v, 5000.0); // 0.5 ha = 5,000 m²
                assert_eq!(u, "m2");
            },
            other => panic!("Expected Unit value for ha to m2 conversion, got {:?}", other),
        }
    }
    
    #[test]
    fn test_numeric_variable_to_currency() {
        let mut variables = HashMap::new();
        
        // Create a numeric variable
        let expr = parse_line("z = 7", &variables);
        let result = evaluate(&expr, &mut variables);
        if let Value::Assignment(name, value) = result {
            variables.insert(name, (*value).clone());
        }
        
        // Verify z is a numeric value
        match variables.get("z").cloned() {
            Some(Value::Number(val)) => {
                assert_eq!(val, 7.0);
            },
            other => panic!("Expected Number value in variable z, got {:?}", other),
        }
        
        // Now try to convert z directly to CAD
        let expr = parse_line("z to CAD", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert_eq!(v, 7.0);
                assert_eq!(u, "CAD");
            },
            other => panic!("Expected Unit value for variable conversion, got {:?}", other),
        }
        
        // Try converting z directly to USD and then to EUR
        let expr = parse_line("z USD to EUR", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert!(v > 0.0);
                assert_eq!(u, "EUR");
            },
            other => panic!("Expected Unit value for variable conversion, got {:?}", other),
        }
    }
    
    #[test]
    fn test_unit_aliases() {
        let mut variables = HashMap::new();
        
        // Test minutes aliases
        let expr = parse_line("60 minutes in h", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert_eq!(v, 1.0); // 60 minutes = 1 hour
                assert_eq!(u, "h");
            },
            other => panic!("Expected Unit value for minutes to h conversion, got {:?}", other),
        }
        
        // Test mins alias
        let expr = parse_line("60 mins in h", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert_eq!(v, 1.0); // 60 mins = 1 hour
                assert_eq!(u, "h");
            },
            other => panic!("Expected Unit value for mins to h conversion, got {:?}", other),
        }
        
        // Test plural/singular forms
        let expr = parse_line("1 day in hours", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert_eq!(v, 24.0); // 1 day = 24 hours
                assert_eq!(u, "h");
            },
            other => panic!("Expected Unit value for day to hours conversion, got {:?}", other),
        }
        
        // Test other common aliases - kilograms to pounds
        let expr = parse_line("1 kg in lb", &variables);
        match evaluate(&expr, &mut variables) {
            Value::Unit(v, u) => {
                assert!((v - 2.20462).abs() < 0.001);
                assert_eq!(u, "lb");
            },
            other => panic!("Expected Unit value for kg to lb conversion, got {:?}", other),
        }
    }
    
    #[test]
    fn test_percentage_operations() {
        let mut variables = HashMap::new();
        
        // Test subtracting a percentage
        let subtract_percentage = Expr::BinaryOp(
            Box::new(Expr::Number(100.0)),
            Op::Subtract,
            Box::new(Expr::Percentage(20.0))
        );
        
        let result = evaluate(&subtract_percentage, &mut variables);
        match result {
            Value::Number(val) => {
                assert!((val - 80.0).abs() < 0.01); // 100 - 20% of 100 = 80
            },
            _ => panic!("Expected number value, got {:?}", result),
        }
        
        // Test adding a percentage
        let add_percentage = Expr::BinaryOp(
            Box::new(Expr::Number(100.0)),
            Op::Add,
            Box::new(Expr::Percentage(10.0))
        );
        
        let result = evaluate(&add_percentage, &mut variables);
        match result {
            Value::Number(val) => {
                assert!((val - 110.0).abs() < 0.01); // 100 + 10% of 100 = 110
            },
            _ => panic!("Expected number value, got {:?}", result),
        }
        
        // Test with units
        let subtract_percentage_unit = Expr::BinaryOp(
            Box::new(Expr::UnitValue(50.0, "USD".to_string())),
            Op::Subtract,
            Box::new(Expr::Percentage(5.0))
        );
        
        let result = evaluate(&subtract_percentage_unit, &mut variables);
        match result {
            Value::Unit(val, unit) => {
                assert_eq!(unit, "USD".to_string());
                assert!((val - 47.5).abs() < 0.01); // 50 USD - 5% of 50 USD = 47.5 USD
            },
            _ => panic!("Expected unit value, got {:?}", result),
        }
        
        // Test the specific case from user example: price + fee - 4%
        // Where: price = 10 USD, fee = 4 GBP (with mock exchange rate)
        
        // Setup variables
        variables.insert("price".to_string(), Value::Unit(10.0, "USD".to_string()));
        variables.insert("fee".to_string(), Value::Unit(4.0, "GBP".to_string()));
        
        // Mock the exchange rate for GBP to USD
        crate::currency::set_exchange_rate("GBP", "USD", 1.3); // 1 GBP = 1.3 USD
        
        // Create expression: (price + fee) - 4%
        let complex_expr = Expr::BinaryOp(
            Box::new(Expr::BinaryOp(
                Box::new(Expr::Variable("price".to_string())),
                Op::Add,
                Box::new(Expr::Variable("fee".to_string()))
            )),
            Op::Subtract,
            Box::new(Expr::Percentage(4.0))
        );
        
        let result = evaluate(&complex_expr, &mut variables);
        match result {
            Value::Unit(val, unit) => {
                assert_eq!(unit, "USD".to_string());
                // Expected: (10 USD + (4 GBP * 1.3)) - 4% = (10 + 5.2) * 0.96 = 15.2 * 0.96 = 14.592 USD
                assert!((val - 14.592).abs() < 0.01);
            },
            _ => panic!("Expected unit value, got {:?}", result),
        }
    }
    
    #[test]
    fn test_automatic_currency_conversion() {
        // Mock the exchange rates for testing
        crate::currency::set_exchange_rate("USD", "EUR", 0.85); // 1 USD = 0.85 EUR
        crate::currency::set_exchange_rate("EUR", "USD", 1.18); // 1 EUR = 1.18 USD
        crate::currency::set_exchange_rate("USD", "CAD", 1.25); // 1 USD = 1.25 CAD
        crate::currency::set_exchange_rate("CAD", "USD", 0.8); // 1 CAD = 0.8 USD
        
        let mut variables = HashMap::new();
        
        // Test adding different currencies
        let add_diff_curr = Expr::BinaryOp(
            Box::new(Expr::UnitValue(100.0, "USD".to_string())),
            Op::Add,
            Box::new(Expr::UnitValue(100.0, "EUR".to_string()))
        );
        
        let result = evaluate(&add_diff_curr, &mut variables);
        match result {
            Value::Unit(val, unit) => {
                assert_eq!(unit, "USD".to_string());
                assert!((val - 218.0).abs() < 0.01); // 100 USD + (100 EUR * 1.18) = 218 USD
            },
            _ => panic!("Expected unit value, got {:?}", result),
        }
        
        // Test subtracting different currencies
        let sub_diff_curr = Expr::BinaryOp(
            Box::new(Expr::UnitValue(200.0, "CAD".to_string())),
            Op::Subtract,
            Box::new(Expr::UnitValue(50.0, "USD".to_string()))
        );
        
        let result = evaluate(&sub_diff_curr, &mut variables);
        match result {
            Value::Unit(val, unit) => {
                assert_eq!(unit, "CAD".to_string());
                assert!((val - 137.5).abs() < 0.01); // 200 CAD - (50 USD * 1.25) = 137.5 CAD
            },
            _ => panic!("Expected unit value, got {:?}", result),
        }
    }
} 