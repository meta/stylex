use std::borrow::Cow;

use stylex_constants::constants::logical_to_ltr::{INLINE_PROPERTY_TO_LTR, PROPERTY_TO_LTR};
use stylex_enums::style_resolution::StyleResolution;
use stylex_structures::{
  pair::{Pair, PairCow},
  stylex_state_options::StyleXStateOptions,
};

pub fn generate_ltr<'a>(pair: &'a Pair, options: &StyleXStateOptions) -> PairCow<'a> {
  let enable_logical_styles_polyfill = options.enable_logical_styles_polyfill;
  let style_resolution = &options.style_resolution;
  let key = pair.key.as_str();

  if style_resolution == &StyleResolution::LegacyExpandShorthands {
    if !enable_logical_styles_polyfill {
      return PairCow::borrowed(pair);
    }

    if let Some(&inline_to_ltr_value) = INLINE_PROPERTY_TO_LTR.get(key) {
      return PairCow {
        key: Cow::Borrowed(inline_to_ltr_value),
        value: Cow::Borrowed(pair.value.as_str()),
      };
    }
  }

  property_to_ltr(pair)
}

fn logical_to_physical_ltr(input: &str) -> Option<&str> {
  match input {
    "start" | "inline-start" => Some("left"),
    "end" | "inline-end" => Some("right"),
    _ => None,
  }
}

fn property_to_ltr(pair: &Pair) -> PairCow<'_> {
  match pair.key.as_str() {
    "background-position" => {
      // Always returns a pair. Only the logical keywords flip to physical values;
      // when none are present, borrow the original value instead of allocating
      // a redundant owned copy (the common case, e.g. `center`).
      //
      // NOTE: `split_whitespace()` collapses runs of whitespace and drops empty
      // tokens. The values reaching here are already single-space normalized, so
      // this has no observable effect in practice.
      if !pair.value.split_whitespace().any(|word| {
        matches!(
          word,
          "start" | "end" | "insetInlineStart" | "insetInlineEnd"
        )
      }) {
        return PairCow::borrowed(pair);
      }

      let mut new_val = String::with_capacity(pair.value.len());
      for word in pair.value.split_whitespace() {
        if !new_val.is_empty() {
          new_val.push(' ');
        }

        new_val.push_str(match word {
          "start" | "insetInlineStart" => "left",
          "end" | "insetInlineEnd" => "right",
          other => other,
        });
      }

      PairCow {
        key: Cow::Borrowed(pair.key.as_str()),
        value: Cow::Owned(new_val),
      }
    },
    "float" | "clear" => logical_to_physical_ltr(pair.value.as_str())
      .map(|value| PairCow {
        key: Cow::Borrowed(pair.key.as_str()),
        value: Cow::Borrowed(value),
      })
      .unwrap_or_else(|| PairCow::borrowed(pair)),
    k => {
      if let Some(&physical) = PROPERTY_TO_LTR.get(k) {
        PairCow {
          key: Cow::Borrowed(physical),
          value: Cow::Borrowed(pair.value.as_str()),
        }
      } else {
        PairCow::borrowed(pair)
      }
    },
  }
}
