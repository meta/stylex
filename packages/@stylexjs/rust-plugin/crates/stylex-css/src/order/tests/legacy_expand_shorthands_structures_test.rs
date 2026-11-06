use stylex_structures::{order::Order, order_pair::OrderPair};

use crate::order::structures::legacy_expand_shorthands_order::LegacyExpandShorthandsOrder;

#[test]
fn get_expansion_fn_for_border() {
  let func = LegacyExpandShorthandsOrder::get_expansion_fn("border").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 4);
  assert_eq!(result[0].0, "borderTop");
}

#[test]
fn get_expansion_fn_for_margin() {
  let func = LegacyExpandShorthandsOrder::get_expansion_fn("margin").unwrap();
  let result = func(Some("10px".into())).unwrap();
  assert_eq!(result.len(), 4);
}

#[test]
fn get_expansion_fn_for_padding() {
  let func = LegacyExpandShorthandsOrder::get_expansion_fn("padding").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 4);
}

#[test]
fn get_expansion_fn_for_overflow() {
  let func = LegacyExpandShorthandsOrder::get_expansion_fn("overflow").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 2);
}

#[test]
fn get_expansion_fn_for_gap() {
  let func = LegacyExpandShorthandsOrder::get_expansion_fn("gap").unwrap();
  let result = func(Some("10px 20px".into())).unwrap();
  assert_eq!(result.len(), 2);
}

// ── Aliases via get_expansion_fn ────────────────────────────────────

#[test]
fn get_expansion_fn_for_alias_block_size() {
  let func = LegacyExpandShorthandsOrder::get_expansion_fn("blockSize").unwrap();
  let result = func(Some("100px".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("height".into(), Some("100px".into()))]
  );
}

#[test]
fn get_expansion_fn_for_alias_overflow_block() {
  let func = LegacyExpandShorthandsOrder::get_expansion_fn("overflowBlock").unwrap();
  let result = func(Some("auto".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("overflowY".into(), Some("auto".into()))]
  );
}

#[test]
fn get_expansion_fn_for_alias_float() {
  let func = LegacyExpandShorthandsOrder::get_expansion_fn("float").unwrap();
  let result = func(Some("start".into())).unwrap();
  assert!(result[0].value_text().contains("var("));
}

#[test]
fn get_expansion_fn_for_alias_clear() {
  let func = LegacyExpandShorthandsOrder::get_expansion_fn("clear").unwrap();
  let result = func(Some("end".into())).unwrap();
  assert!(result[0].value_text().contains("var("));
}

// ── Unknown ─────────────────────────────────────────────────────────

#[test]
fn get_expansion_fn_unknown_returns_none() {
  assert!(LegacyExpandShorthandsOrder::get_expansion_fn("color").is_none());
}

#[test]
fn get_expansion_fn_empty_returns_none() {
  assert!(LegacyExpandShorthandsOrder::get_expansion_fn("").is_none());
}

#[test]
fn get_expansion_fn_special_chars_returns_none() {
  assert!(LegacyExpandShorthandsOrder::get_expansion_fn("###").is_none());
}
