use postcss_value_parser::{NodeKind, ValueParser, stringify, unit};
use std::borrow::Cow;
use stylex_constants::constants::{
  cursor_flip::CURSOR_FLIP,
  logical_to_rtl::{INLINE_TO_RTL, LOGICAL_TO_RTL, LOGICAL_VALUE_TO_RTL},
};
use stylex_enums::style_resolution::StyleResolution;
use stylex_structures::{
  pair::{Pair, PairCow},
  stylex_state_options::StyleXStateOptions,
};

fn logical_to_physical_rtl(input: &str) -> Option<&'static str> {
  // Only logical keywords flip; physical values yield `None`, leaving `rtl` unset.
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
      // Only a bare `start`/`end` warrants an `rtl` rule; then camel-case
      // `insetInlineStart`/`insetInlineEnd` flip too.
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
  let ast = ValueParser::new(value);
  let mut groups = Vec::new();
  let mut start = 0;
  // Only top-level dividers separate shadow groups.
  for (index, node) in ast.nodes.iter().enumerate() {
    if node.kind == NodeKind::Div {
      if start < index {
        groups.push(stringify(&ast.nodes[start..index]));
      }
      start = index + 1;
    }
  }
  if start < ast.nodes.len() {
    groups.push(stringify(&ast.nodes[start..]));
  }
  let rtl = groups
    .into_iter()
    .map(|definition| {
      let mut parts: Vec<String> = definition.split(' ').map(str::to_owned).collect();
      // Match postcss-value-parser `unit()`; no CSS length-unit whitelist.
      let flip_index = if parts.first().is_some_and(|first| unit(first).is_some()) {
        0
      } else {
        1
      };
      if let Some(part) = parts.get_mut(flip_index) {
        *part = flip_sign(part);
      }
      parts.join(" ")
    })
    .collect::<Vec<_>>()
    .join(",");
  if rtl != value { Some(rtl) } else { None }
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
