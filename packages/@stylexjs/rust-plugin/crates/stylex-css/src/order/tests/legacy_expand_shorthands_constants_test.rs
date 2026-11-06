use crate::order::constants::legacy_expand_shorthands_order::{Aliases, Shorthands};
use stylex_structures::order_pair::OrderPair;

// ── Shorthands::get ──

// ── listStyle ──

// ── Border sub-types ──

// ── Unknown ──

// ── Aliases::get ──

#[test]
fn aliases_get_grid_gap_delegates() {
  let func = Aliases::get("gridGap").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 2);
}

#[test]
fn aliases_get_float_none() {
  let func = Aliases::get("float").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result[0], OrderPair("float".into(), None));
}

#[test]
fn aliases_get_clear_none() {
  let func = Aliases::get("clear").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result[0], OrderPair("clear".into(), None));
}

// ── Unknown ──

// ── containIntrinsicSize: the `auto` fold ──
// `auto 1px` means "1px, remembered", so the pair folds into one part before axes are assigned.
// Expectations from `@stylexjs/babel-plugin@0.19.0`.

// ── Coverage: listStyle with non-type tokens ──

// ── Coverage: listStyle error paths ──

/// Upstream's rejection text. The `var()`/global site double-quotes the JSON value; the other
/// three do not. Restated rather than shared with production so wording is actually tested.
fn rejection(json_of_raw_value: &str) -> String {
  format!("invalid \"listStyle\" value of {}", json_of_raw_value)
}

/// The rejection for `raw_value`, or a failure naming what it expanded to.
fn list_style_err(raw_value: &str) -> String {
  let func = match Shorthands::get("listStyle") {
    Some(func) => func,
    None => panic!("expected \"listStyle\" to be a known shorthand"),
  };

  match func(Some(raw_value.into())) {
    Ok(pairs) => panic!(
      "expected listStyle {:?} to be rejected, got {:?}",
      raw_value, pairs
    ),
    Err(err) => err,
  }
}

#[test]
fn shorthands_list_style_duplicate_position() {
  assert_eq!(
    list_style_err("inside outside"),
    rejection("\"inside outside\"")
  );
}

#[test]
fn shorthands_list_style_duplicate_type() {
  assert_eq!(list_style_err("disc square"), rejection("\"disc square\""));
}

// ── An empty part reaches every consumer of a part list ──
// Rule stated in `src/values/parser.rs`. No parity reference: Babel throws
// `Cannot read properties of undefined (reading 'type')` on these values.

fn padding_sides(value: &str) -> Vec<(String, String)> {
  let func = match Shorthands::get("padding") {
    Some(func) => func,
    None => panic!("expected \"padding\" to be a known shorthand"),
  };

  match func(Some(value.into())) {
    Ok(pairs) => pairs
      .iter()
      .map(|pair| (pair.0.to_string(), pair.value_text().into_owned()))
      .collect(),
    Err(err) => panic!("expected padding {value:?} to expand, got {err:?}"),
  }
}

#[test]
fn more_parts_than_sides_still_reads_the_first_four() {
  // The fifth part is discarded by position.
  assert_eq!(
    padding_sides("1px 2px 3px 4px 5px"),
    [
      ("paddingTop".to_string(), "1px".to_string()),
      ("paddingInlineEnd".to_string(), "2px".to_string()),
      ("paddingBottom".to_string(), "3px".to_string()),
      ("paddingInlineStart".to_string(), "4px".to_string()),
    ]
  );
  assert_eq!(
    padding_sides("1px /**/ 3px 4px 5px"),
    [
      ("paddingTop".to_string(), "1px".to_string()),
      ("paddingInlineEnd".to_string(), String::new()),
      ("paddingBottom".to_string(), "3px".to_string()),
      ("paddingInlineStart".to_string(), "4px".to_string()),
    ]
  );
}

#[test]
fn list_style_refuses_an_image_beside_an_empty_part() {
  assert_eq!(
    list_style_err("url(a.png) /**/"),
    rejection("\"url(a.png) /**/\"")
  );
}
