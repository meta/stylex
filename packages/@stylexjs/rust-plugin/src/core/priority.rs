//! Shared rule priorities, including relational and compound pseudo selectors.
use regex::Regex;
use std::sync::OnceLock;

pub fn get_priority(key: &str) -> f64 {
    if key.starts_with("--") {
        return 1.0;
    }
    for (prefix, priority) in [
        ("@supports", 30.0),
        ("@media", 200.0),
        ("@container", 300.0),
    ] {
        if key.starts_with(prefix) {
            return priority;
        }
    }
    static PARTS: OnceLock<Regex> = OnceLock::new();
    let parts: Vec<_> = PARTS
        .get_or_init(|| Regex::new(r"::[a-zA-Z-]+|:[a-zA-Z-]+(?:\([^)]*\))?").unwrap())
        .find_iter(key)
        .map(|m| m.as_str())
        .collect();
    if parts.len() > 1 && !parts.iter().any(|part| part.contains('(')) {
        return parts
            .iter()
            .map(|part| {
                if part.starts_with("::") {
                    5000.0
                } else {
                    super::properties::pseudo_priority(part)
                }
            })
            .sum();
    }
    if key.starts_with("::") {
        return 5000.0;
    }
    static RELATIONAL: OnceLock<Vec<(Regex, f64)>> = OnceLock::new();
    let relational=RELATIONAL.get_or_init(||vec![
        (Regex::new(r"^:where\(\.[0-9a-zA-Z_-]+(:[a-zA-Z-]+)\s+\*\)$").unwrap(),10.0),
        (Regex::new(r"^:where\(:has\(\.[0-9a-zA-Z_-]+(:[a-zA-Z-]+)\)\)$").unwrap(),15.0),
        (Regex::new(r"^:where\(\.[0-9a-zA-Z_-]+(:[a-zA-Z-]+)\s+~\s+\*,\s+:has\(~\s\.[0-9a-zA-Z_-]+(:[a-zA-Z-]+)\)\)$").unwrap(),20.0),
        (Regex::new(r"^:where\(\.[0-9a-zA-Z_-]+(:[a-zA-Z-]+)\s+~\s+\*\)$").unwrap(),30.0),
        (Regex::new(r"^:where\(:has\(~\s\.[0-9a-zA-Z_-]+(:[a-zA-Z-]+)\)\)$").unwrap(),40.0),
    ]);
    for (re, base) in relational {
        if let Some(c) = re.captures(key) {
            let first = super::properties::pseudo_priority(&c[1]);
            let second = c
                .get(2)
                .map(|m| super::properties::pseudo_priority(m.as_str()))
                .unwrap_or(first);
            return base + first.max(second) / 100.0;
        }
    }
    if key.starts_with(':') {
        return super::properties::pseudo_priority(key.split('(').next().unwrap_or(key));
    }
    super::properties::default_priority(key)
}
