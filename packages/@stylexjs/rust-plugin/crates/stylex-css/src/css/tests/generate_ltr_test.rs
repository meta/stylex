#[cfg(test)]
mod generate_ltr_tests {
  use crate::css::generate_ltr::generate_ltr;
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

  // ── property_to_ltr path (default non-legacy) ─────────────────

  #[test]
  fn passthrough_regular_property() {
    let pair = Pair::new("color", "red");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.key, "color");
    assert_eq!(result.value, "red");
  }

  #[test]
  fn logical_property_mapped_to_ltr() {
    let pair = Pair::new("margin-start", "10px");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.key, "margin-left");
    assert_eq!(result.value, "10px");
  }

  #[test]
  fn logical_property_margin_end() {
    let pair = Pair::new("margin-end", "20px");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.key, "margin-right");
    assert_eq!(result.value, "20px");
  }

  #[test]
  fn padding_start_mapped_to_ltr() {
    let pair = Pair::new("padding-start", "5px");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.key, "padding-left");
    assert_eq!(result.value, "5px");
  }

  // ── background-position handling ──────────────────────────────

  #[test]
  fn background_position_start_becomes_left() {
    let pair = Pair::new("background-position", "start center");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.key, "background-position");
    assert_eq!(result.value, "left center");
  }

  #[test]
  fn background_position_end_becomes_right() {
    let pair = Pair::new("background-position", "end top");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.key, "background-position");
    assert_eq!(result.value, "right top");
  }

  #[test]
  fn background_position_no_logical_values() {
    let pair = Pair::new("background-position", "center bottom");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.value, "center bottom");
  }

  // ── float and clear ───────────────────────────────────────────

  #[test]
  fn float_start_becomes_left() {
    let pair = Pair::new("float", "start");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.value, "left");
  }

  #[test]
  fn float_end_becomes_right() {
    let pair = Pair::new("float", "end");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.value, "right");
  }

  #[test]
  fn float_left_stays_left() {
    let pair = Pair::new("float", "left");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.value, "left");
  }

  #[test]
  fn clear_start_becomes_left() {
    let pair = Pair::new("clear", "start");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.value, "left");
  }

  #[test]
  fn clear_end_becomes_right() {
    let pair = Pair::new("clear", "end");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.value, "right");
  }

  #[test]
  fn clear_both_unchanged() {
    let pair = Pair::new("clear", "both");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.value, "both");
  }

  // ── LegacyExpandShorthands without polyfill ───────────────────

  #[test]
  fn legacy_no_polyfill_passthrough() {
    let pair = Pair::new("margin-start", "10px");
    let result = generate_ltr(&pair, &legacy_options());
    // Without polyfill enabled, returns the pair as-is
    assert_eq!(result.key, "margin-start");
    assert_eq!(result.value, "10px");
  }

  // ── LegacyExpandShorthands with polyfill ──────────────────────

  #[test]
  fn legacy_polyfill_inline_property_mapped() {
    let pair = Pair::new("margin-inline-start", "10px");
    let result = generate_ltr(&pair, &legacy_logical_options());
    assert_eq!(result.key, "margin-left");
    assert_eq!(result.value, "10px");
  }

  #[test]
  fn legacy_polyfill_inline_end_mapped() {
    let pair = Pair::new("margin-inline-end", "10px");
    let result = generate_ltr(&pair, &legacy_logical_options());
    assert_eq!(result.key, "margin-right");
    assert_eq!(result.value, "10px");
  }

  #[test]
  fn legacy_polyfill_unknown_falls_through_to_property_to_ltr() {
    let pair = Pair::new("color", "red");
    let result = generate_ltr(&pair, &legacy_logical_options());
    assert_eq!(result.key, "color");
    assert_eq!(result.value, "red");
  }

  // ── inline-start / inline-end ─────────────────────────────────

  #[test]
  fn float_inline_start_becomes_left() {
    let pair = Pair::new("float", "inline-start");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.value, "left");
  }

  #[test]
  fn float_inline_end_becomes_right() {
    let pair = Pair::new("float", "inline-end");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.value, "right");
  }

  #[test]
  fn background_position_inset_inline_start() {
    let pair = Pair::new("background-position", "insetInlineStart center");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.value, "left center");
  }

  #[test]
  fn background_position_inset_inline_end() {
    let pair = Pair::new("background-position", "insetInlineEnd center");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.value, "right center");
  }

  // ── Corpus values the parity harness measured ─────────────────
  //
  // Four logical values the corpus holds that were asserted only through a
  // whole-transform case, where the rewrite can only be inferred from a class
  // name. Each expectation is what `@stylexjs/babel-plugin` emits for the same
  // declaration, read from a harness report.

  #[test]
  fn clear_inline_start_becomes_left() {
    let pair = Pair::new("clear", "inline-start");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.value, "left");
  }

  #[test]
  fn clear_inline_end_becomes_right() {
    let pair = Pair::new("clear", "inline-end");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.value, "right");
  }

  /// A logical keyword in the *second* position of a two-keyword value, which
  /// the `insetInlineStart center` cases above put in the first.
  #[test]
  fn background_position_maps_a_trailing_start() {
    let pair = Pair::new("background-position", "top start");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.value, "top left");
  }

  #[test]
  fn background_position_maps_a_trailing_end() {
    let pair = Pair::new("background-position", "top end");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.value, "top right");
  }

  // ── Unknown property not in PROPERTY_TO_LTR ───────────────────

  #[test]
  fn unknown_property_unchanged() {
    let pair = Pair::new("z-index", "10");
    let result = generate_ltr(&pair, &default_options());
    assert_eq!(result.key, "z-index");
    assert_eq!(result.value, "10");
  }
}
