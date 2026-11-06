/*!
CSS Box Shadow property parsing.

Handles box-shadow property syntax including offset, blur, spread, color, and inset values.
Supports multiple shadow values separated by commas.
*/

use crate::{
  css_types::{Color, Length},
  token_parser::TokenParser,
  token_types::SimpleToken,
};
use std::fmt::{self, Display};

fn is_inset_ident(token: &SimpleToken) -> bool {
  if let SimpleToken::Ident(value) = token {
    value == "inset"
  } else {
    false
  }
}

fn is_none_ident(token: &SimpleToken) -> bool {
  if let SimpleToken::Ident(value) = token {
    value == "none"
  } else {
    false
  }
}

/// Individual box shadow value
#[derive(Debug, Clone, PartialEq)]
pub struct BoxShadow {
  pub offset_x: Length,
  pub offset_y: Length,
  pub blur_radius: Length,
  pub spread_radius: Length,
  pub color: Color,
  pub inset: bool,
}

impl BoxShadow {
  /// Create a new BoxShadow
  pub fn new(
    offset_x: Length,
    offset_y: Length,
    blur_radius: Length,
    spread_radius: Length,
    color: Color,
    inset: bool,
  ) -> Self {
    Self {
      offset_x,
      offset_y,
      blur_radius,
      spread_radius,
      color,
      inset,
    }
  }

  /// Create a simple box shadow with default values for optional parameters
  pub fn simple(
    offset_x: Length,
    offset_y: Length,
    blur_radius: Option<Length>,
    spread_radius: Option<Length>,
    color: Color,
    inset: bool,
  ) -> Self {
    Self::new(
      offset_x,
      offset_y,
      blur_radius.unwrap_or(Length::new(0.0, "px")),
      spread_radius.unwrap_or(Length::new(0.0, "px")),
      color,
      inset,
    )
  }

  /// Parser for box shadow values
  pub fn parser() -> TokenParser<BoxShadow> {
    let whitespace = TokenParser::<SimpleToken>::token(SimpleToken::Whitespace, Some("Whitespace"));

    // Parse outer shadow: offsetX offsetY [blurRadius] [spreadRadius] color
    let outer_shadow = {
      let offset_x = Length::parser();
      let offset_y = whitespace.flat_map(|_| Length::parser(), Some("offset_y"));
      let blur_radius = whitespace
        .flat_map(|_| Length::parser(), Some("blur_radius"))
        .optional();
      let spread_radius = whitespace
        .flat_map(|_| Length::parser(), Some("spread_radius"))
        .optional();
      let color = whitespace.flat_map(|_| Color::parse(), Some("color"));

      offset_x
        .flat_map(
          move |x| {
            let x_clone = x;
            offset_y
              .clone()
              .map(move |y| (x_clone.clone(), y), Some("with_y"))
          },
          Some("x_step"),
        )
        .flat_map(
          move |(x, y)| {
            let x_clone = x;
            let y_clone = y;
            blur_radius.clone().map(
              move |blur| (x_clone.clone(), y_clone.clone(), blur),
              Some("with_blur"),
            )
          },
          Some("blur_step"),
        )
        .flat_map(
          move |(x, y, blur)| {
            let x_clone = x;
            let y_clone = y;
            let blur_clone = blur;
            spread_radius.clone().map(
              move |spread| (x_clone.clone(), y_clone.clone(), blur_clone.clone(), spread),
              Some("with_spread"),
            )
          },
          Some("spread_step"),
        )
        .flat_map(
          move |(x, y, blur, spread)| {
            let x_clone = x;
            let y_clone = y;
            let blur_clone = blur;
            let spread_clone = spread;
            color.clone().map(
              move |color| {
                BoxShadow::new(
                  x_clone.clone(),
                  y_clone.clone(),
                  blur_clone.clone().unwrap_or_else(|| Length::new(0.0, "px")),
                  spread_clone
                    .clone()
                    .unwrap_or_else(|| Length::new(0.0, "px")),
                  color,
                  false,
                )
              },
              Some("create_shadow"),
            )
          },
          Some("color_step"),
        )
    };

    let inset_shadow = {
      let inset_keyword =
        TokenParser::<SimpleToken>::token(SimpleToken::Ident("inset".to_string()), Some("Ident"))
          .where_fn(is_inset_ident, Some("inset_check"));

      let whitespace_for_inset = whitespace.clone();
      let inset_keyword_for_inset = inset_keyword.clone();

      let shadow_then_inset = outer_shadow.flat_map(
        move |shadow| {
          let shadow_clone = shadow;
          let whitespace_clone = whitespace_for_inset.clone();
          let inset_clone = inset_keyword_for_inset.clone();

          whitespace_clone.flat_map(
            move |_| {
              let shadow_for_map = shadow_clone.clone();
              inset_clone.map(
                move |_| {
                  BoxShadow::new(
                    shadow_for_map.offset_x.clone(),
                    shadow_for_map.offset_y.clone(),
                    shadow_for_map.blur_radius.clone(),
                    shadow_for_map.spread_radius.clone(),
                    shadow_for_map.color.clone(),
                    true,
                  )
                },
                Some("to_inset"),
              )
            },
            Some("add_inset"),
          )
        },
        Some("shadow_then_inset"),
      );

      let whitespace_for_prefix = whitespace;
      let outer_shadow_for_prefix = outer_shadow.clone();

      let inset_then_shadow = inset_keyword.flat_map(
        move |_| {
          let whitespace_clone = whitespace_for_prefix.clone();
          let shadow_parser = outer_shadow_for_prefix.clone();

          whitespace_clone.flat_map(
            move |_| {
              shadow_parser.map(
                move |shadow| {
                  BoxShadow::new(
                    shadow.offset_x.clone(),
                    shadow.offset_y.clone(),
                    shadow.blur_radius.clone(),
                    shadow.spread_radius.clone(),
                    shadow.color,
                    true,
                  )
                },
                Some("to_inset_prefix"),
              )
            },
            Some("add_inset_prefix"),
          )
        },
        Some("inset_then_shadow"),
      );

      TokenParser::one_of(vec![shadow_then_inset, inset_then_shadow])
    };

    TokenParser::one_of(vec![inset_shadow, outer_shadow])
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for BoxShadow {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let inset_str = if self.inset { "inset " } else { "" };

    // Only include blur and spread if they're not zero
    let blur_str = if self.blur_radius.value != 0.0 {
      format!(" {}", self.blur_radius)
    } else {
      String::new()
    };

    let spread_str = if self.spread_radius.value != 0.0 {
      format!(" {}", self.spread_radius)
    } else {
      String::new()
    };

    write!(
      f,
      "{}{} {}{}{} {}",
      inset_str, self.offset_x, self.offset_y, blur_str, spread_str, self.color
    )
  }
}

/// List of box shadows (comma-separated)
#[derive(Debug, Clone, PartialEq)]
pub struct BoxShadowList {
  pub shadows: Vec<BoxShadow>,
}

impl BoxShadowList {
  /// Create a new BoxShadowList
  pub fn new(shadows: Vec<BoxShadow>) -> Self {
    Self { shadows }
  }

  /// Parser for box shadow list (comma-separated shadows)
  pub fn parser() -> TokenParser<BoxShadowList> {
    let comma = TokenParser::<SimpleToken>::token(SimpleToken::Comma, Some("Comma"));
    let whitespace = TokenParser::<SimpleToken>::token(SimpleToken::Whitespace, Some("Whitespace"));

    // Parse "none" keyword
    let none_parser =
      TokenParser::<SimpleToken>::token(SimpleToken::Ident("none".to_string()), Some("none_ident"))
        .where_fn(is_none_ident, Some("none_check"))
        .map(
          |_| BoxShadowList::new(Vec::new()),
          Some("empty_shadow_list"),
        );

    // Parse comma with optional surrounding whitespace
    let comma_separator =
      comma.surrounded_by(whitespace.clone().optional(), Some(whitespace.optional()));

    // Parse one or more shadows separated by commas
    let shadow_list_parser =
      TokenParser::one_or_more_separated_by(BoxShadow::parser(), comma_separator)
        .map(BoxShadowList::new, Some("shadow_list"));

    // Try "none" first, then shadow list
    TokenParser::one_of(vec![none_parser, shadow_list_parser])
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for BoxShadowList {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if self.shadows.is_empty() {
      write!(f, "none")
    } else {
      let shadow_strings: Vec<String> = self
        .shadows
        .iter()
        .map(|shadow| shadow.to_string())
        .collect();
      write!(f, "{}", shadow_strings.join(", "))
    }
  }
}

#[cfg(test)]
#[path = "../tests/properties/box_shadow_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/properties/box_shadow_test.rs"]
mod box_shadow_test;

#[cfg(test)]
#[path = "../tests/properties/box_shadow_coverage_test.rs"]
mod box_shadow_coverage_test;
