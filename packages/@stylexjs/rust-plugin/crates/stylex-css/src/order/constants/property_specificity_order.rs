use stylex_structures::{order_pair::OrderPair, raw_value::TRawValue};

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Shorthands;

impl Shorthands {
  fn all(_: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Err("all is not supported".into())
  }
  fn animation(_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Err("animation is not supported".into())
  }
  fn background(_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Err("background is not supported. Use background-color, border-image etc. instead.".into())
  }
  fn border(_raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Err("border is not supported. Use border-width, border-style and border-color instead.".into())
  }
  fn border_inline(_raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Err(
      "borderInline is not supported. Use borderInlineWidth, borderInlineStyle and borderInlineColor instead.".into(),
    )
  }
  // @Deprecated
  fn border_block(_raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Err(
      "borderBlock is not supported. Use borderBlockWidth, borderBlockStyle and borderBlockColor instead.".into(),
    )
  }

  // @Deprecated
  fn border_top(_raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Err(
      "borderTop is not supported. Use borderTopWidth, borderTopStyle and borderTopColor instead."
        .into(),
    )
  }
  // @Deprecated
  fn border_inline_end(_raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Err(
      "borderInlineEnd is not supported. Use borderInlineEndWidth, borderInlineEndStyle and borderInlineEndColor instead.".into(),
    )
  }
  // @Deprecated
  fn border_right(_raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Err(
      "borderRight is not supported. Use borderRightWidth, borderRightStyle and borderRightColor instead.".into(),
    )
  }
  // @Deprecated
  fn border_bottom(_raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Err(
      "borderBottom is not supported. Use borderBottomWidth, borderBottomStyle and borderBottomColor instead.".into(),
    )
  }
  // @Deprecated
  fn border_inline_start(_raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Err(
      "borderInlineStart is not supported. Use borderInlineStartWidth, borderInlineStartStyle and borderInlineStartColor instead.".into(),
    )
  }
  // @Deprecated
  fn border_left(_raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    // Upstream joins these three with a space (`.join(' ')`); concatenating
    // them bare runs the sentences together.
    Err(
      [
        "`borderLeft` is not supported.",
        "You could use `borderLeftWidth`, `borderLeftStyle` and `borderLeftColor`,",
        "but it is preferable to use `borderInlineStartWidth`, `borderInlineStartStyle` and `borderInlineStartColor`.",
      ]
      .join(" "),
    )
  }

  // Keyed by the property name an author writes, never by the Rust identifier
  // that implements it. A key that spells the identifier instead matches
  // nothing, and the shorthand then reaches the stylesheet -- the specificity
  // hole this table exists to close.
  pub fn get(name: &str) -> Option<fn(Option<TRawValue>) -> Result<Vec<OrderPair>, String>> {
    match name {
      "all" => Some(Shorthands::all),
      "animation" => Some(Shorthands::animation),
      "background" => Some(Shorthands::background),
      "border" => Some(Shorthands::border),
      "borderInline" => Some(Shorthands::border_inline),
      "borderBlock" => Some(Shorthands::border_block),
      "borderTop" => Some(Shorthands::border_top),
      "borderInlineEnd" => Some(Shorthands::border_inline_end),
      "borderRight" => Some(Shorthands::border_right),
      "borderBottom" => Some(Shorthands::border_bottom),
      "borderInlineStart" => Some(Shorthands::border_inline_start),
      "borderLeft" => Some(Shorthands::border_left),
      _ => None,
    }
  }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Aliases;

impl Aliases {
  fn block_size(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("height".into(), val)])
  }
  fn inline_size(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("width".into(), val)])
  }
  fn min_block_size(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("minHeight".into(), val)])
  }
  fn min_inline_size(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("minWidth".into(), val)])
  }
  fn max_block_size(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("maxHeight".into(), val)])
  }
  fn max_inline_size(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("maxWidth".into(), val)])
  }

  fn border_horizontal_width(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderInlineWidth".into(), val)])
  }
  fn border_horizontal_style(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderInlineStyle".into(), val)])
  }
  fn border_horizontal_color(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderInlineColor".into(), val)])
  }
  fn border_vertical_width(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderBlockWidth".into(), val)])
  }
  fn border_vertical_style(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderBlockStyle".into(), val)])
  }
  fn border_vertical_color(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderBlockColor".into(), val)])
  }

  fn border_block_start_color(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderTopColor".into(), value)])
  }
  fn border_block_end_color(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderBottomColor".into(), value)])
  }
  fn border_block_start_style(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderTopStyle".into(), value)])
  }
  fn border_block_end_style(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderBottomStyle".into(), value)])
  }
  fn border_block_start_width(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderTopWidth".into(), value)])
  }
  fn border_block_end_width(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderBottomWidth".into(), value)])
  }

  fn border_start_color(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderInlineStartColor".into(), val)])
  }
  fn border_end_color(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderInlineEndColor".into(), val)])
  }
  fn border_start_style(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderInlineStartStyle".into(), val)])
  }
  fn border_end_style(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderInlineEndStyle".into(), val)])
  }
  fn border_start_width(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderInlineStartWidth".into(), val)])
  }
  fn border_end_width(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderInlineEndWidth".into(), val)])
  }

  fn border_top_start_radius(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderStartStartRadius".into(), value)])
  }
  fn border_top_end_radius(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderStartEndRadius".into(), value)])
  }
  fn border_bottom_start_radius(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderEndStartRadius".into(), value)])
  }
  fn border_bottom_end_radius(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderEndEndRadius".into(), value)])
  }

  fn contain_intrinsic_block_size(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("containIntrinsicHeight".into(), value)])
  }
  fn contain_intrinsic_inline_size(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("containIntrinsicWidth".into(), value)])
  }

  fn margin_block_start(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("marginTop".into(), value)])
  }
  fn margin_block_end(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("marginBottom".into(), value)])
  }
  fn margin_start(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("marginInlineStart".into(), val)])
  }
  fn margin_end(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("marginInlineEnd".into(), val)])
  }
  fn margin_horizontal(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("marginInline".into(), val)])
  }
  fn margin_vertical(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("marginBlock".into(), val)])
  }

  fn overflow_block(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("overflowY".into(), value)])
  }
  fn overflow_inline(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("overflowX".into(), value)])
  }

  fn padding_block_start(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("paddingTop".into(), raw_value)])
  }
  fn padding_block_end(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("paddingBottom".into(), raw_value)])
  }
  fn padding_start(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("paddingInlineStart".into(), val)])
  }
  fn padding_end(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("paddingInlineEnd".into(), val)])
  }
  fn padding_horizontal(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("paddingInline".into(), val)])
  }
  fn padding_vertical(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("paddingBlock".into(), val)])
  }

  fn scroll_margin_block_start(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("scrollMarginTop".into(), value)])
  }
  fn scroll_margin_block_end(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("scrollMarginBottom".into(), value)])
  }

  fn inset_block_start(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("top".into(), value)])
  }
  fn inset_block_end(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("bottom".into(), value)])
  }
  fn start(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("insetInlineStart".into(), val)])
  }
  fn end(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("insetInlineEnd".into(), val)])
  }

  pub fn get(name: &str) -> Option<fn(Option<TRawValue>) -> Result<Vec<OrderPair>, String>> {
    match name {
      // @Deprecated
      "borderHorizontal" => Some(Shorthands::border_inline),
      // @Deprecated
      "borderVertical" => Some(Shorthands::border_block),
      // @Deprecated
      "borderBlockStart" => Some(Shorthands::border_top),
      // @Deprecated
      "borderEnd" => Some(Shorthands::border_inline_end),
      // @Deprecated
      "borderBlockEnd" => Some(Shorthands::border_bottom),
      // @Deprecated
      "borderStart" => Some(Shorthands::border_inline_start),

      "blockSize" => Some(Aliases::block_size),
      "inlineSize" => Some(Aliases::inline_size),
      "minBlockSize" => Some(Aliases::min_block_size),
      "minInlineSize" => Some(Aliases::min_inline_size),
      "maxBlockSize" => Some(Aliases::max_block_size),
      "maxInlineSize" => Some(Aliases::max_inline_size),
      "borderHorizontalWidth" => Some(Aliases::border_horizontal_width),
      "borderHorizontalStyle" => Some(Aliases::border_horizontal_style),
      "borderHorizontalColor" => Some(Aliases::border_horizontal_color),
      "borderVerticalWidth" => Some(Aliases::border_vertical_width),
      "borderVerticalStyle" => Some(Aliases::border_vertical_style),
      "borderVerticalColor" => Some(Aliases::border_vertical_color),
      "borderBlockStartColor" => Some(Aliases::border_block_start_color),
      "borderBlockEndColor" => Some(Aliases::border_block_end_color),
      "borderBlockStartStyle" => Some(Aliases::border_block_start_style),
      "borderBlockEndStyle" => Some(Aliases::border_block_end_style),
      "borderBlockStartWidth" => Some(Aliases::border_block_start_width),
      "borderBlockEndWidth" => Some(Aliases::border_block_end_width),
      "borderStartColor" => Some(Aliases::border_start_color),
      "borderEndColor" => Some(Aliases::border_end_color),
      "borderStartStyle" => Some(Aliases::border_start_style),
      "borderEndStyle" => Some(Aliases::border_end_style),
      "borderStartWidth" => Some(Aliases::border_start_width),
      "borderEndWidth" => Some(Aliases::border_end_width),
      "borderTopStartRadius" => Some(Aliases::border_top_start_radius),
      "borderTopEndRadius" => Some(Aliases::border_top_end_radius),
      "borderBottomStartRadius" => Some(Aliases::border_bottom_start_radius),
      "borderBottomEndRadius" => Some(Aliases::border_bottom_end_radius),
      "containIntrinsicBlockSize" => Some(Aliases::contain_intrinsic_block_size),
      "containIntrinsicInlineSize" => Some(Aliases::contain_intrinsic_inline_size),
      "marginBlockStart" => Some(Aliases::margin_block_start),
      "marginBlockEnd" => Some(Aliases::margin_block_end),
      "marginStart" => Some(Aliases::margin_start),
      "marginEnd" => Some(Aliases::margin_end),
      "marginHorizontal" => Some(Aliases::margin_horizontal),
      "marginVertical" => Some(Aliases::margin_vertical),
      "overflowBlock" => Some(Aliases::overflow_block),
      "overflowInline" => Some(Aliases::overflow_inline),
      "paddingBlockStart" => Some(Aliases::padding_block_start),
      "paddingBlockEnd" => Some(Aliases::padding_block_end),
      "paddingStart" => Some(Aliases::padding_start),
      "paddingEnd" => Some(Aliases::padding_end),
      "paddingHorizontal" => Some(Aliases::padding_horizontal),
      "paddingVertical" => Some(Aliases::padding_vertical),
      "scrollMarginBlockStart" => Some(Aliases::scroll_margin_block_start),
      "scrollMarginBlockEnd" => Some(Aliases::scroll_margin_block_end),
      "insetBlockStart" => Some(Aliases::inset_block_start),
      "insetBlockEnd" => Some(Aliases::inset_block_end),
      "start" => Some(Aliases::start),
      "end" => Some(Aliases::end),
      _ => None,
    }
  }
}
