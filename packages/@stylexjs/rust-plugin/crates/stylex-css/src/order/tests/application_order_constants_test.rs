use crate::order::constants::application_order::{Aliases, Shorthands};
use stylex_structures::order_pair::OrderPair;

// ── Shorthands::get ─────────────────────────────────────────────────

#[test]
fn shorthands_get_all_returns_err() {
  let func = Shorthands::get("all").expect("all should be registered");
  let result = func(None);
  assert!(result.is_err());
  assert!(result.unwrap_err().contains("not supported"));
}

#[test]
fn shorthands_get_animation_returns_ok() {
  let func = Shorthands::get("animation").expect("animation should be registered");
  let result = func(None).expect("should expand animation");
  // animation + animationRange sub-expansion
  assert!(result.len() > 10);
  assert_eq!(result[0].0, "animation");
}

#[test]
fn shorthands_get_animation_with_value() {
  let func = Shorthands::get("animation").unwrap();
  let result = func(Some("fadeIn 1s".into())).unwrap();
  assert_eq!(
    result[0],
    OrderPair("animation".into(), Some("fadeIn 1s".into()))
  );
  // All sub-properties should be None
  assert_eq!(result[1].1, None);
}

#[test]
fn shorthands_get_animation_range() {
  let func = Shorthands::get("animationRange").expect("animationRange should be registered");
  let result = func(None).unwrap();
  assert_eq!(result.len(), 3);
  assert_eq!(result[0].0, "animationRange");
}

#[test]
fn shorthands_get_background_returns_ok() {
  let func = Shorthands::get("background").unwrap();
  let result = func(None).unwrap();
  // background + sub-properties + backgroundPosition sub-expansion
  assert!(result.len() >= 10);
  assert_eq!(result[0].0, "background");
}

#[test]
fn shorthands_get_background_position() {
  let func = Shorthands::get("backgroundPosition").unwrap();
  let result = func(Some("center".into())).unwrap();
  assert_eq!(result.len(), 3);
  assert_eq!(
    result[0],
    OrderPair("backgroundPosition".into(), Some("center".into()))
  );
}

#[test]
fn shorthands_get_border_expands() {
  let func = Shorthands::get("border").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() > 5);
  assert_eq!(result[0].0, "border");
}

#[test]
fn shorthands_get_border_inline() {
  let func = Shorthands::get("borderInline").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() > 3);
  assert_eq!(result[0].0, "borderInline");
}

#[test]
fn shorthands_get_border_block() {
  let func = Shorthands::get("borderBlock").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() > 3);
}

#[test]
fn shorthands_get_border_top() {
  let func = Shorthands::get("borderTop").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
  assert_eq!(result[0].0, "borderTop");
}

#[test]
fn shorthands_get_border_inline_end() {
  let func = Shorthands::get("borderInlineEnd").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
}

#[test]
fn shorthands_get_border_right() {
  let func = Shorthands::get("borderRight").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
}

#[test]
fn shorthands_get_border_bottom() {
  let func = Shorthands::get("borderBottom").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
}

#[test]
fn shorthands_get_border_inline_start() {
  let func = Shorthands::get("borderInlineStart").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
}

#[test]
fn shorthands_get_border_left() {
  let func = Shorthands::get("borderLeft").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
}

#[test]
fn shorthands_get_border_radius() {
  let func = Shorthands::get("borderRadius").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
}

#[test]
fn shorthands_get_border_color() {
  let func = Shorthands::get("borderColor").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
}

#[test]
fn shorthands_get_border_style() {
  let func = Shorthands::get("borderStyle").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
}

#[test]
fn shorthands_get_border_width() {
  let func = Shorthands::get("borderWidth").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
}

#[test]
fn shorthands_get_border_inline_width() {
  let func = Shorthands::get("borderInlineWidth").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_corner_shape() {
  let func = Shorthands::get("cornerShape").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
}

#[test]
fn shorthands_get_flex() {
  let func = Shorthands::get("flex").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 3);
}

#[test]
fn shorthands_get_flex_flow() {
  let func = Shorthands::get("flexFlow").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_font() {
  let func = Shorthands::get("font").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() > 3);
}

#[test]
fn shorthands_get_gap() {
  let func = Shorthands::get("gap").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_grid() {
  let func = Shorthands::get("grid").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() > 3);
}

#[test]
fn shorthands_get_grid_area() {
  let func = Shorthands::get("gridArea").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_inset() {
  let func = Shorthands::get("inset").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
}

#[test]
fn shorthands_get_inset_inline() {
  let func = Shorthands::get("insetInline").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_inset_block() {
  let func = Shorthands::get("insetBlock").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_left_right() {
  let left_fn = Shorthands::get("left").unwrap();
  let right_fn = Shorthands::get("right").unwrap();
  assert!(left_fn(None).unwrap().len() >= 2);
  assert!(right_fn(None).unwrap().len() >= 2);
}

#[test]
fn shorthands_get_list_style() {
  let func = Shorthands::get("listStyle").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 3);
}

#[test]
fn shorthands_get_margin() {
  let func = Shorthands::get("margin").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
}

#[test]
fn shorthands_get_margin_inline() {
  let func = Shorthands::get("marginInline").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_margin_block() {
  let func = Shorthands::get("marginBlock").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_overflow() {
  let func = Shorthands::get("overflow").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_padding() {
  let func = Shorthands::get("padding").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
}

#[test]
fn shorthands_get_padding_inline() {
  let func = Shorthands::get("paddingInline").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_scroll_margin() {
  let func = Shorthands::get("scrollMargin").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
}

#[test]
fn shorthands_get_scroll_padding() {
  let func = Shorthands::get("scrollPadding").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
}

#[test]
fn shorthands_get_text_decoration() {
  let func = Shorthands::get("textDecoration").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 3);
}

#[test]
fn shorthands_get_transition() {
  let func = Shorthands::get("transition").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 3);
}

#[test]
fn shorthands_get_place_content() {
  let func = Shorthands::get("placeContent").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_place_items() {
  let func = Shorthands::get("placeItems").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_place_self() {
  let func = Shorthands::get("placeSelf").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_mask() {
  let func = Shorthands::get("mask").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 3);
}

#[test]
fn shorthands_get_outline() {
  let func = Shorthands::get("outline").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 3);
}

#[test]
fn shorthands_get_columns() {
  let func = Shorthands::get("columns").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_container() {
  let func = Shorthands::get("container").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_scroll_snap_type() {
  let func = Shorthands::get("scrollSnapType").unwrap();
  let result = func(None).unwrap();
  assert!(!result.is_empty());
}

#[test]
fn shorthands_get_border_image() {
  let func = Shorthands::get("borderImage").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 3);
}

#[test]
fn shorthands_get_column_rule() {
  let func = Shorthands::get("columnRule").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 3);
}

#[test]
fn shorthands_get_contain_intrinsic_size() {
  let func = Shorthands::get("containIntrinsicSize").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_font_variant() {
  let func = Shorthands::get("fontVariant").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 3);
}

#[test]
fn shorthands_get_grid_row() {
  let func = Shorthands::get("gridRow").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_grid_column() {
  let func = Shorthands::get("gridColumn").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_grid_template() {
  let func = Shorthands::get("gridTemplate").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 3);
}

#[test]
fn shorthands_get_offset() {
  let func = Shorthands::get("offset").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 3);
}

#[test]
fn shorthands_get_text_emphasis() {
  let func = Shorthands::get("textEmphasis").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_mask_border() {
  let func = Shorthands::get("maskBorder").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 3);
}

#[test]
fn shorthands_get_scroll_timeline() {
  let func = Shorthands::get("scrollTimeline").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

// ── Shorthands with values ──────────────────────────────────────────

#[test]
fn shorthands_border_color_with_value() {
  let func = Shorthands::get("borderColor").unwrap();
  let result = func(Some("red".into())).unwrap();
  // Expands to top/right/bottom/left and their LR-specific sub-expansions
  assert!(result.len() >= 4);
}

#[test]
fn shorthands_margin_with_value() {
  let func = Shorthands::get("margin").unwrap();
  let result = func(Some("10px 20px".into())).unwrap();
  assert!(result.len() >= 4);
}

#[test]
fn shorthands_padding_with_value() {
  let func = Shorthands::get("padding").unwrap();
  let result = func(Some("5px".into())).unwrap();
  assert!(result.len() >= 4);
}

#[test]
fn shorthands_overflow_with_value() {
  let func = Shorthands::get("overflow").unwrap();
  let result = func(Some("hidden scroll".into())).unwrap();
  assert!(result.len() >= 2);
}

// ── LR-specific shorthands ──────────────────────────────────────────

#[test]
fn shorthands_get_inset_inline_start() {
  let func = Shorthands::get("insetInlineStart").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_inset_inline_end() {
  let func = Shorthands::get("insetInlineEnd").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_margin_inline_start() {
  let func = Shorthands::get("marginInlineStart").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_margin_inline_end() {
  let func = Shorthands::get("marginInlineEnd").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_margin_left_right() {
  let left_fn = Shorthands::get("marginLeft").unwrap();
  let right_fn = Shorthands::get("marginRight").unwrap();
  assert!(left_fn(None).unwrap().len() >= 2);
  assert!(right_fn(None).unwrap().len() >= 2);
}

#[test]
fn shorthands_get_padding_inline_start() {
  let func = Shorthands::get("paddingInlineStart").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_padding_inline_end() {
  let func = Shorthands::get("paddingInlineEnd").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_padding_left_right() {
  let left_fn = Shorthands::get("paddingLeft").unwrap();
  let right_fn = Shorthands::get("paddingRight").unwrap();
  assert!(left_fn(None).unwrap().len() >= 2);
  assert!(right_fn(None).unwrap().len() >= 2);
}

#[test]
fn shorthands_get_scroll_margin_inline() {
  let func = Shorthands::get("scrollMarginInline").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_scroll_margin_block() {
  let func = Shorthands::get("scrollMarginBlock").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_scroll_padding_inline() {
  let func = Shorthands::get("scrollPaddingInline").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_scroll_padding_block() {
  let func = Shorthands::get("scrollPaddingBlock").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_padding_block() {
  let func = Shorthands::get("paddingBlock").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_border_block_width() {
  let func = Shorthands::get("borderBlockWidth").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_border_block_style() {
  let func = Shorthands::get("borderBlockStyle").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_border_block_color() {
  let func = Shorthands::get("borderBlockColor").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_border_inline_style() {
  let func = Shorthands::get("borderInlineStyle").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_border_inline_color() {
  let func = Shorthands::get("borderInlineColor").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

// ── Border LR shorthands ────────────────────────────────────────────

#[test]
fn shorthands_get_border_inline_start_color() {
  let func = Shorthands::get("borderInlineStartColor").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_border_inline_end_color() {
  let func = Shorthands::get("borderInlineEndColor").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn shorthands_get_border_left_right_color() {
  let left_fn = Shorthands::get("borderLeftColor").unwrap();
  let right_fn = Shorthands::get("borderRightColor").unwrap();
  assert!(left_fn(None).unwrap().len() >= 2);
  assert!(right_fn(None).unwrap().len() >= 2);
}

#[test]
fn shorthands_get_border_start_end_radius() {
  let ss = Shorthands::get("borderStartStartRadius").unwrap();
  let se = Shorthands::get("borderStartEndRadius").unwrap();
  let es = Shorthands::get("borderEndStartRadius").unwrap();
  let ee = Shorthands::get("borderEndEndRadius").unwrap();
  assert!(ss(None).unwrap().len() >= 2);
  assert!(se(None).unwrap().len() >= 2);
  assert!(es(None).unwrap().len() >= 2);
  assert!(ee(None).unwrap().len() >= 2);
}

#[test]
fn shorthands_get_border_top_left_right_radius() {
  let tl = Shorthands::get("borderTopLeftRadius").unwrap();
  let tr = Shorthands::get("borderTopRightRadius").unwrap();
  let bl = Shorthands::get("borderBottomLeftRadius").unwrap();
  let br = Shorthands::get("borderBottomRightRadius").unwrap();
  assert!(tl(None).unwrap().len() >= 2);
  assert!(tr(None).unwrap().len() >= 2);
  assert!(bl(None).unwrap().len() >= 2);
  assert!(br(None).unwrap().len() >= 2);
}

#[test]
fn shorthands_get_corner_start_end_shapes() {
  for name in &[
    "cornerStartStartShape",
    "cornerStartEndShape",
    "cornerEndStartShape",
    "cornerEndEndShape",
    "cornerTopLeftShape",
    "cornerTopRightShape",
    "cornerBottomLeftShape",
    "cornerBottomRightShape",
  ] {
    let func = Shorthands::get(name).unwrap();
    assert!(func(None).unwrap().len() >= 2);
  }
}

#[test]
fn shorthands_get_scroll_margin_inline_start_end() {
  let start = Shorthands::get("scrollMarginInlineStart").unwrap();
  let end = Shorthands::get("scrollMarginInlineEnd").unwrap();
  assert!(start(None).unwrap().len() >= 2);
  assert!(end(None).unwrap().len() >= 2);
}

#[test]
fn shorthands_get_scroll_margin_left_right() {
  let left = Shorthands::get("scrollMarginLeft").unwrap();
  let right = Shorthands::get("scrollMarginRight").unwrap();
  assert!(left(None).unwrap().len() >= 2);
  assert!(right(None).unwrap().len() >= 2);
}

#[test]
fn shorthands_get_scroll_padding_inline_start_end() {
  let start = Shorthands::get("scrollPaddingInlineStart").unwrap();
  let end = Shorthands::get("scrollPaddingInlineEnd").unwrap();
  assert!(start(None).unwrap().len() >= 2);
  assert!(end(None).unwrap().len() >= 2);
}

#[test]
fn shorthands_get_scroll_padding_left_right() {
  let left = Shorthands::get("scrollPaddingLeft").unwrap();
  let right = Shorthands::get("scrollPaddingRight").unwrap();
  assert!(left(None).unwrap().len() >= 2);
  assert!(right(None).unwrap().len() >= 2);
}

#[test]
fn shorthands_get_border_inline_start_end_style() {
  let start = Shorthands::get("borderInlineStartStyle").unwrap();
  let end = Shorthands::get("borderInlineEndStyle").unwrap();
  assert!(start(None).unwrap().len() >= 2);
  assert!(end(None).unwrap().len() >= 2);
}

#[test]
fn shorthands_get_border_inline_start_end_width() {
  let start = Shorthands::get("borderInlineStartWidth").unwrap();
  let end = Shorthands::get("borderInlineEndWidth").unwrap();
  assert!(start(None).unwrap().len() >= 2);
  assert!(end(None).unwrap().len() >= 2);
}

#[test]
fn shorthands_get_border_left_right_style() {
  let left = Shorthands::get("borderLeftStyle").unwrap();
  let right = Shorthands::get("borderRightStyle").unwrap();
  assert!(left(None).unwrap().len() >= 2);
  assert!(right(None).unwrap().len() >= 2);
}

#[test]
fn shorthands_get_border_left_right_width() {
  let left = Shorthands::get("borderLeftWidth").unwrap();
  let right = Shorthands::get("borderRightWidth").unwrap();
  assert!(left(None).unwrap().len() >= 2);
  assert!(right(None).unwrap().len() >= 2);
}

// ── Unknown properties ──────────────────────────────────────────────

#[test]
fn shorthands_get_unknown_returns_none() {
  assert!(Shorthands::get("nonexistent").is_none());
}

#[test]
fn shorthands_get_empty_string_returns_none() {
  assert!(Shorthands::get("").is_none());
}

#[test]
fn shorthands_get_special_chars_returns_none() {
  assert!(Shorthands::get("!!!").is_none());
}

// ── Aliases::get ────────────────────────────────────────────────────

#[test]
fn aliases_get_block_size() {
  let func = Aliases::get("blockSize").expect("blockSize should be registered");
  let result = func(Some("100px".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("height".into(), Some("100px".into()))]
  );
}

#[test]
fn aliases_get_inline_size() {
  let func = Aliases::get("inlineSize").unwrap();
  let result = func(Some("200px".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("width".into(), Some("200px".into()))]
  );
}

#[test]
fn aliases_get_min_block_size() {
  let func = Aliases::get("minBlockSize").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("minHeight".into(), None)]);
}

#[test]
fn aliases_get_min_inline_size() {
  let func = Aliases::get("minInlineSize").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("minWidth".into(), None)]);
}

#[test]
fn aliases_get_max_block_size() {
  let func = Aliases::get("maxBlockSize").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("maxHeight".into(), None)]);
}

#[test]
fn aliases_get_max_inline_size() {
  let func = Aliases::get("maxInlineSize").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("maxWidth".into(), None)]);
}

#[test]
fn aliases_get_border_block_start_color() {
  let func = Aliases::get("borderBlockStartColor").unwrap();
  let result = func(Some("red".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("borderTopColor".into(), Some("red".into()))]
  );
}

#[test]
fn aliases_get_border_block_end_width() {
  let func = Aliases::get("borderBlockEndWidth").unwrap();
  let result = func(Some("2px".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("borderBottomWidth".into(), Some("2px".into()))]
  );
}

#[test]
fn aliases_get_border_top_start_radius() {
  let func = Aliases::get("borderTopStartRadius").unwrap();
  let result = func(Some("4px".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair(
      "borderStartStartRadius".into(),
      Some("4px".into())
    )]
  );
}

#[test]
fn aliases_get_border_bottom_end_radius() {
  let func = Aliases::get("borderBottomEndRadius").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("borderEndEndRadius".into(), None)]);
}

#[test]
fn aliases_get_contain_intrinsic_block_size() {
  let func = Aliases::get("containIntrinsicBlockSize").unwrap();
  let result = func(None).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("containIntrinsicHeight".into(), None)]
  );
}

#[test]
fn aliases_get_contain_intrinsic_inline_size() {
  let func = Aliases::get("containIntrinsicInlineSize").unwrap();
  let result = func(None).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("containIntrinsicWidth".into(), None)]
  );
}

#[test]
fn aliases_get_grid_gap_delegates_to_shorthands() {
  let func = Aliases::get("gridGap").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
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
fn aliases_get_margin_block_start() {
  let func = Aliases::get("marginBlockStart").unwrap();
  let result = func(Some("10px".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("marginTop".into(), Some("10px".into()))]
  );
}

#[test]
fn aliases_get_margin_block_end() {
  let func = Aliases::get("marginBlockEnd").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("marginBottom".into(), None)]);
}

#[test]
fn aliases_get_margin_start_delegates() {
  let func = Aliases::get("marginStart").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn aliases_get_margin_end_delegates() {
  let func = Aliases::get("marginEnd").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn aliases_get_margin_horizontal_delegates() {
  let func = Aliases::get("marginHorizontal").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn aliases_get_margin_vertical_delegates() {
  let func = Aliases::get("marginVertical").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn aliases_get_overflow_block() {
  let func = Aliases::get("overflowBlock").unwrap();
  let result = func(Some("scroll".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("overflowY".into(), Some("scroll".into()))]
  );
}

#[test]
fn aliases_get_overflow_inline() {
  let func = Aliases::get("overflowInline").unwrap();
  let result = func(Some("hidden".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("overflowX".into(), Some("hidden".into()))]
  );
}

#[test]
fn aliases_get_padding_block_start() {
  let func = Aliases::get("paddingBlockStart").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("paddingTop".into(), None)]);
}

#[test]
fn aliases_get_padding_block_end() {
  let func = Aliases::get("paddingBlockEnd").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("paddingBottom".into(), None)]);
}

#[test]
fn aliases_get_padding_start_delegates() {
  let func = Aliases::get("paddingStart").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn aliases_get_padding_end_delegates() {
  let func = Aliases::get("paddingEnd").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn aliases_get_padding_horizontal_delegates() {
  let func = Aliases::get("paddingHorizontal").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn aliases_get_padding_vertical_delegates() {
  let func = Aliases::get("paddingVertical").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn aliases_get_scroll_margin_block_start() {
  let func = Aliases::get("scrollMarginBlockStart").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("scrollMarginTop".into(), None)]);
}

#[test]
fn aliases_get_scroll_margin_block_end() {
  let func = Aliases::get("scrollMarginBlockEnd").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("scrollMarginBottom".into(), None)]);
}

#[test]
fn aliases_get_inset_block_start() {
  let func = Aliases::get("insetBlockStart").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("top".into(), None)]);
}

#[test]
fn aliases_get_inset_block_end() {
  let func = Aliases::get("insetBlockEnd").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("bottom".into(), None)]);
}

#[test]
fn aliases_get_start_delegates() {
  let func = Aliases::get("start").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn aliases_get_end_delegates() {
  let func = Aliases::get("end").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

// ── Deprecated aliases delegate to shorthands ───────────────────────

#[test]
fn aliases_get_border_horizontal_delegates() {
  let func = Aliases::get("borderHorizontal");
  // Delegates to Shorthands::get("borderHorizontal") which may or may not exist
  // in application_order. We just test the result is consistent.
  if let Some(func) = func {
    let _ = func(None);
  }
}

// ── Coverage: uncovered Aliases::get deprecated match arms ──────────

#[test]
fn aliases_get_border_vertical_delegates_to_border_block() {
  let func = Aliases::get("borderVertical").expect("borderVertical should be registered");
  let result = func(None).unwrap();
  assert!(result.len() > 3);
  // Delegates to Shorthands::get("borderBlock") - first element is "borderBlock"
  assert_eq!(result[0].0, "borderBlock");
}

#[test]
fn aliases_get_border_block_start_delegates_to_border_top() {
  let func = Aliases::get("borderBlockStart").expect("borderBlockStart should be registered");
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
  // Delegates to Shorthands::get("borderTop") - first element is "borderTop"
  assert_eq!(result[0].0, "borderTop");
}

#[test]
fn aliases_get_border_end_delegates_to_border_inline_end() {
  let func = Aliases::get("borderEnd").expect("borderEnd should be registered");
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
  // Delegates to Shorthands::get("borderInlineEnd")
  assert_eq!(result[0].0, "borderInlineEnd");
}

#[test]
fn aliases_get_border_block_end_delegates_to_border_bottom() {
  let func = Aliases::get("borderBlockEnd").expect("borderBlockEnd should be registered");
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
  // Delegates to Shorthands::get("borderBottom")
  assert_eq!(result[0].0, "borderBottom");
}

#[test]
fn aliases_get_border_start_delegates_to_border_inline_start() {
  let func = Aliases::get("borderStart").expect("borderStart should be registered");
  let result = func(None).unwrap();
  assert!(result.len() >= 4);
  // Delegates to Shorthands::get("borderInlineStart")
  assert_eq!(result[0].0, "borderInlineStart");
}

#[test]
fn aliases_get_border_horizontal_style_delegates_to_border_inline_style() {
  let func =
    Aliases::get("borderHorizontalStyle").expect("borderHorizontalStyle should be registered");
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
  // Delegates to Shorthands::get("borderInlineStyle")
  assert_eq!(result[0].0, "borderInlineStyle");
}

#[test]
fn aliases_get_border_horizontal_color_delegates_to_border_inline_color() {
  let func =
    Aliases::get("borderHorizontalColor").expect("borderHorizontalColor should be registered");
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
  // Delegates to Shorthands::get("borderInlineColor")
  assert_eq!(result[0].0, "borderInlineColor");
}

#[test]
fn aliases_get_border_vertical_style_delegates_to_border_block_style() {
  let func = Aliases::get("borderVerticalStyle").expect("borderVerticalStyle should be registered");
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
  // Delegates to Shorthands::get("borderBlockStyle")
  assert_eq!(result[0].0, "borderBlockStyle");
}

#[test]
fn aliases_get_border_vertical_color_delegates_to_border_block_color() {
  let func = Aliases::get("borderVerticalColor").expect("borderVerticalColor should be registered");
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
  // Delegates to Shorthands::get("borderBlockColor")
  assert_eq!(result[0].0, "borderBlockColor");
}

#[test]
fn aliases_get_border_start_style_delegates_to_border_inline_start_style() {
  let func = Aliases::get("borderStartStyle").expect("borderStartStyle should be registered");
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
  // Delegates to Shorthands::get("borderInlineStartStyle")
  assert_eq!(result[0].0, "borderInlineStartStyle");
}

#[test]
fn aliases_get_border_end_style_delegates_to_border_inline_end_style() {
  let func = Aliases::get("borderEndStyle").expect("borderEndStyle should be registered");
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
  // Delegates to Shorthands::get("borderInlineEndStyle")
  assert_eq!(result[0].0, "borderInlineEndStyle");
}

#[test]
fn aliases_get_border_start_width_delegates_to_border_inline_start_width() {
  let func = Aliases::get("borderStartWidth").expect("borderStartWidth should be registered");
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
  // Delegates to Shorthands::get("borderInlineStartWidth")
  assert_eq!(result[0].0, "borderInlineStartWidth");
}

#[test]
fn aliases_get_border_end_width_delegates_to_border_inline_end_width() {
  let func = Aliases::get("borderEndWidth").expect("borderEndWidth should be registered");
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
  // Delegates to Shorthands::get("borderInlineEndWidth")
  assert_eq!(result[0].0, "borderInlineEndWidth");
}

#[test]
fn aliases_get_border_horizontal_width_delegates() {
  let func = Aliases::get("borderHorizontalWidth").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn aliases_get_border_vertical_width_delegates() {
  let func = Aliases::get("borderVerticalWidth").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn aliases_get_border_start_color_delegates() {
  let func = Aliases::get("borderStartColor").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn aliases_get_border_end_color_delegates() {
  let func = Aliases::get("borderEndColor").unwrap();
  let result = func(None).unwrap();
  assert!(result.len() >= 2);
}

#[test]
fn aliases_get_unknown_returns_none() {
  assert!(Aliases::get("nonexistent").is_none());
}

#[test]
fn aliases_get_empty_returns_none() {
  assert!(Aliases::get("").is_none());
}

// ── Coverage: uncovered alias functions ─────────────────────────────

#[test]
fn aliases_get_border_block_end_color() {
  let func = Aliases::get("borderBlockEndColor").unwrap();
  let result = func(Some("blue".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("borderBottomColor".into(), Some("blue".into()))]
  );
}

#[test]
fn aliases_get_border_block_start_style() {
  let func = Aliases::get("borderBlockStartStyle").unwrap();
  let result = func(Some("solid".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("borderTopStyle".into(), Some("solid".into()))]
  );
}

#[test]
fn aliases_get_border_block_end_style() {
  let func = Aliases::get("borderBlockEndStyle").unwrap();
  let result = func(Some("dashed".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("borderBottomStyle".into(), Some("dashed".into()))]
  );
}

#[test]
fn aliases_get_border_block_start_width() {
  let func = Aliases::get("borderBlockStartWidth").unwrap();
  let result = func(Some("2px".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("borderTopWidth".into(), Some("2px".into()))]
  );
}

#[test]
fn aliases_get_border_top_end_radius() {
  let func = Aliases::get("borderTopEndRadius").unwrap();
  let result = func(Some("8px".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("borderStartEndRadius".into(), Some("8px".into()))]
  );
}

#[test]
fn aliases_get_border_bottom_start_radius() {
  let func = Aliases::get("borderBottomStartRadius").unwrap();
  let result = func(Some("6px".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("borderEndStartRadius".into(), Some("6px".into()))]
  );
}
