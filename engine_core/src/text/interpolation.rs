// engine_core/src/text/interpolation.rs
use std::collections::HashMap;

/// Interpolates `{{variable}}` placeholders in text with provided values.
///
/// # Example
/// ```
/// let mut vars = HashMap::new();
/// vars.insert("name".to_string(), "Hero".to_string());
/// let result = interpolate("Hello {{name}}!", &vars);
/// assert_eq!(result, "Hello Hero!");
/// ```
pub fn interpolate(text: &str, variables: &HashMap<String, String>) -> String {
    let mut result = text.to_string();
    for (key, value) in variables {
        let placeholder = format!("{{{{{}}}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_single_variable() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Hero".to_string());
        assert_eq!(interpolate("Hello {{name}}!", &vars), "Hello Hero!");
    }

    #[test]
    fn test_interpolate_multiple_variables() {
        let mut vars = HashMap::new();
        vars.insert("player".to_string(), "Alice".to_string());
        vars.insert("gold".to_string(), "100".to_string());
        assert_eq!(
            interpolate("{{player}} has {{gold}} gold.", &vars),
            "Alice has 100 gold."
        );
    }

    #[test]
    fn test_interpolate_no_variables() {
        let vars = HashMap::new();
        assert_eq!(
            interpolate("No placeholders here.", &vars),
            "No placeholders here."
        );
    }

    #[test]
    fn test_interpolate_missing_variable() {
        let vars = HashMap::new();
        assert_eq!(interpolate("Hello {{name}}!", &vars), "Hello {{name}}!");
    }
}
