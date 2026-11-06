use crate::values::{
  common::{split_value_required, value_parts},
  parser::split_value_parts,
};
use stylex_constants::constants::common::{LOGICAL_FLOAT_END_VAR, LOGICAL_FLOAT_START_VAR};
use stylex_structures::{order_pair::OrderPair, raw_value::TRawValue};
use stylex_utils::string::json_stringify;

/// Whether `part` is spellable as a `list-style-type`.
///
/// Upstream asks this with `listStyleTypeRegex` in
/// `shared/preprocess-rules/legacy-expand-shorthands.js`, which is
/// `/^([a-z-]+|".*?"|'.*?')$/`, and the three alternatives are reproduced one by
/// one rather than collapsed. Both quote
/// characters are accepted because a part arrives with the character the author
/// typed: the splitter echoes a string rather than re-quoting it, so a
/// single-quoted family name is single-quoted here.
pub(crate) fn is_list_style_type(part: &str) -> bool {
  is_quoted_with(part, '"') || is_quoted_with(part, '\'') || is_lowercase_ident(part)
}

/// One alternative of that pattern: `quote`, any run of `.`, then `quote`.
///
/// `.` in a JavaScript regular expression matches anything *except* a line
/// terminator, so a quoted part with a newline inside it fails upstream's test
/// and has to fail this one. The four characters JavaScript counts as line
/// terminators are spelled out here; `char::is_control` would also exclude a
/// tab, which upstream accepts.
fn is_quoted_with(part: &str, quote: char) -> bool {
  let Some(inner) = part
    .strip_prefix(quote)
    .and_then(|rest| rest.strip_suffix(quote))
  else {
    return false;
  };

  !inner.contains(['\n', '\r', '\u{2028}', '\u{2029}'])
}

/// The `[a-z-]+` alternative.
fn is_lowercase_ident(part: &str) -> bool {
  !part.is_empty()
    && part
      .chars()
      .all(|character| character.is_ascii_lowercase() || character == '-')
}

/// The text upstream emits when a `listStyle` value cannot be disambiguated.
///
/// Upstream builds this by interpolating `JSON.stringify(rawValue)`, so the
/// value arrives quoted and JSON-escaped rather than printed raw. Three of the
/// four rejection sites spell it exactly this way.
fn list_style_rejection(raw_value_str: &str) -> String {
  format!(
    "invalid \"listStyle\" value of {}",
    json_stringify(raw_value_str)
  )
}

/// [`list_style_rejection`] with the value wrapped in a second pair of quotes.
///
/// Upstream is not self-consistent here: the first of its four throws wraps the
/// already-quoted `JSON.stringify` result in another pair of literal quotes
/// (`legacy-expand-shorthands.js:301`) and the other three do not, so a
/// `var(--x)` rejection reads `value of ""none var(--x)""` where a duplicate
/// `listStylePosition` reads `value of "inside outside"`.
///
/// Reproduced rather than normalised, deliberately. These messages reach an
/// author through `propertyValidationMode`, and an author comparing the two
/// compilers on the same input should read the same sentence from both — the
/// asymmetry is upstream's to fix, and matching it keeps the divergence list
/// free of an entry nobody asked for. Pinned by
/// `shorthands_list_style_var_mixed_with_other` and
/// `shorthands_list_style_global_mixed`, against text measured from the
/// installed 0.19.0 plugin rather than read off the source.
fn list_style_rejection_with_doubled_quotes(raw_value_str: &str) -> String {
  format!(
    "invalid \"listStyle\" value of \"{}\"",
    json_stringify(raw_value_str)
  )
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Shorthands;

impl Shorthands {
  /// All shorthand expansion functions return `Ok(…)` unconditionally;
  /// the `Err` variant exists only for the shared function-pointer type.
  #[cfg_attr(coverage_nightly, coverage(off))]
  fn infallible(result: Result<Vec<OrderPair>, String>) -> Vec<OrderPair> {
    match result {
      Ok(v) => v,
      Err(e) => unreachable!("infallible shorthand returned Err: {}", e),
    }
  }

  fn border(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("borderTop".into(), raw_value.to_owned()),
      OrderPair("borderInlineEnd".into(), raw_value.to_owned()),
      OrderPair("borderBottom".into(), raw_value.to_owned()),
      OrderPair("borderInlineStart".into(), raw_value),
    ])
  }

  fn border_color(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    let (top, right, bottom, left) = split_value_required(raw_value.as_ref());

    Ok(vec![
      OrderPair("borderTopColor".into(), Some(top)),
      OrderPair("borderInlineEndColor".into(), Some(right)),
      OrderPair("borderBottomColor".into(), Some(bottom)),
      OrderPair("borderInlineStartColor".into(), Some(left)),
    ])
  }
  fn border_horizontal(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("borderInlineStart".into(), raw_value.to_owned()),
      OrderPair("borderInlineEnd".into(), raw_value),
    ])
  }
  fn border_style(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    let (top, right, bottom, left) = split_value_required(raw_value.as_ref());

    Ok(vec![
      OrderPair("borderTopStyle".into(), Some(top)),
      OrderPair("borderInlineEndStyle".into(), Some(right)),
      OrderPair("borderBottomStyle".into(), Some(bottom)),
      OrderPair("borderInlineStartStyle".into(), Some(left)),
    ])
  }
  fn border_vertical(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("borderTop".into(), raw_value.to_owned()),
      OrderPair("borderBottom".into(), raw_value),
    ])
  }
  fn border_width(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    let (top, right, bottom, left) = split_value_required(raw_value.as_ref());

    Ok(vec![
      OrderPair("borderTopWidth".into(), Some(top)),
      OrderPair("borderInlineEndWidth".into(), Some(right)),
      OrderPair("borderBottomWidth".into(), Some(bottom)),
      OrderPair("borderInlineStartWidth".into(), Some(left)),
    ])
  }

  fn border_horizontal_color(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("borderInlineStartColor".into(), raw_value.to_owned()),
      OrderPair("borderInlineEndColor".into(), raw_value),
    ])
  }
  fn border_horizontal_style(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("borderInlineStartStyle".into(), raw_value.to_owned()),
      OrderPair("borderInlineEndStyle".into(), raw_value),
    ])
  }
  fn border_horizontal_width(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("borderInlineStartWidth".into(), raw_value.to_owned()),
      OrderPair("borderInlineEndWidth".into(), raw_value),
    ])
  }
  fn border_inline_color(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("borderInlineStartColor".into(), raw_value.to_owned()),
      OrderPair("borderInlineEndColor".into(), raw_value),
    ])
  }
  fn border_inline_style(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("borderInlineStartStyle".into(), raw_value.to_owned()),
      OrderPair("borderInlineEndStyle".into(), raw_value),
    ])
  }
  fn border_inline_width(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("borderInlineStartWidth".into(), raw_value.to_owned()),
      OrderPair("borderInlineEndWidth".into(), raw_value),
    ])
  }
  fn border_vertical_color(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("borderTopColor".into(), raw_value.to_owned()),
      OrderPair("borderBottomColor".into(), raw_value),
    ])
  }
  fn border_vertical_style(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("borderTopStyle".into(), raw_value.to_owned()),
      OrderPair("borderBottomStyle".into(), raw_value),
    ])
  }
  fn border_vertical_width(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("borderTopWidth".into(), raw_value.to_owned()),
      OrderPair("borderBottomWidth".into(), raw_value),
    ])
  }

  fn border_radius(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    let (top, right, bottom, left) = split_value_required(raw_value.as_ref());

    Ok(vec![
      OrderPair("borderStartStartRadius".into(), Some(top)),
      OrderPair("borderStartEndRadius".into(), Some(right)),
      OrderPair("borderEndEndRadius".into(), Some(bottom)),
      OrderPair("borderEndStartRadius".into(), Some(left)),
    ])
  }

  fn contain_intrinsic_size(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    // The parts themselves, not the four-sided view. This expansion folds over
    // them rather than destructuring them, and the four-sided view repeats a
    // missing side: `auto` arrived as four copies of itself, and the fold below
    // joined each copy to the one before it, emitting `auto auto` for both axes.
    let parts = value_parts(raw_value.as_ref());

    let mut coll: Vec<TRawValue> = Vec::with_capacity(parts.len());

    // `auto` is a qualifier here rather than a size: `contain-intrinsic-size:
    // auto 1px` means "1px, remembered", so the two belong to one axis and the
    // fold joins them into one part.
    for part in parts {
      let follows_auto = coll
        .last()
        .is_some_and(|last_element| last_element.as_css_text() == "auto");

      // Joined whatever the part says, the empty part included, which is the
      // rule `crate::values::parser` states for every consumer of a part list:
      // an empty part is present. Upstream's guard here asks whether the part is
      // *absent*, which no part of a split value is here -- and skipping an
      // empty one instead loses the axis: an unterminated comment contributes an
      // empty part, so `auto /*` sized only the width where upstream sizes both.
      //
      // Absent is reachable upstream and not here for a reason worth naming, so
      // that nobody restores the guard by reading `splitValue` alone: upstream's
      // returns `$ReadOnlyArray<number | string | null>`, and the `null` comes
      // from its two early arms -- `str == null` returning `[str]`, and the
      // `Array.isArray(str)` passthrough it keeps only for Flow. `TRawValue`
      // (`stylex_structures::raw_value`) has no array variant and a null value
      // never reaches this far, so neither arm has a counterpart here and the
      // guard has nothing left to guard.
      if follows_auto {
        let combined = format!("auto {}", part.as_css_text());

        coll.pop();
        coll.push(TRawValue::String(combined));
        continue;
      }

      coll.push(part);
    }

    let mut folded = coll.into_iter();
    let width = folded.next().unwrap_or_default();
    // A value of one part sizes both axes with it, which is also why this reads
    // the fold's output rather than indexing it: a single part leaves nothing at
    // index one.
    let height = folded.next().unwrap_or_else(|| width.clone());

    Ok(vec![
      OrderPair("containIntrinsicWidth".into(), Some(width)),
      OrderPair("containIntrinsicHeight".into(), Some(height)),
    ])
  }

  fn inset(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    let (top, right, bottom, left) = split_value_required(raw_value.as_ref());

    Ok(vec![
      OrderPair("top".into(), Some(top)),
      OrderPair("insetInlineEnd".into(), Some(right)),
      OrderPair("bottom".into(), Some(bottom)),
      OrderPair("insetInlineStart".into(), Some(left)),
    ])
  }
  fn inset_inline(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    let (start, end, _, _) = split_value_required(raw_value.as_ref());

    let mut result = vec![];

    result.extend(Self::infallible(Shorthands::start(Some(start))));
    result.extend(Self::infallible(Shorthands::end(Some(end))));

    Ok(result)
  }
  fn inset_block(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    let (top, bottom, _, _) = split_value_required(raw_value.as_ref());

    Ok(vec![
      OrderPair("top".into(), Some(top)),
      OrderPair("bottom".into(), Some(bottom)),
    ])
  }
  fn start(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("insetInlineStart".into(), raw_value),
      OrderPair("left".into(), None),
      OrderPair("right".into(), None),
    ])
  }
  fn end(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("insetInlineEnd".into(), raw_value),
      OrderPair("left".into(), None),
      OrderPair("right".into(), None),
    ])
  }
  fn left(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("left".into(), raw_value),
      OrderPair("insetInlineStart".into(), None),
      OrderPair("insetInlineEnd".into(), None),
    ])
  }
  fn right(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("right".into(), raw_value),
      OrderPair("insetInlineStart".into(), None),
      OrderPair("insetInlineEnd".into(), None),
    ])
  }

  fn gap(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    let (row, column, _, _) = split_value_required(raw_value.as_ref());

    Ok(vec![
      OrderPair("rowGap".into(), Some(row)),
      OrderPair("columnGap".into(), Some(column)),
    ])
  }
  fn margin(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    let (top, right, bottom, left) = split_value_required(raw_value.as_ref());

    Ok(vec![
      OrderPair("marginTop".into(), Some(top)),
      OrderPair("marginInlineEnd".into(), Some(right)),
      OrderPair("marginBottom".into(), Some(bottom)),
      OrderPair("marginInlineStart".into(), Some(left)),
    ])
  }
  fn margin_horizontal(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    let (start, end, _, _) = split_value_required(raw_value.as_ref());

    let mut result = vec![];

    result.extend(Self::infallible(Shorthands::margin_start(Some(start))));
    result.extend(Self::infallible(Shorthands::margin_end(Some(end))));

    Ok(result)
  }
  fn margin_start(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("marginInlineStart".into(), raw_value),
      OrderPair("marginLeft".into(), None),
      OrderPair("marginRight".into(), None),
    ])
  }
  fn margin_end(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("marginInlineEnd".into(), raw_value),
      OrderPair("marginLeft".into(), None),
      OrderPair("marginRight".into(), None),
    ])
  }
  fn margin_left(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("marginLeft".into(), raw_value),
      OrderPair("marginInlineStart".into(), None),
      OrderPair("marginInlineEnd".into(), None),
    ])
  }
  fn margin_right(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("marginRight".into(), raw_value),
      OrderPair("marginInlineStart".into(), None),
      OrderPair("marginInlineEnd".into(), None),
    ])
  }
  fn margin_vertical(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    let (top, bottom, _, _) = split_value_required(raw_value.as_ref());

    Ok(vec![
      OrderPair("marginTop".into(), Some(top)),
      OrderPair("marginBottom".into(), Some(bottom)),
    ])
  }

  fn list_style(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    let Some(raw_value_str) = raw_value.as_ref().map(TRawValue::as_css_text) else {
      return Ok(vec![
        OrderPair("listStyleType".into(), None),
        OrderPair("listStylePosition".into(), None),
        OrderPair("listStyleImage".into(), None),
      ]);
    };

    let parts: Vec<String> = split_value_parts(raw_value_str.as_ref());

    // Global values that must be the only value
    let list_style_global_values = ["inherit", "initial", "revert", "unset"];

    // Position values (unambiguous)
    let list_style_position_values = ["inside", "outside"];

    // Handle global keywords - must be the only value
    if parts.len() == 1 && list_style_global_values.contains(&parts[0].as_str()) {
      let global_value = Some(TRawValue::String(parts[0].clone()));
      return Ok(vec![
        OrderPair("listStyleType".into(), global_value.clone()),
        OrderPair("listStylePosition".into(), global_value.clone()),
        OrderPair("listStyleImage".into(), global_value),
      ]);
    }

    let mut image: Option<TRawValue> = None;
    let mut position: Option<TRawValue> = None;
    let mut list_type: Option<TRawValue> = None;
    let mut remaining_parts: Vec<String> = Vec::new();

    // First pass: assign values that can only belong to one property.
    //
    // The parts are consumed rather than borrowed. `parts` is dead after this
    // loop, and the borrow forced a `String` clone at each of the three places a
    // part is kept -- the global-keyword return above has already finished with
    // it, so there is nothing left that needs the original.
    for part in parts {
      // Check for global keywords mixed with other values (invalid)
      // and use of `var()` which can't be disambiguated
      if list_style_global_values.contains(&part.as_str()) || part.contains("var(--") {
        return Err(list_style_rejection_with_doubled_quotes(&raw_value_str));
      }
      // Check if it's a position value (unambiguous)
      else if list_style_position_values.contains(&part.as_str()) {
        if position.is_some() {
          return Err(list_style_rejection(&raw_value_str));
        }
        position = Some(TRawValue::String(part));
      }
      // Check if it's a type value that's not 'none' (unambiguous)
      // Type values are: keywords (letters and hyphens) or quoted strings
      else if part != "none" && is_list_style_type(&part) {
        if list_type.is_some() {
          return Err(list_style_rejection(&raw_value_str));
        }
        list_type = Some(TRawValue::String(part));
      }
      // Keep ambiguous values for second pass. An empty part arrives here: it is
      // neither a global keyword nor a position, and `is_list_style_type`
      // requires at least one character, mirroring the `+` in upstream's
      // pattern. It then takes the slot it lands in, which is the rule
      // `crate::values::parser` states -- so `list-style: 'url(a.png) /*'`
      // refuses for two images rather than quietly discarding one of them.
      else {
        remaining_parts.push(part);
      }
    }

    // Second pass: handle remaining parts (including 'none' and image values)
    for part in remaining_parts {
      // If 'none' and type is not yet assigned, assign to type
      if part == "none" && list_type.is_none() {
        list_type = Some(TRawValue::String(part));
      }
      // Otherwise assign to image
      else {
        if image.is_some() {
          return Err(list_style_rejection(&raw_value_str));
        }
        image = Some(TRawValue::String(part));
      }
    }

    Ok(vec![
      OrderPair("listStyleType".into(), list_type),
      OrderPair("listStylePosition".into(), position),
      OrderPair("listStyleImage".into(), image),
    ])
  }

  fn overflow(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    let (x, y, _, _) = split_value_required(raw_value.as_ref());
    Ok(vec![
      OrderPair("overflowX".into(), Some(x)),
      OrderPair("overflowY".into(), Some(y)),
    ])
  }
  fn padding(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    let (top, right, bottom, left) = split_value_required(raw_value.as_ref());

    Ok(vec![
      OrderPair("paddingTop".into(), Some(top)),
      OrderPair("paddingInlineEnd".into(), Some(right)),
      OrderPair("paddingBottom".into(), Some(bottom)),
      OrderPair("paddingInlineStart".into(), Some(left)),
    ])
  }
  fn padding_horizontal(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    let (start, end, _, _) = split_value_required(raw_value.as_ref());

    let mut result = vec![];

    result.extend(Self::infallible(Shorthands::padding_start(Some(start))));
    result.extend(Self::infallible(Shorthands::padding_end(Some(end))));

    Ok(result)
  }
  fn padding_start(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("paddingInlineStart".into(), val),
      OrderPair("paddingLeft".into(), None),
      OrderPair("paddingRight".into(), None),
    ])
  }
  fn padding_end(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("paddingInlineEnd".into(), val),
      OrderPair("paddingLeft".into(), None),
      OrderPair("paddingRight".into(), None),
    ])
  }
  fn padding_left(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("paddingLeft".into(), val),
      OrderPair("paddingInlineStart".into(), None),
      OrderPair("paddingInlineEnd".into(), None),
    ])
  }
  fn padding_right(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![
      OrderPair("paddingRight".into(), val),
      OrderPair("paddingInlineStart".into(), None),
      OrderPair("paddingInlineEnd".into(), None),
    ])
  }
  fn padding_vertical(raw_value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    let (top, bottom, _, _) = split_value_required(raw_value.as_ref());

    Ok(vec![
      OrderPair("paddingTop".into(), Some(top)),
      OrderPair("paddingBottom".into(), Some(bottom)),
    ])
  }

  pub fn get(name: &str) -> Option<fn(Option<TRawValue>) -> Result<Vec<OrderPair>, String>> {
    match name {
      "border" => Some(Shorthands::border),
      "borderColor" => Some(Shorthands::border_color),
      "borderHorizontal" => Some(Shorthands::border_horizontal),
      "borderStyle" => Some(Shorthands::border_style),
      "borderVertical" => Some(Shorthands::border_vertical),
      "borderWidth" => Some(Shorthands::border_width),
      "borderHorizontalColor" => Some(Shorthands::border_horizontal_color),
      "borderHorizontalStyle" => Some(Shorthands::border_horizontal_style),
      "borderHorizontalWidth" => Some(Shorthands::border_horizontal_width),
      "borderInlineColor" => Some(Shorthands::border_inline_color),
      "borderInlineStyle" => Some(Shorthands::border_inline_style),
      "borderInlineWidth" => Some(Shorthands::border_inline_width),
      "borderVerticalColor" => Some(Shorthands::border_vertical_color),
      "borderVerticalStyle" => Some(Shorthands::border_vertical_style),
      "borderVerticalWidth" => Some(Shorthands::border_vertical_width),
      "borderRadius" => Some(Shorthands::border_radius),
      "containIntrinsicSize" => Some(Shorthands::contain_intrinsic_size),
      "inset" => Some(Shorthands::inset),
      "insetInline" => Some(Shorthands::inset_inline),
      "insetBlock" => Some(Shorthands::inset_block),
      "start" => Some(Shorthands::start),
      "end" => Some(Shorthands::end),
      "left" => Some(Shorthands::left),
      "right" => Some(Shorthands::right),
      "gap" => Some(Shorthands::gap),
      "listStyle" => Some(Shorthands::list_style),
      "margin" => Some(Shorthands::margin),
      "marginHorizontal" => Some(Shorthands::margin_horizontal),
      "marginStart" => Some(Shorthands::margin_start),
      "marginEnd" => Some(Shorthands::margin_end),
      "marginLeft" => Some(Shorthands::margin_left),
      "marginRight" => Some(Shorthands::margin_right),
      "marginVertical" => Some(Shorthands::margin_vertical),
      "overflow" => Some(Shorthands::overflow),
      "padding" => Some(Shorthands::padding),
      "paddingHorizontal" => Some(Shorthands::padding_horizontal),
      "paddingStart" => Some(Shorthands::padding_start),
      "paddingEnd" => Some(Shorthands::padding_end),
      "paddingLeft" => Some(Shorthands::padding_left),
      "paddingRight" => Some(Shorthands::padding_right),
      "paddingVertical" => Some(Shorthands::padding_vertical),

      _ => None,
    }
  }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Aliases;

#[allow(dead_code)]
impl Aliases {
  fn inset_block_start(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("top".into(), val)])
  }
  fn inset_block_end(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("bottom".into(), val)])
  }

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
  fn border_start(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderInlineStart".into(), val)])
  }
  fn border_end(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderInlineEnd".into(), val)])
  }
  fn border_block_start_width(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderTopWidth".into(), val)])
  }
  fn border_block_start_style(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderTopStyle".into(), val)])
  }
  fn border_block_start_color(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderTopColor".into(), val)])
  }
  fn border_block_end_width(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderBottomWidth".into(), val)])
  }
  fn border_block_end_style(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderBottomStyle".into(), val)])
  }
  fn border_block_end_color(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderBottomColor".into(), val)])
  }
  /// The four legacy radius spellings, each resolving to the logical property.
  ///
  /// The direction is the one [`Shorthands::border_radius`] above already takes,
  /// and the one the reference compiler takes
  /// (`shared/preprocess-rules/legacy-expand-shorthands.js`, 0.19.0). It was
  /// ported the other way from `packages/shared/lib`, a prebuilt artifact of a
  /// release from before the mapping was flipped upstream — which left each
  /// alias resolving to itself and emitting `border-top-start-radius`, a
  /// property CSS does not have and every browser drops.
  fn border_top_start_radius(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderStartStartRadius".into(), val)])
  }
  fn border_top_end_radius(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderStartEndRadius".into(), val)])
  }
  fn border_bottom_start_radius(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderEndStartRadius".into(), val)])
  }
  fn border_bottom_end_radius(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("borderEndEndRadius".into(), val)])
  }

  fn grid_row_gap(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("rowGap".into(), value)])
  }
  fn grid_column_gap(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("columnGap".into(), value)])
  }

  fn margin_block_start(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("marginTop".into(), val)])
  }
  fn margin_block_end(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("marginBottom".into(), val)])
  }
  fn margin_inline_start(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("marginInlineStart".into(), val)])
  }
  fn margin_inline_end(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("marginInlineEnd".into(), val)])
  }

  fn overflow_block(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("overflowY".into(), value)])
  }
  fn overflow_inline(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("overflowX".into(), value)])
  }

  fn padding_block_start(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("paddingTop".into(), val)])
  }
  fn padding_block_end(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("paddingBottom".into(), val)])
  }

  fn padding_inline_start(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("paddingInlineStart".into(), val)])
  }
  fn padding_inline_end(val: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("paddingInlineEnd".into(), val)])
  }

  fn scroll_margin_block_start(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("scrollMarginTop".into(), value)])
  }
  fn scroll_margin_block_end(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    Ok(vec![OrderPair("scrollMarginBottom".into(), value)])
  }

  fn float(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    if let Some(ref val) = value {
      if val == "inline-start" || val == "start" {
        return Ok(vec![OrderPair(
          "float".into(),
          Some(format!("var({})", LOGICAL_FLOAT_START_VAR).into()),
        )]);
      } else if val == "inline-end" || val == "end" {
        return Ok(vec![OrderPair(
          "float".into(),
          Some(format!("var({})", LOGICAL_FLOAT_END_VAR).into()),
        )]);
      }
    }
    Ok(vec![OrderPair("float".into(), value)])
  }

  fn clear(value: Option<TRawValue>) -> Result<Vec<OrderPair>, String> {
    if let Some(ref val) = value {
      if val == "inline-start" || val == "start" {
        return Ok(vec![OrderPair(
          "clear".into(),
          Some(format!("var({})", LOGICAL_FLOAT_START_VAR).into()),
        )]);
      } else if val == "inline-end" || val == "end" {
        return Ok(vec![OrderPair(
          "clear".into(),
          Some(format!("var({})", LOGICAL_FLOAT_END_VAR).into()),
        )]);
      }
    }
    Ok(vec![OrderPair("clear".into(), value)])
  }

  pub fn get(name: &str) -> Option<fn(Option<TRawValue>) -> Result<Vec<OrderPair>, String>> {
    match name {
      "insetBlockStart" => Some(Aliases::inset_block_start),
      "insetBlockEnd" => Some(Aliases::inset_block_end),
      "insetInlineStart" => Some(Shorthands::start),
      "insetInlineEnd" => Some(Shorthands::end),
      "blockSize" => Some(Aliases::block_size),
      "inlineSize" => Some(Aliases::inline_size),
      "minBlockSize" => Some(Aliases::min_block_size),
      "minInlineSize" => Some(Aliases::min_inline_size),
      "maxBlockSize" => Some(Aliases::max_block_size),
      "maxInlineSize" => Some(Aliases::max_inline_size),
      "borderBlockWidth" => Some(Shorthands::border_vertical_width),
      "borderBlockStyle" => Some(Shorthands::border_vertical_style),
      "borderBlockColor" => Some(Shorthands::border_vertical_color),
      "borderStart" => Some(Aliases::border_start),
      "borderEnd" => Some(Aliases::border_end),
      "borderBlockStartWidth" => Some(Aliases::border_block_start_width),
      "borderBlockStartStyle" => Some(Aliases::border_block_start_style),
      "borderBlockStartColor" => Some(Aliases::border_block_start_color),
      "borderBlockEndWidth" => Some(Aliases::border_block_end_width),
      "borderBlockEndStyle" => Some(Aliases::border_block_end_style),
      "borderBlockEndColor" => Some(Aliases::border_block_end_color),
      "borderInlineWidth" => Some(Shorthands::border_inline_width),
      "borderInlineStyle" => Some(Shorthands::border_inline_style),
      "borderInlineColor" => Some(Shorthands::border_inline_color),
      "borderTopStartRadius" => Some(Aliases::border_top_start_radius),
      "borderTopEndRadius" => Some(Aliases::border_top_end_radius),
      "borderBottomStartRadius" => Some(Aliases::border_bottom_start_radius),
      "borderBottomEndRadius" => Some(Aliases::border_bottom_end_radius),
      "gridGap" => Some(Shorthands::gap),
      "gridRowGap" => Some(Aliases::grid_row_gap),
      "gridColumnGap" => Some(Aliases::grid_column_gap),
      "marginBlock" => Some(Shorthands::margin_vertical),
      "marginBlockStart" => Some(Aliases::margin_block_start),
      "marginBlockEnd" => Some(Aliases::margin_block_end),
      "marginInline" => Some(Shorthands::margin_horizontal),
      "marginInlineStart" => Some(Aliases::margin_inline_start),
      "marginInlineEnd" => Some(Aliases::margin_inline_end),
      "overflowBlock" => Some(Aliases::overflow_block),
      "overflowInline" => Some(Aliases::overflow_inline),
      "paddingBlock" => Some(Shorthands::padding_vertical),
      "paddingBlockStart" => Some(Aliases::padding_block_start),
      "paddingBlockEnd" => Some(Aliases::padding_block_end),
      "paddingInline" => Some(Shorthands::padding_horizontal),
      "paddingInlineStart" => Some(Aliases::padding_inline_start),
      "paddingInlineEnd" => Some(Aliases::padding_inline_end),
      "scrollMarginBlockStart" => Some(Aliases::scroll_margin_block_start),
      "scrollMarginBlockEnd" => Some(Aliases::scroll_margin_block_end),
      "float" => Some(Aliases::float),
      "clear" => Some(Aliases::clear),
      _ => None,
    }
  }
}
