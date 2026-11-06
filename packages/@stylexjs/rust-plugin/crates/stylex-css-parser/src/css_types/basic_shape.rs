/**
 * CSS basic shape parsers for `clip-path` and `shape-outside`.
 */
use crate::token_parser::TokenParser;
use crate::{
  CssParseError,
  css_types::{
    length_percentage::{LengthPercentage, length_percentage_parser},
    position::Position,
  },
  token_types::{SimpleToken, TokenList},
};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum BasicShape {
  Inset {
    top: LengthPercentage,
    right: LengthPercentage,
    bottom: LengthPercentage,
    left: LengthPercentage,
    round: Option<LengthPercentage>,
  },
  Circle {
    radius: CircleRadius,
    position: Option<Position>,
  },
  Ellipse {
    radius_x: CircleRadius,
    radius_y: CircleRadius,
    position: Option<Position>,
  },
  Polygon {
    fill_rule: Option<String>,
    points: Vec<(LengthPercentage, LengthPercentage)>,
  },
  Path {
    fill_rule: Option<String>,
    path: String,
  },
}

#[derive(Debug, Clone, PartialEq)]
pub enum CircleRadius {
  Length(LengthPercentage),
  ClosestSide,
  FarthestSide,
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl fmt::Display for CircleRadius {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      CircleRadius::Length(lp) => write!(f, "{}", lp),
      CircleRadius::ClosestSide => write!(f, "closest-side"),
      CircleRadius::FarthestSide => write!(f, "farthest-side"),
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl fmt::Display for BasicShape {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      BasicShape::Inset {
        top,
        right,
        bottom,
        left,
        round,
      } => {
        let round_str = round
          .as_ref()
          .map(|r| format!(" round {}", r))
          .unwrap_or_default();

        if top == right
          && right == bottom
          && bottom == left
          && left == round.as_ref().unwrap_or(left)
        {
          write!(f, "inset({}{})", top, round_str)
        } else if top == bottom && left == right {
          write!(f, "inset({} {}{})", top, right, round_str)
        } else if top == bottom {
          write!(f, "inset({} {} {}{})", top, right, bottom, round_str)
        } else {
          write!(
            f,
            "inset({} {} {} {}{})",
            top, right, bottom, left, round_str
          )
        }
      },
      BasicShape::Circle { radius, position } => {
        let pos_str = position
          .as_ref()
          .map(|p| format!(" at {}", p))
          .unwrap_or_default();
        write!(f, "circle({}{})", radius, pos_str)
      },
      BasicShape::Ellipse {
        radius_x,
        radius_y,
        position,
      } => {
        let pos_str = position
          .as_ref()
          .map(|p| format!(" at {}", p))
          .unwrap_or_default();
        write!(f, "ellipse({} {}{})", radius_x, radius_y, pos_str)
      },
      BasicShape::Polygon { fill_rule, points } => {
        let fill_rule_str = fill_rule
          .as_ref()
          .map(|fr| format!("{}, ", fr))
          .unwrap_or_default();
        let points_str = points
          .iter()
          .map(|(x, y)| format!("{} {}", x, y))
          .collect::<Vec<_>>()
          .join(", ");
        write!(f, "polygon({}{})", fill_rule_str, points_str)
      },
      BasicShape::Path { fill_rule, path } => {
        let fill_rule_str = fill_rule
          .as_ref()
          .map(|fr| format!("{}, ", fr))
          .unwrap_or_default();
        write!(f, "path({}\"{}\")", fill_rule_str, path)
      },
    }
  }
}

impl BasicShape {
  fn inset_parser() -> TokenParser<BasicShape> {
    TokenParser::new(
      |tokens| {
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::Function(fn_name)) if fn_name == "inset" => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected inset() function, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected inset() function but reached end of input".to_string(),
            });
          },
        }

        let top = (length_percentage_parser().run)(tokens)?;

        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        let right = match (length_percentage_parser().run)(tokens) {
          Ok(right_val) => {
            while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
              let _ = tokens.consume_next_token();
            }
            right_val
          },
          _ => top.clone(),
        };

        let bottom = match (length_percentage_parser().run)(tokens) {
          Ok(bottom_val) => {
            while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
              let _ = tokens.consume_next_token();
            }
            bottom_val
          },
          _ => top.clone(),
        };

        let left = match (length_percentage_parser().run)(tokens) {
          Ok(left_val) => {
            while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
              let _ = tokens.consume_next_token();
            }
            left_val
          },
          _ => right.clone(),
        };

        // Optional `round`; preceding whitespace was consumed by the value loops.
        let round = if let Ok(Some(SimpleToken::Ident(keyword))) = tokens.peek() {
          if keyword == "round" {
            let _ = tokens.consume_next_token();

            while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
              let _ = tokens.consume_next_token();
            }

            let radius_value = (length_percentage_parser().run)(tokens)?;
            Some(radius_value)
          } else {
            None
          }
        } else {
          None
        };

        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::RightParen) => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected closing paren, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected closing paren but reached end of input".to_string(),
            });
          },
        }

        Ok(BasicShape::Inset {
          top,
          right,
          bottom,
          left,
          round,
        })
      },
      "inset_parser",
    )
  }

  fn circle_parser() -> TokenParser<BasicShape> {
    TokenParser::new(
      |tokens| {
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::Function(fn_name)) if fn_name == "circle" => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected circle() function, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected circle() function but reached end of input".to_string(),
            });
          },
        }

        // Parse radius: length-percentage | closest-side | farthest-side
        let radius = if let Some(token) = tokens.peek_infallible() {
          match token {
            SimpleToken::Ident(value) if value == "closest-side" => {
              let _ = tokens.consume_next_token(); // consume the ident
              CircleRadius::ClosestSide
            },
            SimpleToken::Ident(value) if value == "farthest-side" => {
              let _ = tokens.consume_next_token(); // consume the ident
              CircleRadius::FarthestSide
            },
            _ => {
              let lp = (length_percentage_parser().run)(tokens)?;
              CircleRadius::Length(lp)
            },
          }
        } else {
          return Err(CssParseError::ParseError {
            message: "Expected radius for circle".to_string(),
          });
        };

        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        let position = Self::parse_optional_position(tokens)?;

        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::RightParen) => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected closing paren, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected closing paren but reached end of input".to_string(),
            });
          },
        }

        Ok(BasicShape::Circle { radius, position })
      },
      "circle_parser",
    )
  }

  fn ellipse_parser() -> TokenParser<BasicShape> {
    TokenParser::new(
      |tokens| {
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::Function(fn_name)) if fn_name == "ellipse" => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected ellipse() function, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected ellipse() function but reached end of input".to_string(),
            });
          },
        }

        let parse_radius = |tokens: &mut TokenList| -> Result<CircleRadius, CssParseError> {
          if let Some(token) = tokens.peek_infallible() {
            match token {
              SimpleToken::Ident(value) if value == "closest-side" => {
                let _ = tokens.consume_next_token();
                Ok(CircleRadius::ClosestSide)
              },
              SimpleToken::Ident(value) if value == "farthest-side" => {
                let _ = tokens.consume_next_token();
                Ok(CircleRadius::FarthestSide)
              },
              _ => {
                let lp = (length_percentage_parser().run)(tokens)?;
                Ok(CircleRadius::Length(lp))
              },
            }
          } else {
            Err(CssParseError::ParseError {
              message: "Expected radius".to_string(),
            })
          }
        };

        let radius_x = parse_radius(tokens)?;

        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        let radius_y = parse_radius(tokens)?;

        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        let position = Self::parse_optional_position(tokens)?;

        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::RightParen) => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected closing paren, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected closing paren but reached end of input".to_string(),
            });
          },
        }

        Ok(BasicShape::Ellipse {
          radius_x,
          radius_y,
          position,
        })
      },
      "ellipse_parser",
    )
  }

  fn polygon_parser() -> TokenParser<BasicShape> {
    TokenParser::new(
      |tokens| {
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::Function(fn_name)) if fn_name == "polygon" => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected polygon() function, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected polygon() function but reached end of input".to_string(),
            });
          },
        }

        // Parse optional fillRule (nonzero | evenodd)
        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        let fill_rule = if let Ok(Some(SimpleToken::Ident(rule))) = tokens.peek() {
          if rule == "nonzero" || rule == "evenodd" {
            // Consume the fill-rule ident matched above.
            let _ = tokens.consume_next_token();
            // Skip optional comma and whitespace
            while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
              let _ = tokens.consume_next_token();
            }
            if let Ok(Some(SimpleToken::Comma)) = tokens.peek() {
              let _ = tokens.consume_next_token();
              while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
                let _ = tokens.consume_next_token();
              }
            }
            Some(rule)
          } else {
            Some("nonzero".to_string()) // default
          }
        } else {
          Some("nonzero".to_string()) // default
        };

        // At least one point (x y, x y, ...)
        let mut points = Vec::new();

        loop {
          while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
            let _ = tokens.consume_next_token();
          }

          if let Ok(Some(SimpleToken::RightParen)) = tokens.peek() {
            break;
          }

          let x = (length_percentage_parser().run)(tokens)?;

          while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
            let _ = tokens.consume_next_token();
          }

          let y = (length_percentage_parser().run)(tokens)?;

          points.push((x, y));

          while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
            let _ = tokens.consume_next_token();
          }

          // Check for comma (more points) or closing paren (end)
          if let Ok(Some(token)) = tokens.peek() {
            match token {
              SimpleToken::Comma => {
                let _ = tokens.consume_next_token(); // consume the comma
                continue; // parse next point
              },
              SimpleToken::RightParen => {
                break; // done parsing points
              },
              _ => {
                return Err(CssParseError::ParseError {
                  message: format!(
                    "Expected comma or closing paren in polygon, got {:?}",
                    token
                  ),
                });
              },
            }
          } else {
            return Err(CssParseError::ParseError {
              message: "Unexpected end of input in polygon".to_string(),
            });
          }
        }

        // The loop only breaks on a peeked RightParen; consume it.
        let _ = tokens.consume_next_token();

        if points.is_empty() {
          return Err(CssParseError::ParseError {
            message: "Polygon must have at least one point".to_string(),
          });
        }

        Ok(BasicShape::Polygon { fill_rule, points })
      },
      "polygon_parser",
    )
  }

  fn path_parser() -> TokenParser<BasicShape> {
    TokenParser::new(
      |tokens| {
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::Function(fn_name)) if fn_name == "path" => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected path() function, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected path() function but reached end of input".to_string(),
            });
          },
        }

        // Parse optional fillRule (nonzero | evenodd)
        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        let fill_rule = if let Ok(Some(SimpleToken::Ident(rule))) = tokens.peek() {
          if rule == "nonzero" || rule == "evenodd" {
            // Consume the fill-rule ident matched above.
            let _ = tokens.consume_next_token();
            // Skip optional comma and whitespace
            while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
              let _ = tokens.consume_next_token();
            }
            if let Ok(Some(SimpleToken::Comma)) = tokens.peek() {
              let _ = tokens.consume_next_token();
              while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
                let _ = tokens.consume_next_token();
              }
            }
            Some(rule)
          } else {
            Some("nonzero".to_string()) // default
          }
        } else {
          Some("nonzero".to_string()) // default
        };

        // Whitespace before the path string is already consumed.
        let path = match tokens.consume_next_token_infallible() {
          Some(SimpleToken::String(s)) => s,
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected string literal for path, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected path string but reached end of input".to_string(),
            });
          },
        };

        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::RightParen) => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected closing paren, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected closing paren but reached end of input".to_string(),
            });
          },
        }

        Ok(BasicShape::Path { fill_rule, path })
      },
      "path_parser",
    )
  }

  /// Parse optional "at Position" for circle and ellipse
  fn parse_optional_position(tokens: &mut TokenList) -> Result<Option<Position>, CssParseError> {
    let checkpoint = tokens.current_index;

    // Callers already drain whitespace; tests exercise this guard directly.
    if let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
      let _ = tokens.consume_next_token();
    }

    match tokens.peek_infallible() {
      Some(SimpleToken::Ident(keyword)) if keyword == "at" => {
        let _ = tokens.consume_next_token(); // consume "at"

        if let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        let position_parser = Position::parser();
        match position_parser.run.as_ref()(tokens) {
          Ok(position) => Ok(Some(position)),
          Err(e) => Err(e),
        }
      },
      _ => {
        // No `at`: rewind
        tokens.set_current_index(checkpoint);
        Ok(None)
      },
    }
  }

  pub fn parse() -> TokenParser<BasicShape> {
    TokenParser::one_of(vec![
      Self::inset_parser(),
      Self::circle_parser(),
      Self::ellipse_parser(),
      Self::polygon_parser(),
      Self::path_parser(),
    ])
  }
}

pub fn basic_shape_parser() -> TokenParser<BasicShape> {
  BasicShape::parse()
}

#[cfg(test)]
#[path = "../tests/css_types/basic_shape_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/css_types/basic_shape_test.rs"]
mod basic_shape_test;

#[cfg(test)]
#[path = "../tests/css_types/basic_shape_coverage_test.rs"]
mod basic_shape_coverage_test;
