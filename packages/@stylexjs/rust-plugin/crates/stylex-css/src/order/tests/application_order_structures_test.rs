use stylex_structures::{order::Order, order_pair::OrderPair};

use crate::order::structures::application_order::ApplicationOrder;

#[test]
fn get_expansion_fn_for_animation() {
  let func = ApplicationOrder::get_expansion_fn("animation").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() > 10);
  assert_eq!(result[0].0, "animation");
}

#[test]
fn get_expansion_fn_for_border() {
  let func = ApplicationOrder::get_expansion_fn("border").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() > 5);
}

#[test]
fn get_expansion_fn_for_margin() {
  let func = ApplicationOrder::get_expansion_fn("margin").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
}

#[test]
fn get_expansion_fn_for_padding() {
  let func = ApplicationOrder::get_expansion_fn("padding").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
}

#[test]
fn get_expansion_fn_for_overflow() {
  let func = ApplicationOrder::get_expansion_fn("overflow").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

// ── Aliases are found through get_expansion_fn ──────────────────────

#[test]
fn get_expansion_fn_for_alias_block_size() {
  let func = ApplicationOrder::get_expansion_fn("blockSize").unwrap();
  let result = func(Some("100px".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("height".into(), Some("100px".into()))]
  );
}

#[test]
fn get_expansion_fn_for_alias_inline_size() {
  let func = ApplicationOrder::get_expansion_fn("inlineSize").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("width".into(), None)]);
}

#[test]
fn get_expansion_fn_for_alias_margin_block_start() {
  let func = ApplicationOrder::get_expansion_fn("marginBlockStart").unwrap();
  let result = func(Some("10px".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("marginTop".into(), Some("10px".into()))]
  );
}

#[test]
fn get_expansion_fn_for_alias_overflow_block() {
  let func = ApplicationOrder::get_expansion_fn("overflowBlock").unwrap();
  let result = func(Some("auto".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("overflowY".into(), Some("auto".into()))]
  );
}

// ── Unknown property returns None ───────────────────────────────────

#[test]
fn get_expansion_fn_unknown_returns_none() {
  assert!(ApplicationOrder::get_expansion_fn("color").is_none());
}

#[test]
fn get_expansion_fn_empty_returns_none() {
  assert!(ApplicationOrder::get_expansion_fn("").is_none());
}

#[test]
fn get_expansion_fn_special_chars_returns_none() {
  assert!(ApplicationOrder::get_expansion_fn("@!#").is_none());
}

// ── Alias has priority over shorthand if both exist ─────────────────

#[test]
fn alias_found_before_shorthand_for_border_horizontal_width() {
  // borderHorizontalWidth is an Alias, not a Shorthand
  let func = ApplicationOrder::get_expansion_fn("borderHorizontalWidth").unwrap();
  let result = func(None).unwrap();
  assert!(!result.is_empty());
}
