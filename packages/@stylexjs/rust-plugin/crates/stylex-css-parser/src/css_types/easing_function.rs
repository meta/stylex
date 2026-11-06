/*!
CSS easing function parser.
*/

use stylex_macros::stylex_unreachable;
use stylex_utils::number::{to_js_string, write_js_number_list};

use crate::{
  CssParseError,
  token_parser::TokenParser,
  token_types::{SimpleToken, TokenList},
};
use std::fmt::{self, Display, Write as _};

#[derive(Debug, Clone, PartialEq)]
pub enum EasingFunction {
  Linear(LinearEasingFunction),
  CubicBezier(CubicBezierEasingFunction),
  CubicBezierKeyword(CubicBezierKeyword),
  Steps(StepsEasingFunction),
  StepsKeyword(StepsKeyword),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinearEasingFunction {
  pub points: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CubicBezierEasingFunction {
  pub points: [f64; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub struct CubicBezierKeyword {
  pub keyword: CubicBezierKeywordType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StepsEasingFunction {
  pub steps: u32,
  pub start: StepsStartType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StepsKeyword {
  pub keyword: StepsKeywordType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CubicBezierKeywordType {
  Ease,
  EaseIn,
  EaseOut,
  EaseInOut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepsStartType {
  Start,
  End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepsKeywordType {
  StepStart,
  StepEnd,
}

impl EasingFunction {
  pub fn parse() -> TokenParser<EasingFunction> {
    TokenParser::one_of(vec![
      LinearEasingFunction::parse().map(EasingFunction::Linear, Some("linear")),
      CubicBezierEasingFunction::parse().map(EasingFunction::CubicBezier, Some("cubic_bezier")),
      StepsEasingFunction::parse().map(EasingFunction::Steps, Some("steps")),
      CubicBezierKeyword::parse().map(
        EasingFunction::CubicBezierKeyword,
        Some("cubic_bezier_keyword"),
      ),
      StepsKeyword::parse().map(EasingFunction::StepsKeyword, Some("steps_keyword")),
    ])
  }
}

impl LinearEasingFunction {
  pub fn new(points: Vec<f64>) -> Self {
    Self { points }
  }

  pub(crate) fn parse_tokens(
    tokens: &mut TokenList,
  ) -> Result<LinearEasingFunction, CssParseError> {
    // Parse 'linear(' function start
    match tokens.consume_next_token_infallible() {
      Some(SimpleToken::Function(fn_name)) if fn_name == "linear" => {},
      Some(token) => {
        return Err(CssParseError::ParseError {
          message: format!("Expected linear() function, got {:?}", token),
        });
      },
      None => {
        return Err(CssParseError::ParseError {
          message: "Expected linear() function but reached end of input".to_string(),
        });
      },
    }

    // Skip optional whitespace
    while matches!(tokens.peek(), Ok(Some(SimpleToken::Whitespace))) {
      let _ = tokens.consume_next_token();
    }

    // Parse one or more numbers separated by commas
    let mut points = Vec::new();

    // Check for empty linear() — at least one point is required.
    // Peek ahead: if the next token is already a closing paren, no numbers were
    // provided and points would remain empty after the loop.
    if matches!(tokens.peek(), Ok(Some(SimpleToken::RightParen))) {
      return Err(CssParseError::ParseError {
        message: "Linear function must have at least one point".to_string(),
      });
    }

    loop {
      // Parse number
      let number = match tokens.consume_next_token_infallible() {
        Some(SimpleToken::Number(value)) => value,
        Some(token) => {
          return Err(CssParseError::ParseError {
            message: format!("Expected number in linear function, got {:?}", token),
          });
        },
        None => {
          return Err(CssParseError::ParseError {
            message: "Expected number but reached end of input".to_string(),
          });
        },
      };

      points.push(number);

      // Skip optional whitespace
      while matches!(tokens.peek(), Ok(Some(SimpleToken::Whitespace))) {
        let _ = tokens.consume_next_token();
      }

      // Consume comma (more numbers) or closing paren (end)
      match tokens.consume_next_token_infallible() {
        Some(SimpleToken::Comma) => {
          // Skip optional whitespace after comma
          while matches!(tokens.peek(), Ok(Some(SimpleToken::Whitespace))) {
            let _ = tokens.consume_next_token();
          }
          continue; // parse next number
        },
        Some(SimpleToken::RightParen) => {
          break; // done parsing numbers (RightParen consumed)
        },
        Some(token) => {
          return Err(CssParseError::ParseError {
            message: format!(
              "Expected comma or closing paren in linear function, got {:?}",
              token
            ),
          });
        },
        None => {
          return Err(CssParseError::ParseError {
            message: "Unexpected end of input in linear function".to_string(),
          });
        },
      }
    }

    Ok(LinearEasingFunction::new(points))
  }

  pub fn parse() -> TokenParser<LinearEasingFunction> {
    TokenParser::new(Self::parse_tokens, "linear_parser")
  }
}

impl CubicBezierEasingFunction {
  pub fn new(points: [f64; 4]) -> Self {
    Self { points }
  }

  pub(crate) fn parse_tokens(
    tokens: &mut TokenList,
  ) -> Result<CubicBezierEasingFunction, CssParseError> {
    // Parse 'cubic-bezier(' function start
    match tokens.consume_next_token_infallible() {
      Some(SimpleToken::Function(fn_name)) if fn_name == "cubic-bezier" => {},
      Some(token) => {
        return Err(CssParseError::ParseError {
          message: format!("Expected cubic-bezier() function, got {:?}", token),
        });
      },
      None => {
        return Err(CssParseError::ParseError {
          message: "Expected cubic-bezier() function but reached end of input".to_string(),
        });
      },
    }

    // Skip optional whitespace
    while matches!(tokens.peek(), Ok(Some(SimpleToken::Whitespace))) {
      let _ = tokens.consume_next_token();
    }

    // Parse exactly 4 numbers separated by commas: x1, y1, x2, y2
    let mut numbers = Vec::new();

    for i in 0..4 {
      if i > 0 {
        // Skip optional whitespace before comma
        while matches!(tokens.peek(), Ok(Some(SimpleToken::Whitespace))) {
          let _ = tokens.consume_next_token();
        }

        // Expect comma
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::Comma) => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected comma in cubic-bezier function, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected comma but reached end of input".to_string(),
            });
          },
        }

        // Skip optional whitespace after comma
        while matches!(tokens.peek(), Ok(Some(SimpleToken::Whitespace))) {
          let _ = tokens.consume_next_token();
        }
      }

      // Parse number
      let number = match tokens.consume_next_token_infallible() {
        Some(SimpleToken::Number(value)) => value,
        Some(token) => {
          return Err(CssParseError::ParseError {
            message: format!("Expected number in cubic-bezier function, got {:?}", token),
          });
        },
        None => {
          return Err(CssParseError::ParseError {
            message: "Expected number but reached end of input".to_string(),
          });
        },
      };

      numbers.push(number);
    }

    // Skip optional whitespace
    while matches!(tokens.peek(), Ok(Some(SimpleToken::Whitespace))) {
      let _ = tokens.consume_next_token();
    }

    // Parse closing paren
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

    Ok(CubicBezierEasingFunction::new([
      numbers[0], numbers[1], numbers[2], numbers[3],
    ]))
  }

  pub fn parse() -> TokenParser<CubicBezierEasingFunction> {
    TokenParser::new(Self::parse_tokens, "cubic_bezier_parser")
  }
}

impl CubicBezierKeyword {
  pub fn new(keyword: CubicBezierKeywordType) -> Self {
    Self { keyword }
  }

  /// Create from string representation
  pub fn from_string(value: &str) -> Result<Self, CssParseError> {
    let keyword = match value {
      "ease" => CubicBezierKeywordType::Ease,
      "ease-in" => CubicBezierKeywordType::EaseIn,
      "ease-out" => CubicBezierKeywordType::EaseOut,
      "ease-in-out" => CubicBezierKeywordType::EaseInOut,
      _ => {
        return Err(CssParseError::ParseError {
          message: format!("Unknown cubic-bezier keyword: {}", value),
        });
      },
    };
    Ok(Self::new(keyword))
  }

  pub(crate) fn is_easing_keyword(token: &SimpleToken) -> bool {
    if let SimpleToken::Ident(value) = token {
      matches!(
        value.as_str(),
        "ease" | "ease-in" | "ease-out" | "ease-in-out"
      )
    } else {
      false
    }
  }

  pub(crate) fn extract_keyword_token(token: SimpleToken) -> CubicBezierKeyword {
    if let SimpleToken::Ident(value) = token {
      let keyword = match value.as_str() {
        "ease" => CubicBezierKeywordType::Ease,
        "ease-in" => CubicBezierKeywordType::EaseIn,
        "ease-out" => CubicBezierKeywordType::EaseOut,
        "ease-in-out" => CubicBezierKeywordType::EaseInOut,
        _ => stylex_unreachable!(),
      };
      CubicBezierKeyword::new(keyword)
    } else {
      stylex_unreachable!()
    }
  }

  pub fn parse() -> TokenParser<CubicBezierKeyword> {
    TokenParser::<SimpleToken>::ident()
      .where_fn(Self::is_easing_keyword, Some("easing_keyword"))
      .map(Self::extract_keyword_token, Some("to_keyword"))
  }
}

impl StepsEasingFunction {
  pub fn new(steps: u32, start: StepsStartType) -> Self {
    Self { steps, start }
  }

  pub(crate) fn parse_tokens(tokens: &mut TokenList) -> Result<StepsEasingFunction, CssParseError> {
    // Parse 'steps(' function start
    match tokens.consume_next_token_infallible() {
      Some(SimpleToken::Function(fn_name)) if fn_name == "steps" => {},
      Some(token) => {
        return Err(CssParseError::ParseError {
          message: format!("Expected steps() function, got {:?}", token),
        });
      },
      None => {
        return Err(CssParseError::ParseError {
          message: "Expected steps() function but reached end of input".to_string(),
        });
      },
    }

    // Skip optional whitespace
    while matches!(tokens.peek(), Ok(Some(SimpleToken::Whitespace))) {
      let _ = tokens.consume_next_token();
    }

    // Parse step count (integer number)
    let steps = match tokens.consume_next_token_infallible() {
      Some(SimpleToken::Number(value)) => {
        let int_value = value as u32;
        if int_value as f64 == value && value >= 0.0 {
          int_value
        } else {
          return Err(CssParseError::ParseError {
            message: "Steps count must be a positive integer".to_string(),
          });
        }
      },
      Some(token) => {
        return Err(CssParseError::ParseError {
          message: format!("Expected number for steps count, got {:?}", token),
        });
      },
      None => {
        return Err(CssParseError::ParseError {
          message: "Expected steps count but reached end of input".to_string(),
        });
      },
    };

    // Skip optional whitespace
    while matches!(tokens.peek(), Ok(Some(SimpleToken::Whitespace))) {
      let _ = tokens.consume_next_token();
    }

    // Expect comma
    match tokens.consume_next_token_infallible() {
      Some(SimpleToken::Comma) => {},
      Some(token) => {
        return Err(CssParseError::ParseError {
          message: format!("Expected comma in steps function, got {:?}", token),
        });
      },
      None => {
        return Err(CssParseError::ParseError {
          message: "Expected comma but reached end of input".to_string(),
        });
      },
    }

    // Skip optional whitespace after comma
    while matches!(tokens.peek(), Ok(Some(SimpleToken::Whitespace))) {
      let _ = tokens.consume_next_token();
    }

    // Parse start/end type
    let start_type = match tokens.consume_next_token_infallible() {
      Some(SimpleToken::Ident(value)) => match value.as_str() {
        "start" => StepsStartType::Start,
        "end" => StepsStartType::End,
        _ => {
          return Err(CssParseError::ParseError {
            message: format!(
              "Expected 'start' or 'end' in steps function, got '{}'",
              value
            ),
          });
        },
      },
      Some(token) => {
        return Err(CssParseError::ParseError {
          message: format!(
            "Expected 'start' or 'end' in steps function, got {:?}",
            token
          ),
        });
      },
      None => {
        return Err(CssParseError::ParseError {
          message: "Expected 'start' or 'end' but reached end of input".to_string(),
        });
      },
    };

    // Skip optional whitespace
    while matches!(tokens.peek(), Ok(Some(SimpleToken::Whitespace))) {
      let _ = tokens.consume_next_token();
    }

    // Parse closing paren
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

    Ok(StepsEasingFunction::new(steps, start_type))
  }

  pub fn parse() -> TokenParser<StepsEasingFunction> {
    TokenParser::new(Self::parse_tokens, "steps_parser")
  }
}

impl StepsKeyword {
  pub fn new(keyword: StepsKeywordType) -> Self {
    Self { keyword }
  }

  /// Create from string representation
  pub fn from_string(value: &str) -> Result<Self, CssParseError> {
    let keyword = match value {
      "step-start" => StepsKeywordType::StepStart,
      "step-end" => StepsKeywordType::StepEnd,
      _ => {
        return Err(CssParseError::ParseError {
          message: format!("Unknown steps keyword: {}", value),
        });
      },
    };
    Ok(Self::new(keyword))
  }

  pub(crate) fn is_steps_keyword(token: &SimpleToken) -> bool {
    if let SimpleToken::Ident(value) = token {
      matches!(value.as_str(), "step-start" | "step-end")
    } else {
      false
    }
  }

  pub(crate) fn extract_steps_keyword_token(token: SimpleToken) -> StepsKeyword {
    if let SimpleToken::Ident(value) = token {
      let keyword = match value.as_str() {
        "step-start" => StepsKeywordType::StepStart,
        "step-end" => StepsKeywordType::StepEnd,
        _ => stylex_unreachable!(),
      };
      StepsKeyword::new(keyword)
    } else {
      stylex_unreachable!()
    }
  }

  pub fn parse() -> TokenParser<StepsKeyword> {
    TokenParser::<SimpleToken>::ident()
      .where_fn(Self::is_steps_keyword, Some("steps_keyword"))
      .map(Self::extract_steps_keyword_token, Some("to_steps_keyword"))
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for EasingFunction {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      EasingFunction::Linear(linear) => linear.fmt(f),
      EasingFunction::CubicBezier(cubic_bezier) => cubic_bezier.fmt(f),
      EasingFunction::CubicBezierKeyword(keyword) => keyword.fmt(f),
      EasingFunction::Steps(steps) => steps.fmt(f),
      EasingFunction::StepsKeyword(keyword) => keyword.fmt(f),
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for LinearEasingFunction {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("linear(")?;
    write_js_number_list(f, self.points.iter().copied())?;
    f.write_char(')')
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for CubicBezierEasingFunction {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "cubic-bezier({}, {}, {}, {})",
      to_js_string(self.points[0]),
      to_js_string(self.points[1]),
      to_js_string(self.points[2]),
      to_js_string(self.points[3])
    )
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for CubicBezierKeyword {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let keyword_str = match self.keyword {
      CubicBezierKeywordType::Ease => "ease",
      CubicBezierKeywordType::EaseIn => "ease-in",
      CubicBezierKeywordType::EaseOut => "ease-out",
      CubicBezierKeywordType::EaseInOut => "ease-in-out",
    };
    write!(f, "{}", keyword_str)
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for StepsEasingFunction {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let start_str = match self.start {
      StepsStartType::Start => "start",
      StepsStartType::End => "end",
    };
    write!(f, "steps({}, {})", self.steps, start_str)
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for StepsKeyword {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let keyword_str = match self.keyword {
      StepsKeywordType::StepStart => "step-start",
      StepsKeywordType::StepEnd => "step-end",
    };
    write!(f, "{}", keyword_str)
  }
}

#[cfg(test)]
#[path = "../tests/css_types/easing_function_test.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/css_types/easing_function_coverage_test.rs"]
mod easing_function_coverage_test;

#[cfg(test)]
#[path = "../tests/css_types/easing_function_precision_test.rs"]
mod easing_function_precision_test;
