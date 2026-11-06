use stylex_structures::{order::Order, order_pair::OrderPair};

use crate::order::structures::property_specificity_order::PropertySpecificityOrder;
use crate::order::tests::REJECTING_SHORTHANDS;

// ── Aliases found via get_expansion_fn ──────────────────────────────

#[test]
fn get_expansion_fn_for_alias_block_size() {
  let func = PropertySpecificityOrder::get_expansion_fn("blockSize").unwrap();
  let result = func(Some("100px".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("height".into(), Some("100px".into()))]
  );
}

#[test]
fn get_expansion_fn_for_alias_inline_size() {
  let func = PropertySpecificityOrder::get_expansion_fn("inlineSize").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("width".into(), None)]);
}

#[test]
fn get_expansion_fn_for_alias_margin_block_start() {
  let func = PropertySpecificityOrder::get_expansion_fn("marginBlockStart").unwrap();
  let result = func(Some("5px".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("marginTop".into(), Some("5px".into()))]
  );
}

#[test]
fn get_expansion_fn_for_alias_overflow_block() {
  let func = PropertySpecificityOrder::get_expansion_fn("overflowBlock").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("overflowY".into(), None)]);
}

// ── Unknown returns None ────────────────────────────────────────────

#[test]
fn get_expansion_fn_unknown_returns_none() {
  assert!(PropertySpecificityOrder::get_expansion_fn("color").is_none());
}

#[test]
fn get_expansion_fn_empty_returns_none() {
  assert!(PropertySpecificityOrder::get_expansion_fn("").is_none());
}

#[test]
fn get_expansion_fn_special_chars_returns_none() {
  assert!(PropertySpecificityOrder::get_expansion_fn("@#$").is_none());
}

// ── Priority: alias found before shorthand ──────────────────────────

#[test]
fn alias_has_priority_for_inset_block_start() {
  let func = PropertySpecificityOrder::get_expansion_fn("insetBlockStart").unwrap();
  let result = func(Some("20px".into())).unwrap();
  assert_eq!(result, vec![OrderPair("top".into(), Some("20px".into()))]);
}

// ── Every rejecting shorthand is reachable by its authored name ─────

#[test]
fn every_rejecting_shorthand_is_reachable_by_its_authored_name() {
  // `border` agreed with the table by accident — it is spelled the same either
  // way. The rest were unreachable, so a `borderTop: "none"` reached the
  // stylesheet as `border-top:none` and defeated the specificity model this
  // table exists to enforce.
  for name in REJECTING_SHORTHANDS {
    let func = PropertySpecificityOrder::get_expansion_fn(name)
      .unwrap_or_else(|| panic!("'{name}' should have an expansion fn"));

    for value in [None, Some("1px solid red".into())] {
      match func(value) {
        Ok(pairs) => panic!("'{name}' should reject, expanded to {pairs:?}"),
        Err(error) => assert!(
          error.contains("is not supported"),
          "'{name}' reported {error:?}"
        ),
      }
    }
  }
}

#[test]
fn a_snake_case_spelling_is_not_a_property_name() {
  // The keys are authored property names, never the Rust identifiers that
  // implement them. Derived from the same list the table is checked against,
  // so the two cannot drift: a single-word name snake_cases to itself and is
  // skipped, since it is a real property either way.
  for name in REJECTING_SHORTHANDS {
    let snake = snake_case(name);

    if snake == name {
      continue;
    }

    assert!(
      PropertySpecificityOrder::get_expansion_fn(&snake).is_none(),
      "'{snake}' is not a property an author can write"
    );
  }
}

fn snake_case(name: &str) -> String {
  name.chars().fold(String::new(), |mut acc, ch| {
    if ch.is_ascii_uppercase() {
      acc.push('_');
      acc.push(ch.to_ascii_lowercase());
    } else {
      acc.push(ch);
    }

    acc
  })
}
