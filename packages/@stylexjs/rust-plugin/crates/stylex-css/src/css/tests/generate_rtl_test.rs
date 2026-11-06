#[cfg(test)]
mod generate_rtl_tests {
  use crate::css::generate_rtl::generate_rtl;
  use stylex_enums::style_resolution::StyleResolution;
  use stylex_structures::{pair::Pair, stylex_state_options::StyleXStateOptions};

  fn default_options() -> StyleXStateOptions {
    StyleXStateOptions::default()
  }

  fn legacy_options() -> StyleXStateOptions {
    StyleXStateOptions::default().with_style_resolution(StyleResolution::LegacyExpandShorthands)
  }

  fn legacy_logical_options() -> StyleXStateOptions {
    StyleXStateOptions::default()
      .with_style_resolution(StyleResolution::LegacyExpandShorthands)
      .with_enable_logical_styles_polyfill(true)
  }

  fn flipping_options() -> StyleXStateOptions {
    let mut options = StyleXStateOptions::default();
    options.core.enable_legacy_value_flipping = true;
    options
  }

  // ── No RTL needed for non-logical properties ──────────────────

  #[test]
  fn no_rtl_for_regular_property() {
    let pair = Pair::new("color", "red");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_none());
  }

  #[test]
  fn no_rtl_for_width() {
    let pair = Pair::new("width", "100px");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_none());
  }

  // ── Logical properties produce RTL pairs ──────────────────────

  #[test]
  fn margin_start_to_rtl_margin_right() {
    let pair = Pair::new("margin-start", "10px");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    let rtl = result.unwrap();
    assert_eq!(rtl.key, "margin-right");
    assert_eq!(rtl.value, "10px");
  }

  #[test]
  fn margin_end_to_rtl_margin_left() {
    let pair = Pair::new("margin-end", "10px");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    let rtl = result.unwrap();
    assert_eq!(rtl.key, "margin-left");
  }

  #[test]
  fn padding_start_to_rtl() {
    let pair = Pair::new("padding-start", "5px");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    let rtl = result.unwrap();
    assert_eq!(rtl.key, "padding-right");
  }

  #[test]
  fn padding_end_to_rtl_padding_left() {
    let pair = Pair::new("padding-end", "5px");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().key, "padding-left");
  }

  // ── `start` / `end` as keys (propertyToRTL.start / .end) ──────

  #[test]
  fn start_key_becomes_right_key() {
    let pair = Pair::new("start", "10px");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    let rtl = result.unwrap();
    assert_eq!(rtl.key, "right");
    assert_eq!(rtl.value, "10px");
  }

  #[test]
  fn end_key_becomes_left_key() {
    let pair = Pair::new("end", "10px");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    let rtl = result.unwrap();
    assert_eq!(rtl.key, "left");
    assert_eq!(rtl.value, "10px");
  }

  // ── border logical longhands & radii (LOGICAL_TO_RTL) ─────────

  #[test]
  fn border_start_to_rtl_border_right() {
    let pair = Pair::new("border-start", "1px solid");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    let rtl = result.unwrap();
    assert_eq!(rtl.key, "border-right");
    assert_eq!(rtl.value, "1px solid");
  }

  #[test]
  fn border_end_to_rtl_border_left() {
    let pair = Pair::new("border-end", "1px solid");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().key, "border-left");
  }

  #[test]
  fn border_start_width_to_rtl_border_right_width() {
    let pair = Pair::new("border-start-width", "2px");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().key, "border-right-width");
  }

  #[test]
  fn border_end_color_to_rtl_border_left_color() {
    let pair = Pair::new("border-end-color", "red");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().key, "border-left-color");
  }

  #[test]
  fn border_start_style_to_rtl_border_right_style() {
    let pair = Pair::new("border-start-style", "dashed");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().key, "border-right-style");
  }

  #[test]
  fn border_top_start_radius_to_rtl_border_top_right_radius() {
    let pair = Pair::new("border-top-start-radius", "4px");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().key, "border-top-right-radius");
  }

  #[test]
  fn border_bottom_end_radius_to_rtl_border_bottom_left_radius() {
    let pair = Pair::new("border-bottom-end-radius", "4px");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().key, "border-bottom-left-radius");
  }

  // ── float / clear RTL conversion ──────────────────────────────

  #[test]
  fn float_start_becomes_right_in_rtl() {
    let pair = Pair::new("float", "start");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "right");
  }

  #[test]
  fn float_end_becomes_left_in_rtl() {
    let pair = Pair::new("float", "end");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "left");
  }

  #[test]
  fn clear_start_becomes_right_in_rtl() {
    let pair = Pair::new("clear", "start");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "right");
  }

  #[test]
  fn clear_end_becomes_left_in_rtl() {
    let pair = Pair::new("clear", "end");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "left");
  }

  #[test]
  fn float_left_no_rtl() {
    let pair = Pair::new("float", "left");
    let result = generate_rtl(&pair, &default_options());
    // Physical values have no logical mapping, so no `rtl` rule is emitted.
    assert!(result.is_none());
  }

  #[test]
  fn clear_both_no_rtl() {
    let pair = Pair::new("clear", "both");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_none());
  }

  // ── float inline-start / inline-end ───────────────────────────

  #[test]
  fn float_inline_start_becomes_right_in_rtl() {
    let pair = Pair::new("float", "inline-start");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "right");
  }

  #[test]
  fn float_inline_end_becomes_left_in_rtl() {
    let pair = Pair::new("float", "inline-end");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "left");
  }

  // ── background-position RTL ───────────────────────────────────

  #[test]
  fn background_position_start_becomes_right() {
    let pair = Pair::new("background-position", "start center");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    let rtl = result.unwrap();
    assert_eq!(rtl.value, "right center");
  }

  #[test]
  fn background_position_end_becomes_left() {
    let pair = Pair::new("background-position", "end top");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    let rtl = result.unwrap();
    assert_eq!(rtl.value, "left top");
  }

  #[test]
  fn background_position_start_with_inset_inline_end_flips() {
    let pair = Pair::new("background-position", "start insetInlineEnd");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "right left");
  }

  #[test]
  fn background_position_end_with_inset_inline_start_flips() {
    let pair = Pair::new("background-position", "end insetInlineStart");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "left right");
  }

  #[test]
  fn background_position_no_logical_no_rtl() {
    let pair = Pair::new("background-position", "center bottom");
    let result = generate_rtl(&pair, &default_options());
    // No `start`/`end` keyword → no directional flip → no `rtl` rule
    // (avoids a redundant, higher-specificity duplicate).
    assert!(result.is_none());
  }

  // ── LegacyExpandShorthands without polyfill ───────────────────

  #[test]
  fn legacy_no_polyfill_returns_none() {
    let pair = Pair::new("margin-start", "10px");
    let result = generate_rtl(&pair, &legacy_options());
    assert!(result.is_none());
  }

  // ── LegacyExpandShorthands with polyfill ──────────────────────

  #[test]
  fn legacy_polyfill_inline_start_mapped() {
    let pair = Pair::new("margin-inline-start", "10px");
    let result = generate_rtl(&pair, &legacy_logical_options());
    assert!(result.is_some());
    let rtl = result.unwrap();
    assert_eq!(rtl.key, "margin-right");
  }

  #[test]
  fn legacy_polyfill_inline_end_mapped() {
    let pair = Pair::new("margin-inline-end", "10px");
    let result = generate_rtl(&pair, &legacy_logical_options());
    assert!(result.is_some());
    let rtl = result.unwrap();
    assert_eq!(rtl.key, "margin-left");
  }

  #[test]
  fn legacy_polyfill_padding_inline_start_mapped() {
    let pair = Pair::new("padding-inline-start", "10px");
    let result = generate_rtl(&pair, &legacy_logical_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().key, "padding-right");
  }

  #[test]
  fn legacy_polyfill_border_inline_end_mapped() {
    let pair = Pair::new("border-inline-end", "1px solid");
    let result = generate_rtl(&pair, &legacy_logical_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().key, "border-left");
  }

  #[test]
  fn legacy_polyfill_border_start_start_radius_mapped() {
    let pair = Pair::new("border-start-start-radius", "4px");
    let result = generate_rtl(&pair, &legacy_logical_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().key, "border-top-right-radius");
  }

  #[test]
  fn legacy_polyfill_inset_inline_start_becomes_right() {
    let pair = Pair::new("inset-inline-start", "0");
    let result = generate_rtl(&pair, &legacy_logical_options());
    assert!(result.is_some());
    let rtl = result.unwrap();
    assert_eq!(rtl.key, "right");
    assert_eq!(rtl.value, "0");
  }

  #[test]
  fn legacy_polyfill_inset_inline_end_becomes_left() {
    let pair = Pair::new("inset-inline-end", "0");
    let result = generate_rtl(&pair, &legacy_logical_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().key, "left");
  }

  #[test]
  fn non_legacy_inline_property_falls_through_to_none() {
    // `margin-inline-start` is only mapped under the legacy polyfill; with the
    // default resolution it is not in `propertyToRTL`, so no `rtl` is emitted.
    let pair = Pair::new("margin-inline-start", "10px");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_none());
  }

  // ── box-shadow / text-shadow flipping ─────────────────────────

  #[test]
  fn box_shadow_flips_with_legacy_value_flipping() {
    let pair = Pair::new("box-shadow", "5px 5px 10px #000");
    let result = generate_rtl(&pair, &flipping_options());
    assert!(result.is_some());
    let rtl = result.unwrap();
    assert!(rtl.value.contains("-5px"));
  }

  #[test]
  fn text_shadow_flips_with_legacy_value_flipping() {
    let pair = Pair::new("text-shadow", "2px 2px 4px #000");
    let result = generate_rtl(&pair, &flipping_options());
    assert!(result.is_some());
    let rtl = result.unwrap();
    assert!(rtl.value.contains("-2px"));
  }

  #[test]
  fn box_shadow_no_flip_without_legacy() {
    let pair = Pair::new("box-shadow", "5px 5px 10px #000");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_none());
  }

  #[test]
  fn box_shadow_zero_offset_no_flip() {
    let pair = Pair::new("box-shadow", "0 0 10px #000");
    let result = generate_rtl(&pair, &flipping_options());
    // 0 stays 0, no actual flip needed
    assert!(result.is_none());
  }

  #[test]
  fn box_shadow_negative_becomes_positive() {
    let pair = Pair::new("box-shadow", "-3px 3px 5px #000");
    let result = generate_rtl(&pair, &flipping_options());
    assert!(result.is_some());
    let rtl = result.unwrap();
    assert!(rtl.value.contains("3px 3px"));
  }

  #[test]
  fn box_shadow_multiple_shadows() {
    let pair = Pair::new("box-shadow", "2px 0 4px #000, -1px 0 2px #fff");
    let result = generate_rtl(&pair, &flipping_options());
    assert!(result.is_some());
    let rtl = result.unwrap();
    assert!(rtl.value.contains("-2px"));
    assert!(rtl.value.contains("1px"));
  }

  // ── cursor flipping ───────────────────────────────────────────

  #[test]
  fn cursor_no_flip_without_legacy() {
    let pair = Pair::new("cursor", "e-resize");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_none());
  }

  #[test]
  fn cursor_no_flip_for_unknown_value() {
    let pair = Pair::new("cursor", "pointer");
    let result = generate_rtl(&pair, &flipping_options());
    // "pointer" is not in CURSOR_FLIP
    assert!(result.is_none());
  }

  // ── other properties return None ──────────────────────────────

  #[test]
  fn non_shadow_non_logical_returns_none() {
    let pair = Pair::new("z-index", "10");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_none());
  }

  #[test]
  fn display_returns_none() {
    let pair = Pair::new("display", "flex");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_none());
  }

  // ── Additional coverage for remaining branches ──────────────────

  #[test]
  fn cursor_e_resize_flips_with_legacy() {
    let pair = Pair::new("cursor", "e-resize");
    let result = generate_rtl(&pair, &flipping_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "w-resize");
  }

  #[test]
  fn cursor_w_resize_flips_with_legacy() {
    let pair = Pair::new("cursor", "w-resize");
    let result = generate_rtl(&pair, &flipping_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "e-resize");
  }

  #[test]
  fn cursor_ne_resize_flips_with_legacy() {
    let pair = Pair::new("cursor", "ne-resize");
    let result = generate_rtl(&pair, &flipping_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "nw-resize");
  }

  #[test]
  fn cursor_se_resize_flips_with_legacy() {
    let pair = Pair::new("cursor", "se-resize");
    let result = generate_rtl(&pair, &flipping_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "sw-resize");
  }

  #[test]
  fn cursor_nw_resize_flips_with_legacy() {
    let pair = Pair::new("cursor", "nw-resize");
    let result = generate_rtl(&pair, &flipping_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "ne-resize");
  }

  #[test]
  fn cursor_sw_resize_flips_with_legacy() {
    let pair = Pair::new("cursor", "sw-resize");
    let result = generate_rtl(&pair, &flipping_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "se-resize");
  }

  #[test]
  fn cursor_nesw_resize_flips_with_legacy() {
    let pair = Pair::new("cursor", "nesw-resize");
    let result = generate_rtl(&pair, &flipping_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "nwse-resize");
  }

  #[test]
  fn cursor_nwse_resize_flips_with_legacy() {
    let pair = Pair::new("cursor", "nwse-resize");
    let result = generate_rtl(&pair, &flipping_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "nesw-resize");
  }

  #[test]
  fn background_position_inset_inline_start_no_rtl() {
    let pair = Pair::new("background-position", "inset-inline-start center");
    let result = generate_rtl(&pair, &default_options());
    // The guard only matches the bare `start`/`end` keywords, so
    // `inset-inline-*` values produce no `rtl` rule.
    assert!(result.is_none());
  }

  #[test]
  fn background_position_inset_inline_end_no_rtl() {
    let pair = Pair::new("background-position", "inset-inline-end top");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_none());
  }

  #[test]
  fn clear_inline_start_becomes_right() {
    let pair = Pair::new("clear", "inline-start");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "right");
  }

  #[test]
  fn clear_inline_end_becomes_left() {
    let pair = Pair::new("clear", "inline-end");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "left");
  }

  #[test]
  fn text_shadow_no_flip_without_legacy() {
    let pair = Pair::new("text-shadow", "2px 2px #000");
    let result = generate_rtl(&pair, &default_options());
    assert!(result.is_none());
  }

  #[test]
  fn box_shadow_inset_flips() {
    let pair = Pair::new("box-shadow", "inset 5px 5px 10px #000");
    let result = generate_rtl(&pair, &flipping_options());
    assert!(result.is_some());
    let rtl = result.unwrap();
    assert!(rtl.value.contains("-5px") || rtl.value.contains("inset"));
  }

  #[test]
  fn legacy_polyfill_non_inline_key_falls_through() {
    let pair = Pair::new("float", "start");
    let result = generate_rtl(&pair, &legacy_logical_options());
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "right");
  }

  #[test]
  fn legacy_no_polyfill_regular_property_returns_none() {
    let pair = Pair::new("color", "red");
    let result = generate_rtl(&pair, &legacy_options());
    assert!(result.is_none());
  }

  /// The `else { 1 }` branch in `flip_shadow` is taken when `parts[0]`
  /// is NOT a CSS unit value (e.g. a keyword like "inset" or a color).
  /// This exercises `index = 1` and flips `parts[1]` instead of `parts[0]`.
  #[test]
  fn box_shadow_keyword_first_part_uses_index_one() {
    // "inset 3px 2px 4px #000" → parts[0] = "inset" (not a unit)
    // → index = 1 → flip parts[1] = "3px" → "-3px"
    let pair = Pair::new("box-shadow", "inset 3px 2px 4px #000");
    let result = generate_rtl(&pair, &flipping_options());
    assert!(result.is_some());
    let rtl = result.unwrap();
    assert!(
      rtl.value.contains("-3px"),
      "expected -3px in: {}",
      rtl.value
    );
  }

  /// When a comma-separated shadow segment has a single non-unit token
  /// (e.g. just a color), `index = 1` but `parts.len() == 1`, so the
  /// flip is skipped — exercising the `index >= parts.len()` false branch.
  #[test]
  fn box_shadow_single_color_segment_skips_flip() {
    let pair = Pair::new("box-shadow", "3px 3px #000, #fff");
    let result = generate_rtl(&pair, &flipping_options());
    assert!(result.is_some());
    let rtl = result.unwrap();
    // The second segment "#fff" has only one non-unit token so it is unchanged
    assert!(
      rtl.value.contains("#fff"),
      "expected #fff in: {}",
      rtl.value
    );
  }
}
