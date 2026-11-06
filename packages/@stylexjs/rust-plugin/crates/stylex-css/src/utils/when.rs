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
      // NOTE: a marker carries no `classNamePrefix`, so a value that reached
      // here without resolving yields the bare `default-marker`, prefixed
      // only when the options themselves occupy this slot.
      let prefix = match options.class_name_prefix() {
        Some(class_name_prefix) => format!("{}-", class_name_prefix),
        None => String::new(),
      };
      format!("{}default-marker", prefix)
    })
}

/// Resolves the second argument of every `when` function, which holds either
/// a marker class name used verbatim or a value to derive the marker from.
///
/// `None` stands for the `options` default parameter these functions declare.
/// Building the defaults on that branch costs an allocation, but the compiler
/// always passes its own options, so only callers with nothing to say — the
/// tests — reach it.
fn resolve_marker(options: Option<&dyn WhenMarkerValue>) -> String {
  match options {
    Some(options) => match options.as_str_value() {
      Some(marker) => marker.to_string(),
      None => get_default_marker_class_name(options),
    },
    // The closure is what turns the options into the trait object the reader
    // takes. A bare function name cannot, so it does not read as redundant.
    None => with_default_options(|options| get_default_marker_class_name(options)),
  }
}

/// Validates that a selector opens with ':' or '[', and is not a pseudo element.
fn validate_pseudo_selector(pseudo: &str) -> Result<(), String> {
  // The single colon is deliberate: pseudo elements must pass this gate so that
  // `is_pseudo_element` can reject them with their own message.
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

    // Validate proper bracket matching and quote pairing
    if !is_valid_attribute_selector(pseudo) {
      return Err(
        "Attribute selector has invalid format (mismatched brackets or quotes)".to_string(),
      );
    }
  }

  Ok(())
}

/// Validates that an attribute selector has proper bracket and quote matching
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

/// Creates selector that observes if the given pseudo-class is
/// active on an ancestor with the "defaultMarker"
///
/// # Arguments
/// * `pseudo` - The pseudo selector (e.g., ':hover', ':focus')
/// * `options` - Either a custom marker string or StyleXStateOptions reference
///
/// # Returns
/// A :where() clause for the ancestor observer
pub fn ancestor(pseudo: &str, options: Option<&dyn WhenMarkerValue>) -> Result<String, String> {
  validate_pseudo_selector(pseudo)?;
  let default_marker = resolve_marker(options);
  Ok(format!(":where(.{}{} *)", default_marker, pseudo))
}

/// Creates selector that observes if the given pseudo-class is
/// active on a descendant with the "defaultMarker"
///
/// # Arguments
/// * `pseudo` - The pseudo selector (e.g., ':hover', ':focus')
/// * `options` - Either a custom marker string or StyleXStateOptions reference
///
/// # Returns
/// A :has() clause for the descendant observer
pub fn descendant(pseudo: &str, options: Option<&dyn WhenMarkerValue>) -> Result<String, String> {
  validate_pseudo_selector(pseudo)?;
  let default_marker = resolve_marker(options);
  Ok(format!(":where(:has(.{}{}))", default_marker, pseudo))
}

/// Creates selector that observes if the given pseudo-class is
/// active on a previous sibling with the "defaultMarker"
///
/// # Arguments
/// * `pseudo` - The pseudo selector (e.g., ':hover', ':focus')
/// * `options` - Either a custom marker string or StyleXStateOptions reference
///
/// # Returns
/// A :where() clause for the previous sibling observer
pub fn sibling_before(
  pseudo: &str,
  options: Option<&dyn WhenMarkerValue>,
) -> Result<String, String> {
  validate_pseudo_selector(pseudo)?;
  let default_marker = resolve_marker(options);
  Ok(format!(":where(.{}{} ~ *)", default_marker, pseudo))
}

/// Creates selector that observes if the given pseudo-class is
/// active on a next sibling with the "defaultMarker"
///
/// # Arguments
/// * `pseudo` - The pseudo selector (e.g., ':hover', ':focus')
/// * `options` - Either a custom marker string or StyleXStateOptions reference
///
/// # Returns
/// A :has() clause for the next sibling observer
pub fn sibling_after(
  pseudo: &str,
  options: Option<&dyn WhenMarkerValue>,
) -> Result<String, String> {
  validate_pseudo_selector(pseudo)?;
  let default_marker = resolve_marker(options);
  Ok(format!(":where(:has(~ .{}{}))", default_marker, pseudo))
}

/// Creates selector that observes if the given pseudo-class is
/// active on any sibling with the "defaultMarker"
///
/// # Arguments
/// * `pseudo` - The pseudo selector (e.g., ':hover', ':focus')
/// * `options` - Either a custom marker string or StyleXStateOptions reference
///
/// # Returns
/// A :where() clause for the any sibling observer
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
