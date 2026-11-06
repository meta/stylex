use crate::order::constants::legacy_expand_shorthands_order::{
  Aliases, Shorthands, is_list_style_type,
};
use stylex_structures::order_pair::OrderPair;

// ── Shorthands::get ──

#[test]
fn shorthands_get_border_expands() {
  let func = Shorthands::get("border").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 4);
  assert_eq!(result[0].0, "borderTop");
}

#[test]
fn shorthands_get_border_with_value() {
  let func = Shorthands::get("border").unwrap();
  let result = func(Some("1px solid red".into())).unwrap();
  assert_eq!(result[0].1, Some("1px solid red".into()));
}

#[test]
fn shorthands_get_border_color() {
  let func = Shorthands::get("borderColor").unwrap();
  let result = func(Some("red green blue yellow".into())).unwrap();
  assert_eq!(result.len(), 4);
  assert_eq!(
    result[0],
    OrderPair("borderTopColor".into(), Some("red".into()))
  );
}

#[test]
fn shorthands_get_border_style() {
  let func = Shorthands::get("borderStyle").unwrap();
  let result = func(Some("solid".into())).unwrap();
  assert_eq!(result.len(), 4);
}

#[test]
fn shorthands_get_border_width() {
  let func = Shorthands::get("borderWidth").unwrap();
  let result = func(Some("1px 2px".into())).unwrap();
  assert_eq!(result.len(), 4);
}

#[test]
fn shorthands_get_border_horizontal() {
  let func = Shorthands::get("borderHorizontal").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 2);
  assert_eq!(result[0].0, "borderInlineStart");
  assert_eq!(result[1].0, "borderInlineEnd");
}

#[test]
fn shorthands_get_border_vertical() {
  let func = Shorthands::get("borderVertical").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 2);
}

#[test]
fn shorthands_get_border_horizontal_color() {
  let func = Shorthands::get("borderHorizontalColor").unwrap();
  let result = func(Some("red".into())).unwrap();
  assert_eq!(result.len(), 2);
  assert_eq!(result[0].0, "borderInlineStartColor");
}

#[test]
fn shorthands_get_border_inline_width() {
  let func = Shorthands::get("borderInlineWidth").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 2);
}

#[test]
fn shorthands_get_border_vertical_width() {
  let func = Shorthands::get("borderVerticalWidth").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 2);
}

#[test]
fn shorthands_get_border_radius() {
  let func = Shorthands::get("borderRadius").unwrap();
  let result = func(Some("4px".into())).unwrap();
  assert_eq!(result.len(), 4);
  assert_eq!(result[0].0, "borderStartStartRadius");
}

#[test]
fn shorthands_get_contain_intrinsic_size() {
  let func = Shorthands::get("containIntrinsicSize").unwrap();
  let result = func(Some("300px 200px".into())).unwrap();
  assert_eq!(result.len(), 2);
  assert_eq!(result[0].0, "containIntrinsicWidth");
  assert_eq!(result[1].0, "containIntrinsicHeight");
}

#[test]
fn shorthands_get_contain_intrinsic_size_auto() {
  let func = Shorthands::get("containIntrinsicSize").unwrap();
  let result = func(Some("auto 300px".into())).unwrap();
  assert_eq!(result.len(), 2);
  let width = result[0].value_text();
  assert!(width.contains("auto"));
}

#[test]
fn shorthands_get_inset() {
  let func = Shorthands::get("inset").unwrap();
  let result = func(Some("10px 20px 30px 40px".into())).unwrap();
  assert_eq!(result.len(), 4);
  assert_eq!(result[0], OrderPair("top".into(), Some("10px".into())));
}

#[test]
fn shorthands_get_inset_inline() {
  let func = Shorthands::get("insetInline").unwrap();
  let result = func(Some("10px 20px".into())).unwrap();
  assert!(result.len() >= 4); // start + end each expand to 3
}

#[test]
fn shorthands_get_inset_block() {
  let func = Shorthands::get("insetBlock").unwrap();
  let result = func(Some("10px 20px".into())).unwrap();
  assert_eq!(result.len(), 2);
}

#[test]
fn shorthands_get_start() {
  let func = Shorthands::get("start").unwrap();
  let result = func(Some("10px".into())).unwrap();
  assert_eq!(result.len(), 3);
  assert_eq!(
    result[0],
    OrderPair("insetInlineStart".into(), Some("10px".into()))
  );
}

#[test]
fn shorthands_get_end() {
  let func = Shorthands::get("end").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 3);
  assert_eq!(result[0].0, "insetInlineEnd");
}

#[test]
fn shorthands_get_left() {
  let func = Shorthands::get("left").unwrap();
  let result = func(Some("5px".into())).unwrap();
  assert_eq!(result.len(), 3);
  assert_eq!(result[0], OrderPair("left".into(), Some("5px".into())));
}

#[test]
fn shorthands_get_right() {
  let func = Shorthands::get("right").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 3);
}

#[test]
fn shorthands_get_gap() {
  let func = Shorthands::get("gap").unwrap();
  let result = func(Some("10px 20px".into())).unwrap();
  assert_eq!(result.len(), 2);
  assert_eq!(result[0], OrderPair("rowGap".into(), Some("10px".into())));
  assert_eq!(
    result[1],
    OrderPair("columnGap".into(), Some("20px".into()))
  );
}

#[test]
fn shorthands_get_margin() {
  let func = Shorthands::get("margin").unwrap();
  let result = func(Some("10px".into())).unwrap();
  assert_eq!(result.len(), 4);
  assert_eq!(result[0].0, "marginTop");
}

#[test]
fn shorthands_get_margin_horizontal() {
  let func = Shorthands::get("marginHorizontal").unwrap();
  let result = func(Some("10px 20px".into())).unwrap();
  // margin_start(3) + margin_end(3)
  assert_eq!(result.len(), 6);
}

#[test]
fn shorthands_get_margin_start() {
  let func = Shorthands::get("marginStart").unwrap();
  let result = func(Some("10px".into())).unwrap();
  assert_eq!(result.len(), 3);
  assert_eq!(
    result[0],
    OrderPair("marginInlineStart".into(), Some("10px".into()))
  );
}

#[test]
fn shorthands_get_margin_end() {
  let func = Shorthands::get("marginEnd").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 3);
}

#[test]
fn shorthands_get_margin_left() {
  let func = Shorthands::get("marginLeft").unwrap();
  let result = func(Some("5px".into())).unwrap();
  assert_eq!(result.len(), 3);
  assert_eq!(
    result[0],
    OrderPair("marginLeft".into(), Some("5px".into()))
  );
}

#[test]
fn shorthands_get_margin_right() {
  let func = Shorthands::get("marginRight").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 3);
}

#[test]
fn shorthands_get_margin_vertical() {
  let func = Shorthands::get("marginVertical").unwrap();
  let result = func(Some("10px".into())).unwrap();
  assert_eq!(result.len(), 2);
}

#[test]
fn shorthands_get_overflow() {
  let func = Shorthands::get("overflow").unwrap();
  let result = func(Some("hidden scroll".into())).unwrap();
  assert_eq!(result.len(), 2);
  assert_eq!(
    result[0],
    OrderPair("overflowX".into(), Some("hidden".into()))
  );
}

#[test]
fn shorthands_get_padding() {
  let func = Shorthands::get("padding").unwrap();
  let result = func(Some("10px 20px 30px 40px".into())).unwrap();
  assert_eq!(result.len(), 4);
  assert_eq!(
    result[0],
    OrderPair("paddingTop".into(), Some("10px".into()))
  );
}

#[test]
fn shorthands_get_padding_horizontal() {
  let func = Shorthands::get("paddingHorizontal").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 6);
}

#[test]
fn shorthands_get_padding_start() {
  let func = Shorthands::get("paddingStart").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 3);
}

#[test]
fn shorthands_get_padding_end() {
  let func = Shorthands::get("paddingEnd").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 3);
}

#[test]
fn shorthands_get_padding_left() {
  let func = Shorthands::get("paddingLeft").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 3);
}

#[test]
fn shorthands_get_padding_right() {
  let func = Shorthands::get("paddingRight").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 3);
}

#[test]
fn shorthands_get_padding_vertical() {
  let func = Shorthands::get("paddingVertical").unwrap();
  let result = func(Some("8px 12px".into())).unwrap();
  assert_eq!(result.len(), 2);
}

// ── listStyle ──

#[test]
fn shorthands_get_list_style_none() {
  let func = Shorthands::get("listStyle").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 3);
  assert!(result.iter().all(|p| p.1.is_none()));
}

#[test]
fn shorthands_get_list_style_single_type() {
  let func = Shorthands::get("listStyle").unwrap();
  let result = func(Some("disc".into())).unwrap();
  assert_eq!(result.len(), 3);
  let type_val = &result[0];
  assert_eq!(type_val.0, "listStyleType");
  assert_eq!(type_val.1, Some("disc".into()));
}

#[test]
fn shorthands_get_list_style_position() {
  let func = Shorthands::get("listStyle").unwrap();
  let result = func(Some("inside".into())).unwrap();
  let pos = &result[1];
  assert_eq!(pos.0, "listStylePosition");
  assert_eq!(pos.1, Some("inside".into()));
}

#[test]
fn shorthands_get_list_style_global_inherit() {
  let func = Shorthands::get("listStyle").unwrap();
  let result = func(Some("inherit".into())).unwrap();
  assert_eq!(result.len(), 3);
  assert!(result.iter().all(|p| p.1 == Some("inherit".into())));
}

#[test]
fn shorthands_get_list_style_none_value() {
  let func = Shorthands::get("listStyle").unwrap();
  let result = func(Some("none".into())).unwrap();
  assert_eq!(result.len(), 3);
  assert_eq!(result[0].1, Some("none".into()));
}

#[test]
fn shorthands_get_list_style_quoted_type() {
  let func = Shorthands::get("listStyle").unwrap();
  let result = func(Some("\"→\"".into())).unwrap();
  assert_eq!(result[0].0, "listStyleType");
}

#[test]
fn shorthands_get_list_style_quoted_type_single_quote() {
  let func = Shorthands::get("listStyle").unwrap();
  let result = func(Some("'→'".into())).unwrap();
  assert_eq!(result[0].0, "listStyleType");
}

#[test]
fn shorthands_get_list_style_type_and_position() {
  let func = Shorthands::get("listStyle").unwrap();
  let result = func(Some("disc inside".into())).unwrap();
  assert_eq!(
    result[0],
    OrderPair("listStyleType".into(), Some("disc".into()))
  );
  assert_eq!(
    result[1],
    OrderPair("listStylePosition".into(), Some("inside".into()))
  );
}

// ── Border sub-types ──

#[test]
fn shorthands_get_border_horizontal_style() {
  let func = Shorthands::get("borderHorizontalStyle").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 2);
}

#[test]
fn shorthands_get_border_horizontal_width() {
  let func = Shorthands::get("borderHorizontalWidth").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 2);
}

#[test]
fn shorthands_get_border_inline_color() {
  let func = Shorthands::get("borderInlineColor").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 2);
}

#[test]
fn shorthands_get_border_inline_style() {
  let func = Shorthands::get("borderInlineStyle").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 2);
}

#[test]
fn shorthands_get_border_vertical_color() {
  let func = Shorthands::get("borderVerticalColor").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 2);
}

#[test]
fn shorthands_get_border_vertical_style() {
  let func = Shorthands::get("borderVerticalStyle").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 2);
}

// ── Unknown ──

#[test]
fn shorthands_get_unknown_returns_none() {
  assert!(Shorthands::get("nonexistent").is_none());
}

#[test]
fn shorthands_get_empty_returns_none() {
  assert!(Shorthands::get("").is_none());
}

#[test]
fn shorthands_get_special_chars_returns_none() {
  assert!(Shorthands::get("$%^").is_none());
}

// ── Aliases::get ──

#[test]
fn aliases_get_inset_block_start() {
  let func = Aliases::get("insetBlockStart").unwrap();
  let result = func(Some("10px".into())).unwrap();
  assert_eq!(result, vec![OrderPair("top".into(), Some("10px".into()))]);
}

#[test]
fn aliases_get_inset_block_end() {
  let func = Aliases::get("insetBlockEnd").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("bottom".into(), None)]);
}

#[test]
fn aliases_get_inset_inline_start_delegates() {
  let func = Aliases::get("insetInlineStart").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 3);
}

#[test]
fn aliases_get_inset_inline_end_delegates() {
  let func = Aliases::get("insetInlineEnd").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 3);
}

#[test]
fn aliases_get_block_size() {
  let func = Aliases::get("blockSize").unwrap();
  let result = func(Some("200px".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("height".into(), Some("200px".into()))]
  );
}

#[test]
fn aliases_get_inline_size() {
  let func = Aliases::get("inlineSize").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("width".into(), None)]);
}

#[test]
fn aliases_get_min_max_block_inline_size() {
  let min_block = Aliases::get("minBlockSize").unwrap()(None).unwrap();
  let min_inline = Aliases::get("minInlineSize").unwrap()(None).unwrap();
  let max_block = Aliases::get("maxBlockSize").unwrap()(None).unwrap();
  let max_inline = Aliases::get("maxInlineSize").unwrap()(None).unwrap();
  assert_eq!(min_block[0].0, "minHeight");
  assert_eq!(min_inline[0].0, "minWidth");
  assert_eq!(max_block[0].0, "maxHeight");
  assert_eq!(max_inline[0].0, "maxWidth");
}

#[test]
fn aliases_get_border_block_delegates() {
  for name in &["borderBlockWidth", "borderBlockStyle", "borderBlockColor"] {
    let func = Aliases::get(name).unwrap();
    let result = func(None).unwrap();
    assert_eq!(result.len(), 2);
  }
}

#[test]
fn aliases_get_border_inline_delegates() {
  for name in &[
    "borderInlineWidth",
    "borderInlineStyle",
    "borderInlineColor",
  ] {
    let func = Aliases::get(name).unwrap();
    let result = func(None).unwrap();
    assert_eq!(result.len(), 2);
  }
}

#[test]
fn aliases_get_border_start_end() {
  let start = Aliases::get("borderStart").unwrap()(None).unwrap();
  let end = Aliases::get("borderEnd").unwrap()(None).unwrap();
  assert_eq!(start[0].0, "borderInlineStart");
  assert_eq!(end[0].0, "borderInlineEnd");
}

#[test]
fn aliases_get_border_block_start_end_properties() {
  let bsw = Aliases::get("borderBlockStartWidth").unwrap()(None).unwrap();
  let bss = Aliases::get("borderBlockStartStyle").unwrap()(None).unwrap();
  let bsc = Aliases::get("borderBlockStartColor").unwrap()(None).unwrap();
  let bew = Aliases::get("borderBlockEndWidth").unwrap()(None).unwrap();
  let bes = Aliases::get("borderBlockEndStyle").unwrap()(None).unwrap();
  let bec = Aliases::get("borderBlockEndColor").unwrap()(None).unwrap();
  assert_eq!(bsw[0].0, "borderTopWidth");
  assert_eq!(bss[0].0, "borderTopStyle");
  assert_eq!(bsc[0].0, "borderTopColor");
  assert_eq!(bew[0].0, "borderBottomWidth");
  assert_eq!(bes[0].0, "borderBottomStyle");
  assert_eq!(bec[0].0, "borderBottomColor");
}

/// A self-resolving alias emitted `border-top-start-radius`, which browsers drop.
#[test]
fn aliases_get_border_radius_aliases() {
  let ss = Aliases::get("borderTopStartRadius").unwrap()(None).unwrap();
  let se = Aliases::get("borderTopEndRadius").unwrap()(None).unwrap();
  let es = Aliases::get("borderBottomStartRadius").unwrap()(None).unwrap();
  let ee = Aliases::get("borderBottomEndRadius").unwrap()(None).unwrap();
  assert_eq!(ss[0].0, "borderStartStartRadius");
  assert_eq!(se[0].0, "borderStartEndRadius");
  assert_eq!(es[0].0, "borderEndStartRadius");
  assert_eq!(ee[0].0, "borderEndEndRadius");
}

#[test]
fn aliases_get_grid_gap_delegates() {
  let func = Aliases::get("gridGap").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 2);
}

#[test]
fn aliases_get_grid_row_gap() {
  let func = Aliases::get("gridRowGap").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("rowGap".into(), None)]);
}

#[test]
fn aliases_get_grid_column_gap() {
  let func = Aliases::get("gridColumnGap").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("columnGap".into(), None)]);
}

#[test]
fn aliases_get_margin_block_start_end() {
  let start = Aliases::get("marginBlockStart").unwrap()(Some("5px".into())).unwrap();
  let end = Aliases::get("marginBlockEnd").unwrap()(None).unwrap();
  assert_eq!(start[0], OrderPair("marginTop".into(), Some("5px".into())));
  assert_eq!(end[0], OrderPair("marginBottom".into(), None));
}

#[test]
fn aliases_get_margin_inline_start_end() {
  let start = Aliases::get("marginInlineStart").unwrap()(None).unwrap();
  let end = Aliases::get("marginInlineEnd").unwrap()(None).unwrap();
  assert_eq!(start[0].0, "marginInlineStart");
  assert_eq!(end[0].0, "marginInlineEnd");
}

#[test]
fn aliases_get_margin_block_delegates() {
  let func = Aliases::get("marginBlock").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 2);
}

#[test]
fn aliases_get_margin_inline_delegates() {
  let func = Aliases::get("marginInline").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 6);
}

#[test]
fn aliases_get_overflow_block_inline() {
  let block = Aliases::get("overflowBlock").unwrap()(None).unwrap();
  let inline = Aliases::get("overflowInline").unwrap()(None).unwrap();
  assert_eq!(block[0].0, "overflowY");
  assert_eq!(inline[0].0, "overflowX");
}

#[test]
fn aliases_get_padding_block_start_end() {
  let start = Aliases::get("paddingBlockStart").unwrap()(None).unwrap();
  let end = Aliases::get("paddingBlockEnd").unwrap()(None).unwrap();
  assert_eq!(start[0].0, "paddingTop");
  assert_eq!(end[0].0, "paddingBottom");
}

#[test]
fn aliases_get_padding_inline_start_end() {
  let start = Aliases::get("paddingInlineStart").unwrap()(None).unwrap();
  let end = Aliases::get("paddingInlineEnd").unwrap()(None).unwrap();
  assert_eq!(start[0].0, "paddingInlineStart");
  assert_eq!(end[0].0, "paddingInlineEnd");
}

#[test]
fn aliases_get_padding_block_delegates() {
  let func = Aliases::get("paddingBlock").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 2);
}

#[test]
fn aliases_get_padding_inline_delegates() {
  let func = Aliases::get("paddingInline").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result.len(), 6);
}

#[test]
fn aliases_get_scroll_margin_block_start_end() {
  let start = Aliases::get("scrollMarginBlockStart").unwrap()(None).unwrap();
  let end = Aliases::get("scrollMarginBlockEnd").unwrap()(None).unwrap();
  assert_eq!(start[0].0, "scrollMarginTop");
  assert_eq!(end[0].0, "scrollMarginBottom");
}

#[test]
fn aliases_get_float_start() {
  let func = Aliases::get("float").unwrap();
  let result = func(Some("start".into())).unwrap();
  assert_eq!(result[0].0, "float");
  assert!(result[0].value_text().contains("var("));
}

#[test]
fn aliases_get_float_inline_start() {
  let func = Aliases::get("float").unwrap();
  let result = func(Some("inline-start".into())).unwrap();
  assert!(result[0].value_text().contains("var("));
}

#[test]
fn aliases_get_float_end() {
  let func = Aliases::get("float").unwrap();
  let result = func(Some("end".into())).unwrap();
  assert!(result[0].value_text().contains("var("));
}

#[test]
fn aliases_get_float_inline_end() {
  let func = Aliases::get("float").unwrap();
  let result = func(Some("inline-end".into())).unwrap();
  assert!(result[0].value_text().contains("var("));
}

#[test]
fn aliases_get_float_left_passthrough() {
  let func = Aliases::get("float").unwrap();
  let result = func(Some("left".into())).unwrap();
  assert_eq!(result[0], OrderPair("float".into(), Some("left".into())));
}

#[test]
fn aliases_get_float_none() {
  let func = Aliases::get("float").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result[0], OrderPair("float".into(), None));
}

#[test]
fn aliases_get_clear_start() {
  let func = Aliases::get("clear").unwrap();
  let result = func(Some("start".into())).unwrap();
  assert!(result[0].value_text().contains("var("));
}

#[test]
fn aliases_get_clear_end() {
  let func = Aliases::get("clear").unwrap();
  let result = func(Some("end".into())).unwrap();
  assert!(result[0].value_text().contains("var("));
}

#[test]
fn aliases_get_clear_inline_start() {
  let func = Aliases::get("clear").unwrap();
  let result = func(Some("inline-start".into())).unwrap();
  assert!(result[0].value_text().contains("var("));
}

#[test]
fn aliases_get_clear_inline_end() {
  let func = Aliases::get("clear").unwrap();
  let result = func(Some("inline-end".into())).unwrap();
  assert!(result[0].value_text().contains("var("));
}

#[test]
fn aliases_get_clear_both_passthrough() {
  let func = Aliases::get("clear").unwrap();
  let result = func(Some("both".into())).unwrap();
  assert_eq!(result[0], OrderPair("clear".into(), Some("both".into())));
}

#[test]
fn aliases_get_clear_none() {
  let func = Aliases::get("clear").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result[0], OrderPair("clear".into(), None));
}

// ── Unknown ──

#[test]
fn aliases_get_unknown_returns_none() {
  assert!(Aliases::get("nonexistent").is_none());
}

#[test]
fn aliases_get_empty_returns_none() {
  assert!(Aliases::get("").is_none());
}

// ── containIntrinsicSize: the `auto` fold ──
// `auto 1px` means "1px, remembered", so the pair folds into one part before axes are assigned.
// Expectations from `@stylexjs/babel-plugin@0.19.0`.

fn intrinsic_size(value: &str) -> (String, String) {
  // Handled rather than unwrapped, per `guidelines/stack/RUST.md`.
  let func = match Shorthands::get("containIntrinsicSize") {
    Some(func) => func,
    None => panic!("containIntrinsicSize has no expansion"),
  };
  let result = match func(Some(value.into())) {
    Ok(result) => result,
    Err(error) => panic!("containIntrinsicSize refused {value:?}: {error}"),
  };

  assert_eq!(result.len(), 2);
  assert_eq!(result[0].0, "containIntrinsicWidth");
  assert_eq!(result[1].0, "containIntrinsicHeight");

  (
    result[0].value_text().into_owned(),
    result[1].value_text().into_owned(),
  )
}

#[test]
fn one_size_sizes_both_axes() {
  assert_eq!(
    intrinsic_size("300px"),
    ("300px".to_string(), "300px".to_string())
  );
}

#[test]
fn two_sizes_take_one_axis_each() {
  assert_eq!(
    intrinsic_size("300px 200px"),
    ("300px".to_string(), "200px".to_string())
  );
  assert_eq!(
    intrinsic_size("1px 2px 3px"),
    ("1px".to_string(), "2px".to_string())
  );
}

#[test]
fn auto_joins_the_size_after_it_into_one_part() {
  assert_eq!(
    intrinsic_size("auto 300px"),
    ("auto 300px".to_string(), "auto 300px".to_string())
  );
  assert_eq!(
    intrinsic_size("auto 300px auto 200px"),
    ("auto 300px".to_string(), "auto 200px".to_string())
  );
}

#[test]
fn a_lone_auto_qualifies_nothing_and_sizes_both_axes() {
  // The fold once ran over the four-sided view, yielding `auto auto` on both axes.
  assert_eq!(
    intrinsic_size("auto"),
    ("auto".to_string(), "auto".to_string())
  );
}

#[test]
fn a_trailing_auto_has_nothing_to_qualify() {
  // Same cause: the height came out `auto 300px`.
  assert_eq!(
    intrinsic_size("300px auto"),
    ("300px".to_string(), "auto".to_string())
  );
}

#[test]
fn auto_joins_an_empty_part_too() {
  // Upstream asks whether the part is absent, which no split part is; an empty part
  // (from `auto /*`) still takes an axis.
  assert_eq!(
    intrinsic_size("auto /*"),
    ("auto ".to_string(), "auto ".to_string())
  );
}

#[test]
fn a_lone_quote_is_not_a_quoted_list_style_type() {
  // `/^".*?"$/` needs two characters. Not observable through the expansion, which falls back to
  // type anyway.
  assert!(!is_list_style_type("\""));
  assert!(!is_list_style_type("'"));
  assert!(is_list_style_type("\"\""));
  assert!(is_list_style_type("''"));
  assert!(is_list_style_type("'a'"));
  assert!(is_list_style_type("\"a\""));
  // A line terminator fails upstream's `.`.
  assert!(!is_list_style_type("\"a\nb\""));
  // The identifier alternative is lowercase and hyphens only.
  assert!(is_list_style_type("lower-alpha"));
  assert!(!is_list_style_type("Disc"));
  assert!(!is_list_style_type(""));
}

#[test]
fn two_autos_join_each_other() {
  assert_eq!(
    intrinsic_size("auto auto"),
    ("auto auto".to_string(), "auto auto".to_string())
  );
  assert_eq!(
    intrinsic_size("auto auto 300px"),
    ("auto auto".to_string(), "300px".to_string())
  );
}

// ── Coverage: listStyle with non-type tokens ──

/// `list-style-type` identifiers are lowercase, so `Disc` falls through to image.
#[test]
fn shorthands_list_style_uppercase_ident_falls_through() {
  let func = Shorthands::get("listStyle").unwrap();
  let result = func(Some("Disc".into())).unwrap();
  assert_eq!(result[2].0, "listStyleImage");
  assert_eq!(result[2].1, Some("Disc".into()));
}

// ── Coverage: listStyle error paths ──

/// Upstream's rejection text. The `var()`/global site double-quotes the JSON value; the other
/// three do not. Restated rather than shared with production so wording is actually tested.
fn rejection(json_of_raw_value: &str) -> String {
  format!("invalid \"listStyle\" value of {}", json_of_raw_value)
}

fn rejection_with_doubled_quotes(json_of_raw_value: &str) -> String {
  format!("invalid \"listStyle\" value of \"{}\"", json_of_raw_value)
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

/// The site with the extra quotes.
#[test]
fn shorthands_list_style_var_mixed_with_other() {
  assert_eq!(
    list_style_err("disc var(--foo)"),
    rejection_with_doubled_quotes("\"disc var(--foo)\"")
  );
}

#[test]
fn shorthands_list_style_global_mixed() {
  assert_eq!(
    list_style_err("disc inherit"),
    rejection_with_doubled_quotes("\"disc inherit\"")
  );
  assert_eq!(
    list_style_err("none inherit"),
    rejection_with_doubled_quotes("\"none inherit\"")
  );
}

#[test]
fn shorthands_list_style_global_first_reads_the_same() {
  assert_eq!(
    list_style_err("inherit disc"),
    rejection_with_doubled_quotes("\"inherit disc\"")
  );
}

#[test]
fn shorthands_list_style_every_global_keyword_is_refused_when_mixed() {
  for keyword in ["inherit", "initial", "revert", "unset"] {
    let raw_value = format!("disc {}", keyword);
    assert_eq!(
      list_style_err(&raw_value),
      rejection_with_doubled_quotes(&format!("\"{}\"", raw_value))
    );
  }
}

#[test]
fn shorthands_list_style_duplicate_position() {
  assert_eq!(
    list_style_err("inside outside"),
    rejection("\"inside outside\"")
  );
}

/// The position site throws first, so no doubled quotes despite the global keyword.
#[test]
fn shorthands_list_style_duplicate_position_wins_over_a_later_global() {
  assert_eq!(
    list_style_err("inside outside inherit"),
    rejection("\"inside outside inherit\"")
  );
}

#[test]
fn shorthands_list_style_duplicate_type() {
  assert_eq!(list_style_err("disc square"), rejection("\"disc square\""));
}

#[test]
fn shorthands_list_style_duplicate_quoted_type() {
  assert_eq!(
    list_style_err("\"disc\" \"square\""),
    rejection("\"\\\"disc\\\" \\\"square\\\"\"")
  );
}

#[test]
fn shorthands_list_style_too_many_nones() {
  // type, image, then a duplicate image.
  assert_eq!(
    list_style_err("none none none"),
    rejection("\"none none none\"")
  );
}

/// Quoted via `JSON.stringify`; the C0 control is where `{:?}` would differ.
#[test]
fn shorthands_list_style_rejection_escapes_the_value_as_json_does() {
  assert_eq!(
    list_style_err("inherit \tx"),
    rejection_with_doubled_quotes("\"inherit \\tx\"")
  );
}

#[test]
fn shorthands_list_style_rejection_escapes_a_c0_control_in_the_value() {
  assert_eq!(
    list_style_err("inherit \u{1}x"),
    rejection_with_doubled_quotes("\"inherit \\u0001x\"")
  );
}

/// Astral scalars included; two non-keywords collide at the image site (no doubled quotes).
#[test]
fn shorthands_list_style_rejection_writes_non_ascii_through_raw() {
  assert_eq!(list_style_err("é é"), rejection("\"é é\""));
  assert_eq!(
    list_style_err("inherit 🎉"),
    rejection_with_doubled_quotes("\"inherit 🎉\"")
  );
}

#[test]
fn shorthands_list_style_rejection_quotes_a_long_value_whole() {
  let raw_value = format!("inherit {}", "x ".repeat(500).trim_end());
  assert_eq!(
    list_style_err(&raw_value),
    rejection_with_doubled_quotes(&format!("\"{}\"", raw_value))
  );
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
fn the_four_sided_view_gives_an_empty_part_the_side_it_landed_on() {
  // The empty sides still exist as pairs; empty declarations are dropped by the rule writer.
  assert_eq!(
    padding_sides("1px /*"),
    [
      ("paddingTop".to_string(), "1px".to_string()),
      ("paddingInlineEnd".to_string(), String::new()),
      ("paddingBottom".to_string(), "1px".to_string()),
      ("paddingInlineStart".to_string(), String::new()),
    ]
  );
}

#[test]
fn an_empty_first_part_does_not_slide_the_remaining_sides_up() {
  // Reading the empty part as absent would make `1px` the top.
  assert_eq!(
    padding_sides("/**/ 1px"),
    [
      ("paddingTop".to_string(), String::new()),
      ("paddingInlineEnd".to_string(), "1px".to_string()),
      ("paddingBottom".to_string(), String::new()),
      ("paddingInlineStart".to_string(), "1px".to_string()),
    ]
  );
}

#[test]
fn a_value_of_nothing_but_empty_parts_still_expands_to_four_sides() {
  assert_eq!(
    padding_sides("/**//**/"),
    [
      ("paddingTop".to_string(), String::new()),
      ("paddingInlineEnd".to_string(), String::new()),
      ("paddingBottom".to_string(), String::new()),
      ("paddingInlineStart".to_string(), String::new()),
    ]
  );
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
fn the_intrinsic_size_fold_joins_an_empty_part_from_a_terminated_comment_too() {
  // Same part shape as `auto_joins_an_empty_part_too` via a closed comment.
  assert_eq!(
    intrinsic_size("auto /**/"),
    ("auto ".to_string(), "auto ".to_string())
  );
  assert_eq!(
    intrinsic_size("/**/ 1px"),
    (String::new(), "1px".to_string())
  );
}

#[test]
fn the_intrinsic_size_fold_counts_an_empty_part_toward_the_arity() {
  // A fold that skipped the empty part would size both axes `auto 1px`.
  assert_eq!(
    intrinsic_size("auto 1px /**/"),
    ("auto 1px".to_string(), String::new())
  );
}

#[test]
fn list_style_lets_an_empty_part_take_the_slot_it_landed_in() {
  // An empty part is not global, position or type (upstream's `+`), so it lands at the image site.
  let func = match Shorthands::get("listStyle") {
    Some(func) => func,
    None => panic!("expected \"listStyle\" to be a known shorthand"),
  };

  let pairs = match func(Some("/**/".into())) {
    Ok(pairs) => pairs,
    Err(err) => panic!("expected listStyle \"/**/\" to expand, got {err:?}"),
  };
  assert_eq!(
    pairs,
    vec![
      OrderPair("listStyleType".into(), None),
      OrderPair("listStylePosition".into(), None),
      OrderPair("listStyleImage".into(), Some("".into())),
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
