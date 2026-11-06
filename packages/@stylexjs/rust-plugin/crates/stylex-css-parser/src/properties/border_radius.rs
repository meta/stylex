/*!
CSS Border Radius property parsing.

Handles border-radius property syntax including individual values and shorthand notation.
Supports both horizontal and vertical radius values with proper fallback logic.
*/

use stylex_macros::stylex_unreachable;

use crate::{
  css_types::{LengthPercentage, length_percentage_parser},
  token_parser::TokenParser,
  token_types::SimpleToken,
};
use std::fmt::{self, Display};

/// Individual border radius value (can have different horizontal and vertical
/// values)
#[derive(Debug, Clone, PartialEq)]
pub struct BorderRadiusIndividual {
  pub horizontal: LengthPercentage,
  pub vertical: LengthPercentage,
}

impl BorderRadiusIndividual {
  /// Create a new BorderRadiusIndividual
  pub fn new(horizontal: LengthPercentage, vertical: Option<LengthPercentage>) -> Self {
    Self {
      horizontal: horizontal.clone(),
      vertical: vertical.unwrap_or(horizontal),
    }
  }

  /// Parser for individual border radius values
  pub fn parser() -> TokenParser<BorderRadiusIndividual> {
    let whitespace = TokenParser::<SimpleToken>::token(SimpleToken::Whitespace, Some("Whitespace"));

    // Use the WORKING pattern from BorderRadiusShorthand
    let first_value = length_percentage_parser();
    let second_value_optional = whitespace
      .flat_map(|_| length_percentage_parser(), Some("second_value"))
      .optional();

    first_value.flat_map(
      move |first| {
        second_value_optional.clone().map(
          move |second_opt| match second_opt {
            Some(second) => BorderRadiusIndividual::new(first.clone(), Some(second)),
            None => BorderRadiusIndividual::new(first.clone(), None),
          },
          Some("individual_radius"),
        )
      },
      Some("individual_parser"),
    )
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for BorderRadiusIndividual {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let horizontal = self.horizontal.to_string();
    let vertical = self.vertical.to_string();

    if horizontal == vertical {
      write!(f, "{}", horizontal)
    } else {
      write!(f, "{} {}", horizontal, vertical)
    }
  }
}

/// Border radius shorthand property (all four corners)
#[derive(Debug, Clone, PartialEq)]
pub struct BorderRadiusShorthand {
  // Horizontal radii
  pub horizontal_top_left: LengthPercentage,
  pub horizontal_top_right: LengthPercentage,
  pub horizontal_bottom_right: LengthPercentage,
  pub horizontal_bottom_left: LengthPercentage,

  // Vertical radii
  pub vertical_top_left: LengthPercentage,
  pub vertical_top_right: LengthPercentage,
  pub vertical_bottom_right: LengthPercentage,
  pub vertical_bottom_left: LengthPercentage,
}

impl BorderRadiusShorthand {
  /// Apply CSS shorthand expansion to a list of 1-4 (clamped) radii values.
  /// Panics if `values` is empty (unreachable through the public parser).
  fn expand_radii(values: Vec<LengthPercentage>) -> [LengthPercentage; 4] {
    match values.len() {
      1 => [
        values[0].clone(),
        values[0].clone(),
        values[0].clone(),
        values[0].clone(),
      ],
      2 => [
        values[0].clone(),
        values[1].clone(),
        values[0].clone(),
        values[1].clone(),
      ],
      3 => [
        values[0].clone(),
        values[1].clone(),
        values[2].clone(),
        values[1].clone(),
      ],
      4 => [
        values[0].clone(),
        values[1].clone(),
        values[2].clone(),
        values[3].clone(),
      ],
      _ => stylex_unreachable!("border-radius expansion only accepts one to four radii"),
    }
  }

  /// Create a new BorderRadiusShorthand with CSS shorthand expansion logic
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    horizontal_top_left: LengthPercentage,
    horizontal_top_right: Option<LengthPercentage>,
    horizontal_bottom_right: Option<LengthPercentage>,
    horizontal_bottom_left: Option<LengthPercentage>,
    vertical_top_left: Option<LengthPercentage>,
    vertical_top_right: Option<LengthPercentage>,
    vertical_bottom_right: Option<LengthPercentage>,
    vertical_bottom_left: Option<LengthPercentage>,
  ) -> Self {
    // CSS shorthand expansion logic
    let h_top_right = horizontal_top_right.unwrap_or(horizontal_top_left.clone());
    let h_bottom_right = horizontal_bottom_right.unwrap_or(horizontal_top_left.clone());
    let h_bottom_left = horizontal_bottom_left.unwrap_or(h_top_right.clone());

    let v_top_left = vertical_top_left.unwrap_or(horizontal_top_left.clone());
    let v_top_right = vertical_top_right.unwrap_or(v_top_left.clone());
    let v_bottom_right = vertical_bottom_right.unwrap_or(v_top_left.clone());
    let v_bottom_left = vertical_bottom_left.unwrap_or(v_top_right.clone());

    Self {
      horizontal_top_left,
      horizontal_top_right: h_top_right,
      horizontal_bottom_right: h_bottom_right,
      horizontal_bottom_left: h_bottom_left,
      vertical_top_left: v_top_left,
      vertical_top_right: v_top_right,
      vertical_bottom_right: v_bottom_right,
      vertical_bottom_left: v_bottom_left,
    }
  }

  pub fn parser() -> TokenParser<BorderRadiusShorthand> {
    // Syntax: horizontal-radii [ / vertical-radii ]?

    let whitespace = TokenParser::<SimpleToken>::token(SimpleToken::Whitespace, Some("Whitespace"));
    let slash = TokenParser::<SimpleToken>::token(SimpleToken::Delim('/'), Some("Slash"));

    // Helper to parse 1-4 space-separated length/percentage values
    let space_separated_radii = {
      let first_value = length_percentage_parser();
      let remaining_values = TokenParser::<LengthPercentage>::zero_or_more(
        whitespace.flat_map(|_| length_percentage_parser(), Some("next_value")),
      );

      first_value.flat_map(
        move |first| {
          let first_clone = first;
          remaining_values.clone().map(
            move |rest| {
              let mut all_values = vec![first_clone.clone()];
              all_values.extend(rest);

              // Limit to 4 values and apply CSS shorthand expansion.
              let values = if all_values.len() > 4 {
                all_values[..4].to_vec()
              } else {
                all_values
              };

              Self::expand_radii(values)
            },
            Some("expand_radii"),
          )
        },
        Some("space_separated"),
      )
    };

    // Parse optional " / vertical-radii" part
    let slash_vertical = {
      let whitespace_before_slash = whitespace.clone().optional();
      let whitespace_after_slash = whitespace.optional();
      let slash_clone = slash;
      let radii_clone = space_separated_radii.clone();

      whitespace_before_slash
        .flat_map(move |_| slash_clone.clone(), Some("slash"))
        .flat_map(
          move |_| whitespace_after_slash.clone(),
          Some("ws_after_slash"),
        )
        .flat_map(move |_| radii_clone.clone(), Some("vertical_radii"))
        .optional()
    };

    // Main parser: horizontal-radii [/ vertical-radii]?
    space_separated_radii.flat_map(
      move |horizontal_radii| {
        let h_radii = horizontal_radii;
        slash_vertical.clone().map(
          move |vertical_opt| {
            let [h_tl, h_tr, h_br, h_bl] = h_radii.clone();

            match vertical_opt {
              Some(vertical_radii) => {
                let [v_tl, v_tr, v_br, v_bl] = vertical_radii;
                BorderRadiusShorthand::new(
                  h_tl,
                  Some(h_tr),
                  Some(h_br),
                  Some(h_bl),
                  Some(v_tl),
                  Some(v_tr),
                  Some(v_br),
                  Some(v_bl),
                )
              },
              None => {
                // Only horizontal radii provided, vertical defaults to horizontal
                BorderRadiusShorthand::new(
                  h_tl,
                  Some(h_tr),
                  Some(h_br),
                  Some(h_bl),
                  None,
                  None,
                  None,
                  None,
                )
              },
            }
          },
          Some("with_vertical"),
        )
      },
      Some("main_parser"),
    )
  }

  /// Get the shortest possible string representation
  fn to_shortest_string(&self) -> String {
    let h_top_left = self.horizontal_top_left.to_string();
    let h_top_right = self.horizontal_top_right.to_string();
    let h_bottom_right = self.horizontal_bottom_right.to_string();
    let h_bottom_left = self.horizontal_bottom_left.to_string();

    // Determine shortest horizontal representation
    let horizontal_str = if h_top_left == h_top_right
      && h_top_right == h_bottom_right
      && h_bottom_right == h_bottom_left
    {
      // All four are the same
      h_top_left
    } else if h_top_left == h_bottom_right && h_top_right == h_bottom_left {
      // TopLeft === BottomRight && TopRight === BottomLeft
      format!("{} {}", h_top_left, h_top_right)
    } else if h_top_right == h_bottom_left {
      // TopRight === BottomLeft
      format!("{} {} {}", h_top_left, h_top_right, h_bottom_right)
    } else {
      // All four values needed
      format!(
        "{} {} {} {}",
        h_top_left, h_top_right, h_bottom_right, h_bottom_left
      )
    };

    let v_top_left = self.vertical_top_left.to_string();
    let v_top_right = self.vertical_top_right.to_string();
    let v_bottom_right = self.vertical_bottom_right.to_string();
    let v_bottom_left = self.vertical_bottom_left.to_string();

    // Determine shortest vertical representation
    let vertical_str = if v_top_left == v_top_right
      && v_top_right == v_bottom_right
      && v_bottom_right == v_bottom_left
    {
      // All four are the same
      v_top_left
    } else if v_top_left == v_bottom_right && v_top_right == v_bottom_left {
      // TopLeft === BottomRight && TopRight === BottomLeft
      format!("{} {}", v_top_left, v_top_right)
    } else if v_top_right == v_bottom_left {
      // TopRight === BottomLeft
      format!("{} {} {}", v_top_left, v_top_right, v_bottom_right)
    } else {
      // All four values needed
      format!(
        "{} {} {} {}",
        v_top_left, v_top_right, v_bottom_right, v_bottom_left
      )
    };

    // If horizontal and vertical are the same, just return horizontal
    if horizontal_str == vertical_str {
      horizontal_str
    } else {
      format!("{} / {}", horizontal_str, vertical_str)
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for BorderRadiusShorthand {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.to_shortest_string())
  }
}

#[cfg(test)]
#[path = "../tests/properties/border_radius_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/properties/border_radius_test.rs"]
mod border_radius_test;

#[cfg(test)]
#[path = "../tests/properties/border_radius_coverage_test.rs"]
mod border_radius_coverage_test;
