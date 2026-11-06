use crate::order::constants::property_specificity_order::{Aliases, Shorthands};
use crate::order::tests::{DEPRECATED_BORDER_ALIASES, REJECTING_SHORTHANDS};
use stylex_structures::order_pair::OrderPair;

// ── Shorthands::get ─────────────────────────────────────────────────

#[test]
fn shorthands_get_rejects_every_name_it_registers() {
  // Both value arms: the rejection is about the property name, so a registered
  // name must refuse whether or not a value came with it.
  for name in REJECTING_SHORTHANDS {
    let func = Shorthands::get(name).unwrap_or_else(|| panic!("'{name}' should be registered"));

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
fn shorthands_get_unknown_returns_none() {
  assert!(Shorthands::get("nonexistent").is_none());
}

#[test]
fn shorthands_get_empty_returns_none() {
  assert!(Shorthands::get("").is_none());
}

#[test]
fn shorthands_get_special_chars_returns_none() {
  assert!(Shorthands::get("@!!").is_none());
}

// ── Aliases::get ────────────────────────────────────────────────────

#[test]
fn aliases_get_block_size() {
  let func = Aliases::get("blockSize").unwrap();
  let result = func(Some("100px".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("height".into(), Some("100px".into()))]
  );
}

#[test]
fn aliases_get_inline_size() {
  let func = Aliases::get("inlineSize").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("width".into(), None)]);
}

#[test]
fn aliases_get_min_block_size() {
  let func = Aliases::get("minBlockSize").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("minHeight".into(), None)]);
}

#[test]
fn aliases_get_max_inline_size() {
  let func = Aliases::get("maxInlineSize").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("maxWidth".into(), None)]);
}

#[test]
fn aliases_get_border_horizontal_width() {
  let func = Aliases::get("borderHorizontalWidth").unwrap();
  let result = func(Some("1px".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("borderInlineWidth".into(), Some("1px".into()))]
  );
}

#[test]
fn aliases_get_border_horizontal_style() {
  let func = Aliases::get("borderHorizontalStyle").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("borderInlineStyle".into(), None)]);
}

#[test]
fn aliases_get_border_horizontal_color() {
  let func = Aliases::get("borderHorizontalColor").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("borderInlineColor".into(), None)]);
}

#[test]
fn aliases_get_border_vertical_width() {
  let func = Aliases::get("borderVerticalWidth").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("borderBlockWidth".into(), None)]);
}

#[test]
fn aliases_get_border_vertical_style() {
  let func = Aliases::get("borderVerticalStyle").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("borderBlockStyle".into(), None)]);
}

#[test]
fn aliases_get_border_vertical_color() {
  let func = Aliases::get("borderVerticalColor").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("borderBlockColor".into(), None)]);
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
fn aliases_get_border_block_end_style() {
  let func = Aliases::get("borderBlockEndStyle").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("borderBottomStyle".into(), None)]);
}

#[test]
fn aliases_get_border_start_color() {
  let func = Aliases::get("borderStartColor").unwrap();
  let result = func(None).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("borderInlineStartColor".into(), None)]
  );
}

#[test]
fn aliases_get_border_end_width() {
  let func = Aliases::get("borderEndWidth").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("borderInlineEndWidth".into(), None)]);
}

#[test]
fn aliases_get_border_top_start_radius() {
  let func = Aliases::get("borderTopStartRadius").unwrap();
  let result = func(None).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("borderStartStartRadius".into(), None)]
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
fn aliases_get_margin_block_start() {
  let func = Aliases::get("marginBlockStart").unwrap();
  let result = func(Some("8px".into())).unwrap();
  assert_eq!(
    result,
    vec![OrderPair("marginTop".into(), Some("8px".into()))]
  );
}

#[test]
fn aliases_get_margin_block_end() {
  let func = Aliases::get("marginBlockEnd").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("marginBottom".into(), None)]);
}

#[test]
fn aliases_get_margin_start() {
  let func = Aliases::get("marginStart").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("marginInlineStart".into(), None)]);
}

#[test]
fn aliases_get_margin_end() {
  let func = Aliases::get("marginEnd").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("marginInlineEnd".into(), None)]);
}

#[test]
fn aliases_get_margin_horizontal() {
  let func = Aliases::get("marginHorizontal").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("marginInline".into(), None)]);
}

#[test]
fn aliases_get_margin_vertical() {
  let func = Aliases::get("marginVertical").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("marginBlock".into(), None)]);
}

#[test]
fn aliases_get_overflow_block() {
  let func = Aliases::get("overflowBlock").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("overflowY".into(), None)]);
}

#[test]
fn aliases_get_overflow_inline() {
  let func = Aliases::get("overflowInline").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("overflowX".into(), None)]);
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
fn aliases_get_padding_start() {
  let func = Aliases::get("paddingStart").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("paddingInlineStart".into(), None)]);
}

#[test]
fn aliases_get_padding_end() {
  let func = Aliases::get("paddingEnd").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("paddingInlineEnd".into(), None)]);
}

#[test]
fn aliases_get_padding_horizontal() {
  let func = Aliases::get("paddingHorizontal").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("paddingInline".into(), None)]);
}

#[test]
fn aliases_get_padding_vertical() {
  let func = Aliases::get("paddingVertical").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("paddingBlock".into(), None)]);
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
fn aliases_get_start() {
  let func = Aliases::get("start").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("insetInlineStart".into(), None)]);
}

#[test]
fn aliases_get_end() {
  let func = Aliases::get("end").unwrap();
  let result = func(None).unwrap();
  assert_eq!(result, vec![OrderPair("insetInlineEnd".into(), None)]);
}

// ── Deprecated aliases ──────────────────────────────────────────────

// ── Unknown ─────────────────────────────────────────────────────────

#[test]
fn aliases_get_unknown_returns_none() {
  assert!(Aliases::get("nonexistent").is_none());
}

#[test]
fn aliases_get_empty_returns_none() {
  assert!(Aliases::get("").is_none());
}

// ── Call all aliases with Some values ───────────────────────────────

#[test]
fn aliases_all_called_with_some_value() {
  let entries = vec![
    ("blockSize", "100px", "height"),
    ("inlineSize", "200px", "width"),
    ("minBlockSize", "50px", "minHeight"),
    ("minInlineSize", "50px", "minWidth"),
    ("maxBlockSize", "500px", "maxHeight"),
    ("maxInlineSize", "500px", "maxWidth"),
    ("borderHorizontalWidth", "1px", "borderInlineWidth"),
    ("borderHorizontalStyle", "solid", "borderInlineStyle"),
    ("borderHorizontalColor", "red", "borderInlineColor"),
    ("borderVerticalWidth", "2px", "borderBlockWidth"),
    ("borderVerticalStyle", "dashed", "borderBlockStyle"),
    ("borderVerticalColor", "blue", "borderBlockColor"),
    ("borderBlockStartColor", "red", "borderTopColor"),
    ("borderBlockEndColor", "blue", "borderBottomColor"),
    ("borderBlockStartStyle", "solid", "borderTopStyle"),
    ("borderBlockEndStyle", "dotted", "borderBottomStyle"),
    ("borderBlockStartWidth", "1px", "borderTopWidth"),
    ("borderBlockEndWidth", "2px", "borderBottomWidth"),
    ("borderStartColor", "red", "borderInlineStartColor"),
    ("borderEndColor", "blue", "borderInlineEndColor"),
    ("borderStartStyle", "solid", "borderInlineStartStyle"),
    ("borderEndStyle", "dotted", "borderInlineEndStyle"),
    ("borderStartWidth", "1px", "borderInlineStartWidth"),
    ("borderEndWidth", "2px", "borderInlineEndWidth"),
    ("borderTopStartRadius", "4px", "borderStartStartRadius"),
    ("borderTopEndRadius", "4px", "borderStartEndRadius"),
    ("borderBottomStartRadius", "4px", "borderEndStartRadius"),
    ("borderBottomEndRadius", "4px", "borderEndEndRadius"),
    (
      "containIntrinsicBlockSize",
      "auto 300px",
      "containIntrinsicHeight",
    ),
    (
      "containIntrinsicInlineSize",
      "200px",
      "containIntrinsicWidth",
    ),
    ("marginBlockStart", "10px", "marginTop"),
    ("marginBlockEnd", "20px", "marginBottom"),
    ("marginStart", "5px", "marginInlineStart"),
    ("marginEnd", "5px", "marginInlineEnd"),
    ("marginHorizontal", "10px", "marginInline"),
    ("marginVertical", "10px", "marginBlock"),
    ("overflowBlock", "scroll", "overflowY"),
    ("overflowInline", "hidden", "overflowX"),
    ("paddingBlockStart", "10px", "paddingTop"),
    ("paddingBlockEnd", "20px", "paddingBottom"),
    ("paddingStart", "5px", "paddingInlineStart"),
    ("paddingEnd", "5px", "paddingInlineEnd"),
    ("paddingHorizontal", "10px", "paddingInline"),
    ("paddingVertical", "10px", "paddingBlock"),
    ("scrollMarginBlockStart", "10px", "scrollMarginTop"),
    ("scrollMarginBlockEnd", "10px", "scrollMarginBottom"),
    ("insetBlockStart", "10px", "top"),
    ("insetBlockEnd", "10px", "bottom"),
    ("start", "10px", "insetInlineStart"),
    ("end", "10px", "insetInlineEnd"),
  ];

  for (alias_name, value, expected_key) in entries {
    let func = Aliases::get(alias_name)
      .unwrap_or_else(|| panic!("Alias '{}' should be registered", alias_name));
    let result = func(Some(value.into()))
      .unwrap_or_else(|e| panic!("Alias '{}' should return Ok, got: {}", alias_name, e));
    assert_eq!(
      result[0].0, expected_key,
      "Alias '{}' first pair key mismatch",
      alias_name
    );
    assert_eq!(
      result[0].1,
      Some(value.into()),
      "Alias '{}' first pair value mismatch",
      alias_name
    );
  }
}

// ── Deprecated aliases that delegate ────────────────────────────────

#[test]
fn aliases_deprecated_delegate_to_the_shorthand_they_alias() {
  // Each of these is an alias *of* another shorthand, so it must answer with
  // that shorthand's rejection rather than with its own name.
  for (alias_name, expected_prefix) in DEPRECATED_BORDER_ALIASES {
    let func = Aliases::get(alias_name)
      .unwrap_or_else(|| panic!("alias '{alias_name}' should be registered"));

    match func(Some("1px solid red".into())) {
      Ok(pairs) => panic!("alias '{alias_name}' should reject, expanded to {pairs:?}"),
      Err(error) => assert!(
        error.starts_with(expected_prefix),
        "alias '{alias_name}' reported {error:?}"
      ),
    }
  }
}

// ── Message text, byte for byte against upstream ─────────────────────

#[test]
fn every_rejection_message_matches_upstream_exactly() {
  // Nine of these had never been reachable before the table was re-keyed, so
  // nothing had ever read them: all nine were missing upstream's trailing
  // period, and `borderLeft` ran its three sentences together because it
  // concatenated where upstream joins with a space. The harness compares
  // outcomes and not messages, so only this test holds the text.
  //
  // Source: `@stylexjs/babel-plugin@0.19.0`,
  // `src/shared/preprocess-rules/property-specificity.js`.
  let expected = [
    ("all", "all is not supported"),
    ("animation", "animation is not supported"),
    (
      "background",
      "background is not supported. Use background-color, border-image etc. instead.",
    ),
    (
      "border",
      "border is not supported. Use border-width, border-style and border-color instead.",
    ),
    (
      "borderInline",
      "borderInline is not supported. Use borderInlineWidth, borderInlineStyle and borderInlineColor instead.",
    ),
    (
      "borderBlock",
      "borderBlock is not supported. Use borderBlockWidth, borderBlockStyle and borderBlockColor instead.",
    ),
    (
      "borderTop",
      "borderTop is not supported. Use borderTopWidth, borderTopStyle and borderTopColor instead.",
    ),
    (
      "borderInlineEnd",
      "borderInlineEnd is not supported. Use borderInlineEndWidth, borderInlineEndStyle and borderInlineEndColor instead.",
    ),
    (
      "borderRight",
      "borderRight is not supported. Use borderRightWidth, borderRightStyle and borderRightColor instead.",
    ),
    (
      "borderBottom",
      "borderBottom is not supported. Use borderBottomWidth, borderBottomStyle and borderBottomColor instead.",
    ),
    (
      "borderInlineStart",
      "borderInlineStart is not supported. Use borderInlineStartWidth, borderInlineStartStyle and borderInlineStartColor instead.",
    ),
    (
      "borderLeft",
      "`borderLeft` is not supported. You could use `borderLeftWidth`, `borderLeftStyle` and `borderLeftColor`, but it is preferable to use `borderInlineStartWidth`, `borderInlineStartStyle` and `borderInlineStartColor`.",
    ),
  ];

  assert_eq!(
    expected.len(),
    REJECTING_SHORTHANDS.len(),
    "a shorthand was added or removed without its message being pinned"
  );

  for (name, message) in expected {
    let func = Shorthands::get(name).unwrap_or_else(|| panic!("'{name}' should be registered"));

    match func(Some("1px solid red".into())) {
      Ok(pairs) => panic!("'{name}' should reject, expanded to {pairs:?}"),
      Err(error) => assert_eq!(error, message, "'{name}' message drifted"),
    }
  }
}
