use std::borrow::Cow;
use stylex_constants::constants::{
  cursor_flip::CURSOR_FLIP,
  logical_to_rtl::{INLINE_TO_RTL, LOGICAL_TO_RTL, LOGICAL_VALUE_TO_RTL},
};
use stylex_enums::style_resolution::StyleResolution;
use stylex_regex::regex::LENGTH_UNIT_TESTER_REGEX;
use stylex_structures::{
  pair::{Pair, PairCow},
  stylex_state_options::StyleXStateOptions,
};

fn logical_to_physical_rtl(input: &str) -> Option<&'static str> {
  // Only logical keywords (start/end/inline-start/inline-end) flip; physical values
  // such as `left`/`right` yield `None`, leaving `rtl` unset.
  LOGICAL_VALUE_TO_RTL.get(input).copied()
}

fn property_to_rtl<'a>(pair: &'a Pair, options: &StyleXStateOptions) -> Option<PairCow<'a>> {
  if let Some(&ltr_key) = LOGICAL_TO_RTL.get(pair.key.as_str()) {
    return Some(PairCow {
      key: Cow::Borrowed(ltr_key),
      value: Cow::Borrowed(pair.value.as_str()),
    });
  }

  match pair.key.as_str() {
    "float" | "clear" => logical_to_physical_rtl(pair.value.as_str()).map(|value| PairCow {
      key: Cow::Borrowed(pair.key.as_str()),
      value: Cow::Borrowed(value),
    }),
    "background-position" => {
      // Bail out unless the value carries a bare `start`/`end` keyword.
      // Directional-neutral values such as `center` or physical values such as
      // `left top` must not emit a redundant, higher-specificity `rtl` rule.
      // Once a logical keyword is present, also flip camel-case
      // `insetInlineStart`/`insetInlineEnd` tokens that appear in the same value.
      //
      // NOTE: `split_whitespace()` collapses runs of whitespace and drops empty
      // tokens. The values reaching here are already single-space normalized, so
      // this has no observable effect in practice.
      if !pair
        .value
        .split_whitespace()
        .any(|word| matches!(word, "start" | "end"))
      {
        return None;
      }

      let mut new_val = String::with_capacity(pair.value.len());
      for word in pair.value.split_whitespace() {
        if !new_val.is_empty() {
          new_val.push(' ');
        }

        new_val.push_str(match word {
          "start" | "insetInlineStart" => "right",
          "end" | "insetInlineEnd" => "left",
          other => other,
        });
      }

      Some(PairCow {
        key: Cow::Borrowed(pair.key.as_str()),
        value: Cow::Owned(new_val),
      })
    },
    "cursor" => {
      if !options.enable_legacy_value_flipping {
        return None;
      }

      CURSOR_FLIP.get(pair.value.as_str()).map(|&val| PairCow {
        key: Cow::Borrowed(pair.key.as_str()),
        value: Cow::Borrowed(val),
      })
    },
    _ => shadows_flip(pair.key.as_str(), pair.value.as_str(), options),
  }
}

pub fn generate_rtl<'a>(pair: &'a Pair, options: &StyleXStateOptions) -> Option<PairCow<'a>> {
  let enable_logical_styles_polyfill = options.enable_logical_styles_polyfill;
  let style_resolution = &options.style_resolution;
  let key = pair.key.as_str();

  if style_resolution == &StyleResolution::LegacyExpandShorthands {
    if !enable_logical_styles_polyfill {
      return None;
    }
    if let Some(&inline_to_rtl_value) = INLINE_TO_RTL.get(key) {
      return Some(PairCow {
        key: Cow::Borrowed(inline_to_rtl_value),
        value: Cow::Borrowed(pair.value.as_str()),
      });
    }
  }

  property_to_rtl(pair, options)
}

fn shadows_flip<'a>(key: &'a str, val: &str, options: &StyleXStateOptions) -> Option<PairCow<'a>> {
  match key {
    "box-shadow" | "text-shadow" => {
      if !options.enable_legacy_value_flipping {
        return None;
      }

      let rtl_val = flip_shadow(val);
      rtl_val.map(|rtl_val| PairCow {
        key: Cow::Borrowed(key),
        value: Cow::Owned(rtl_val),
      })
    },
    _ => None,
  }
}

fn flip_shadow(value: &str) -> Option<String> {
  let rtl = value
    .split(',')
    .map(|def| {
      let parts: Vec<&str> = def.split_whitespace().collect();
      // NOTE: temporary solution, need to implement unit parser
      let flip_index = match parts.first() {
        Some(first) if is_unit(first) => 0,
        _ => 1,
      };

      let mut out = String::with_capacity(def.len() + 1);
      for (i, part) in parts.iter().enumerate() {
        if i > 0 {
          out.push(' ');
        }
        if i == flip_index {
          out.push_str(&flip_sign(part));
        } else {
          out.push_str(part);
        }
      }
      out
    })
    .collect::<Vec<_>>()
    .join(",");

  if rtl != value { Some(rtl) } else { None }
}

fn is_unit(input: &str) -> bool {
  // Regex match failure is practically unreachable; default to false.
  LENGTH_UNIT_TESTER_REGEX.is_match(input).unwrap_or(false)
}

fn flip_sign(value: &str) -> String {
  if value == "0" {
    value.to_string()
  } else if value.starts_with('-') {
    value.strip_prefix('-').unwrap_or(value).to_string()
  } else {
    let mut flipped = String::with_capacity(value.len() + 1);
    flipped.push('-');
    flipped.push_str(value);
    flipped
  }
}
