use std::collections::HashMap;

pub fn create_map() -> HashMap<String, i32> {
    let mut map = HashMap::new();
    map.insert("key1".to_string(), 10);
    map.insert("key2".to_string(), 20);
    map
}

pub fn format_string(input: &str) -> String {
    format!("Formatted: {}", input.to_uppercase())
}

pub fn fibonacci(n: u32) -> u32 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}