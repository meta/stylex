use stylex_structures::stylex_state_options::with_default_options;
use stylex_types::traits::WhenMarkerValue;

use crate::utils::pseudo::{is_pseudo_element, is_pseudo_selector};

pub fn from_proxy(value: &dyn WhenMarkerValue) -> Option<String> {
  value.as_proxy_string()
}

pub fn from_stylex_style(value: &dyn WhenMarkerValue) -> Option<String> {
  value.first_css_key().map(str::to_string)
}

/// Gets the default marker class name based on options.
fn get_default_marker_class_name(options: &dyn WhenMarkerValue) -> String {
  from_proxy(options)
    .or_else(|| from_stylex_style(options))
    .unwrap_or_else(|| {
      // Markers carry no `classNamePrefix`; only options supply one.
      let prefix = match options.class_name_prefix() {
        Some(class_name_prefix) => format!("{}-", class_name_prefix),
        None => String::new(),
      };
      format!("{}default-marker", prefix)
    })
}

/// Resolves a `when` function's second argument: a marker class name used verbatim, or a value to
/// derive it from. `None` means the default options.
fn resolve_marker(options: Option<&dyn WhenMarkerValue>) -> String {
  match options {
    Some(options) => match options.as_str_value() {
      Some(marker) => marker.to_string(),
      None => get_default_marker_class_name(options),
    },
    // A closure, not a bare fn name, is needed to coerce to the trait object.
    None => with_default_options(|options| get_default_marker_class_name(options)),
  }
}

/// Validates that a selector opens with ':' or '[', and is not a pseudo element.
fn validate_pseudo_selector(pseudo: &str) -> Result<(), String> {
  // Single colon on purpose: pseudo elements get their own message below.
  if !is_pseudo_selector(pseudo) && !pseudo.starts_with('[') {
    return Err("Pseudo selector must start with \":\" or \"[\"".to_string());
  }

  if is_pseudo_element(pseudo) {
    return Err(
      "Pseudo selector cannot start with \"::\" (pseudo-elements are not supported)".to_string(),
    );
  }

  if pseudo.starts_with('[') {
    if !pseudo.ends_with(']') {
      return Err("Attribute selector must end with \"]\"".to_string());
    }

    if !is_valid_attribute_selector(pseudo) {
      return Err(
        "Attribute selector has invalid format (mismatched brackets or quotes)".to_string(),
      );
    }
  }

  Ok(())
}

/// Validates that an attribute selector has proper bracket and quote matching.
fn is_valid_attribute_selector(selector: &str) -> bool {
  if !selector.starts_with('[') || !selector.ends_with(']') {
    return false;
  }

  let mut in_single_quote = false;
  let mut in_double_quote = false;
  let mut bracket_count: i32 = 1;
  let mut prev = '[';

  for c in selector[1..].chars() {
    match c {
      '\'' if prev != '\\' && !in_double_quote => in_single_quote = !in_single_quote,
      '"' if prev != '\\' && !in_single_quote => in_double_quote = !in_double_quote,
      '[' if !in_single_quote && !in_double_quote => {
        bracket_count += 1;
        // CSS attribute selectors can only have one opening bracket (at the start)
        if bracket_count > 1 {
          return false;
        }
      },
      ']' if !in_single_quote && !in_double_quote => {
        bracket_count -= 1;
        if bracket_count < 0 {
          return false;
        }
      },
      _ => {},
    }
    prev = c;
  }

  bracket_count == 0 && !in_single_quote && !in_double_quote
}

/// Selector matching when `pseudo` is active on an ancestor with the default marker.
pub fn ancestor(pseudo: &str, options: Option<&dyn WhenMarkerValue>) -> Result<String, String> {
  validate_pseudo_selector(pseudo)?;
  let default_marker = resolve_marker(options);
  Ok(format!(":where(.{}{} *)", default_marker, pseudo))
}

/// Selector matching when `pseudo` is active on a descendant with the default marker.
pub fn descendant(pseudo: &str, options: Option<&dyn WhenMarkerValue>) -> Result<String, String> {
  validate_pseudo_selector(pseudo)?;
  let default_marker = resolve_marker(options);
  Ok(format!(":where(:has(.{}{}))", default_marker, pseudo))
}

/// Selector matching when `pseudo` is active on a previous sibling with the default marker.
pub fn sibling_before(
  pseudo: &str,
  options: Option<&dyn WhenMarkerValue>,
) -> Result<String, String> {
  validate_pseudo_selector(pseudo)?;
  let default_marker = resolve_marker(options);
  Ok(format!(":where(.{}{} ~ *)", default_marker, pseudo))
}

/// Selector matching when `pseudo` is active on a next sibling with the default marker.
pub fn sibling_after(
  pseudo: &str,
  options: Option<&dyn WhenMarkerValue>,
) -> Result<String, String> {
  validate_pseudo_selector(pseudo)?;
  let default_marker = resolve_marker(options);
  Ok(format!(":where(:has(~ .{}{}))", default_marker, pseudo))
}

/// Selector matching when `pseudo` is active on any sibling with the default marker.
pub fn any_sibling(pseudo: &str, options: Option<&dyn WhenMarkerValue>) -> Result<String, String> {
  validate_pseudo_selector(pseudo)?;
  let default_marker = resolve_marker(options);
  Ok(format!(
    ":where(.{}{} ~ *, :has(~ .{}{}))",
    default_marker, pseudo, default_marker, pseudo
  ))
}

#[cfg(test)]
#[path = "../tests/when_tests.rs"]
mod tests;
